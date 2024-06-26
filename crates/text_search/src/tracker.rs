use std::{
    collections::BTreeMap,
    fs::File,
    io::{
        BufReader,
        BufWriter,
        Read,
        Write,
    },
    iter::zip,
    ops::AddAssign,
    path::Path,
};

use anyhow::Context;
use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};
use common::id_tracker::{
    MemoryIdTracker,
    StaticIdTracker,
};
use sucds::{
    int_vectors::{
        Access,
        Build,
        DacsOpt,
    },
    mii_sequences::{
        EliasFano,
        EliasFanoBuilder,
    },
    Serializable,
};
use tantivy::{
    fastfield::AliveBitSet,
    schema::Field,
    termdict::{
        TermDictionary,
        TermOrdinal,
    },
    DocId,
};
use tantivy_common::{
    BitSet,
    OwnedBytes,
};
use value::InternalId;

use crate::metrics::{
    load_alive_bitset_timer,
    load_deleted_terms_table_timer,
    log_alive_bitset_size,
    log_deleted_terms_table_size,
};

/// Version 1 of the deletion tracker has the following format:
/// ```
/// [version] [field_header_size] [[field_id] [num_terms_deleted] [deleted_term_ordinals_size] [counts_size]]* [[deleted_term_ordinals] [counts]]*
/// ```
/// - version (u8): version number for the file format
/// - field_header_size (little-endian u16): length of the header describing the
///   deleted terms table for each field
/// - field_id (little-endian u32): field id
/// - num_terms_deleted (little-endian u64): number of non-unique terms that
///   were deleted from the segment
/// - deleted_term_ordinals_size (little-endian u32): size of the term ordinals
///   EliasFano
/// - counts_size (little-endian u32): size of the DacsOpt encoded counts of
///   deleted documents per term
/// - deleted_term_ordinals: EliasFano-encoded list of term ordinals from
///   deleted documents
/// - counts (DacsOpt): number of deleted documents per term, indexed in the
///   same order as `deleted_term_ordinals`
pub const DELETED_TERMS_TABLE_VERSION: u8 = 1;
pub const SIZE_PER_FIELD_HEADER: u16 = 20;

pub struct StaticDeletionTracker {
    alive_bitset: AliveBitSet,
    deleted_terms_by_field: BTreeMap<Field, DeletedTermsTable>,
}

/// Efficient read-only data structure for storing term deletions. This can be
/// converted into a mutable [FieldTermMetadata] to process updates.
struct DeletedTermsTable {
    /// Set of term ordinals of terms that are in documents that have been
    /// deleted.
    term_ordinals: EliasFano,
    /// Number of documents deleted for each term, corresponding to the order in
    /// term_ordinals.
    term_documents_deleted: DacsOpt,
    /// Number of non-unique terms that were deleted from the field in the
    /// segment
    num_terms_deleted: u64,
}

struct FieldHeader {
    field: Field,
    num_terms_deleted: u64,
    deleted_term_ordinals_size: u32,
    counts_size: u32,
}

impl DeletedTermsTable {
    /// Returns the number of documents deleted containing a term.
    fn term_documents_deleted(&self, term_ord: TermOrdinal) -> anyhow::Result<u32> {
        if let Some(pos) = self.term_ordinals.binsearch(term_ord as usize) {
            self.term_documents_deleted
                .access(pos)
                .map(|x| x as u32)
                .with_context(|| {
                    format!(
                        "No documents deleted count found for term {term_ord} in position {pos}"
                    )
                })
        } else {
            Ok(0)
        }
    }
}

impl From<DeletedTermsTable> for FieldTermMetadata {
    fn from(
        DeletedTermsTable {
            num_terms_deleted,
            term_ordinals,
            term_documents_deleted,
        }: DeletedTermsTable,
    ) -> Self {
        let term_documents_deleted = zip(term_ordinals.iter(0), term_documents_deleted.iter())
            .map(|(term_ord, num_deleted)| (term_ord as u64, num_deleted as u32))
            .collect();
        FieldTermMetadata {
            term_documents_deleted,
            num_terms_deleted,
        }
    }
}

impl TryFrom<FieldTermMetadata> for Option<DeletedTermsTable> {
    type Error = anyhow::Error;

    fn try_from(
        FieldTermMetadata {
            term_documents_deleted,
            num_terms_deleted,
        }: FieldTermMetadata,
    ) -> Result<Self, Self::Error> {
        let (term_ordinals, counts): (Vec<_>, Vec<_>) = term_documents_deleted.into_iter().unzip();
        let term_documents_deleted = DacsOpt::build_from_slice(&counts)?;
        let deleted_terms_table = term_ordinals
            .last()
            .map(|highest_term_ord| {
                let mut elias_fano_builder =
                    EliasFanoBuilder::new((*highest_term_ord + 1) as usize, term_ordinals.len())?;
                elias_fano_builder.extend(term_ordinals.iter().map(|x| *x as usize))?;
                let term_ordinals = elias_fano_builder.build();
                anyhow::Ok(DeletedTermsTable {
                    term_ordinals,
                    term_documents_deleted,
                    num_terms_deleted,
                })
            })
            .transpose()?;
        Ok(deleted_terms_table)
    }
}

#[minitrace::trace]
pub fn load_alive_bitset(path: &Path) -> anyhow::Result<AliveBitSet> {
    let _timer = load_alive_bitset_timer();
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    log_alive_bitset_size(size as usize);
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    let owned = OwnedBytes::new(buf);
    let alive_bitset = AliveBitSet::open(owned);
    Ok(alive_bitset)
}

impl StaticDeletionTracker {
    // TODO(CX-6513) Remove after migrating to multisegment index
    pub fn empty(num_docs: u32) -> Self {
        Self {
            alive_bitset: AliveBitSet::from_bitset(&BitSet::with_max_value_and_full(num_docs)),
            deleted_terms_by_field: BTreeMap::new(),
        }
    }

    pub fn load(alive_bitset: AliveBitSet, deleted_terms_path: &Path) -> anyhow::Result<Self> {
        let deleted_terms_file = File::open(deleted_terms_path)?;
        let deleted_terms_file_len = deleted_terms_file.metadata()?.len() as usize;
        let deleted_terms_reader = BufReader::new(deleted_terms_file);
        let deleted_terms_by_field =
            Self::load_deleted_terms(deleted_terms_file_len, deleted_terms_reader)?;
        Ok(Self {
            alive_bitset,
            deleted_terms_by_field,
        })
    }

    #[minitrace::trace]
    fn load_deleted_terms(
        file_len: usize,
        mut reader: impl Read,
    ) -> anyhow::Result<BTreeMap<Field, DeletedTermsTable>> {
        log_deleted_terms_table_size(file_len);
        let _timer = load_deleted_terms_table_timer();
        let mut expected_len = 0;
        let version = reader.read_u8()?;
        expected_len += 1;
        anyhow::ensure!(version == DELETED_TERMS_TABLE_VERSION);

        let field_header_size = reader.read_u16::<LittleEndian>()?;
        expected_len += 2;

        let mut field_headers = vec![];
        for _ in 0..field_header_size / SIZE_PER_FIELD_HEADER {
            let field_id = reader.read_u32::<LittleEndian>()?;
            expected_len += 4;
            let num_terms_deleted = reader.read_u64::<LittleEndian>()?;
            expected_len += 8;
            let deleted_term_ordinals_size = reader.read_u32::<LittleEndian>()?;
            expected_len += 4;
            let counts_size = reader.read_u32::<LittleEndian>()?;
            expected_len += 4;

            field_headers.push(FieldHeader {
                field: Field::from_field_id(field_id),
                num_terms_deleted,
                deleted_term_ordinals_size,
                counts_size,
            });
        }

        let mut deleted_terms_by_field = BTreeMap::new();
        for FieldHeader {
            field,
            num_terms_deleted,
            deleted_term_ordinals_size,
            counts_size,
        } in field_headers
        {
            let mut elias_fano_buf = vec![0; deleted_term_ordinals_size as usize];
            reader.read_exact(&mut elias_fano_buf).with_context(|| {
                format!("Expected to fill EliasFano buffer with {deleted_term_ordinals_size} bytes")
            })?;
            expected_len += deleted_term_ordinals_size;
            let term_ordinals = EliasFano::deserialize_from(&elias_fano_buf[..])?;
            let mut counts_buf = vec![0; counts_size as usize];
            reader.read_exact(&mut counts_buf)?;
            expected_len += counts_size;
            let term_documents_deleted = DacsOpt::deserialize_from(&counts_buf[..])?;
            let deleted_terms_table = DeletedTermsTable {
                term_ordinals,
                term_documents_deleted,
                num_terms_deleted,
            };
            deleted_terms_by_field.insert(field, deleted_terms_table);
        }
        anyhow::ensure!(
            file_len == expected_len as usize,
            "Deleted terms file length mismatch, expected {expected_len}, got {file_len}"
        );
        Ok(deleted_terms_by_field)
    }

    pub fn doc_frequency(
        &self,
        field: Field,
        term_dict: &TermDictionary,
        term_ord: TermOrdinal,
    ) -> anyhow::Result<u64> {
        let term_info = term_dict.term_info_from_ord(term_ord);
        let term_documents_deleted = self.term_documents_deleted(field, term_ord)?;
        (term_info.doc_freq as u64)
            .checked_sub(term_documents_deleted as u64)
            .context("doc_frequency underflow")
    }

    /// Number of non-unique terms that have been deleted from a field
    pub fn num_terms_deleted(&self, field: Field) -> u64 {
        self.deleted_terms_by_field
            .get(&field)
            .map_or(0, |t| t.num_terms_deleted)
    }

    /// How many documents in the segment are not deleted?
    pub fn num_alive_docs(&self) -> usize {
        self.alive_bitset.num_alive_docs()
    }

    /// How many of a term's documents have been deleted?
    pub fn term_documents_deleted(
        &self,
        field: Field,
        term_ord: TermOrdinal,
    ) -> anyhow::Result<u32> {
        self.deleted_terms_by_field
            .get(&field)
            .map_or(Ok(0), |t| t.term_documents_deleted(term_ord))
    }

    /// Which documents have been deleted in the segment?
    pub fn alive_bitset(&self) -> &AliveBitSet {
        &self.alive_bitset
    }
}

#[derive(Default, Debug)]
pub struct SearchMemoryIdTracker(MemoryIdTracker);
impl SearchMemoryIdTracker {
    pub fn set_link(&mut self, convex_id: InternalId, tantivy_id: DocId) -> anyhow::Result<()> {
        let maybe_id = self.0.index_id(convex_id.0);
        anyhow::ensure!(
            maybe_id.is_none(),
            "Id {convex_id} already exists in SearchIdTracker with tantivy id: {maybe_id:?}, was \
             going to be set to {tantivy_id}"
        );
        self.0.insert(tantivy_id, convex_id.0);
        Ok(())
    }

    pub fn num_ids(&self) -> usize {
        self.0.by_convex_id.len()
    }

    #[minitrace::trace]
    pub fn write<P: AsRef<Path>>(mut self, id_tracker_path: P) -> anyhow::Result<()> {
        let mut out = BufWriter::new(File::create(id_tracker_path)?);
        self.0.write_id_tracker(&mut out)?;
        out.into_inner()?.sync_all()?;
        Ok(())
    }
}

// TODO(CX-6565): Make protos private
#[cfg_attr(any(test, feature = "testing"), derive(PartialEq, Debug, Clone))]
#[derive(Default)]
/// Mutable data structure for tracking term deletions for a field in a segment.
/// [DeletedTermsTable] is a more efficient read-only version of this data
/// structure.
pub struct FieldTermMetadata {
    /// The number of documents containing the term that have been deleted, by
    /// term ordinal.
    pub term_documents_deleted: BTreeMap<TermOrdinal, u32>,
    /// The number of non-unique terms that have been deleted from the field's
    /// inverted index.
    pub num_terms_deleted: u64,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for FieldTermMetadata {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = FieldTermMetadata>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<u64>(),
            prop::collection::btree_map(any::<u64>(), any::<u32>(), 0..10),
        )
            .prop_filter_map(
                "Invalid FieldTermMetadata",
                |(num_terms_deleted, term_documents_deleted)| {
                    if (num_terms_deleted == 0) != term_documents_deleted.is_empty() {
                        return None;
                    }
                    Some(FieldTermMetadata {
                        term_documents_deleted,
                        num_terms_deleted,
                    })
                },
            )
    }
}

#[cfg_attr(any(test, feature = "testing"), derive(PartialEq, Debug, Clone))]
#[derive(Default)]
/// Term deletion metadata for a segment, tracked by field.
pub struct SegmentTermMetadata {
    pub term_metadata_by_field: BTreeMap<Field, FieldTermMetadata>,
}

#[cfg(any(test, feature = "testing"))]
impl proptest::arbitrary::Arbitrary for SegmentTermMetadata {
    type Parameters = ();

    type Strategy = impl proptest::strategy::Strategy<Value = SegmentTermMetadata>;

    fn arbitrary_with((): Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        prop::collection::btree_map(any::<u32>(), any::<FieldTermMetadata>(), 0..10).prop_map(
            |term_documents_deleted| {
                let term_documents_deleted = term_documents_deleted
                    .into_iter()
                    .map(|(k, v)| (Field::from_field_id(k), v))
                    .collect();
                SegmentTermMetadata {
                    term_metadata_by_field: term_documents_deleted,
                }
            },
        )
    }
}

impl AddAssign for SegmentTermMetadata {
    fn add_assign(&mut self, other: Self) {
        for (field, other_field_metadata) in other.term_metadata_by_field {
            let field_metadata = self.term_metadata_by_field.entry(field).or_default();
            for (term_ord, num_docs_deleted) in other_field_metadata.term_documents_deleted {
                field_metadata
                    .term_documents_deleted
                    .entry(term_ord)
                    .and_modify(|n| *n += num_docs_deleted)
                    .or_insert(num_docs_deleted);
            }
            field_metadata.num_terms_deleted += other_field_metadata.num_terms_deleted;
        }
    }
}

impl SegmentTermMetadata {
    pub fn write(self, mut out: impl Write) -> anyhow::Result<()> {
        out.write_u8(DELETED_TERMS_TABLE_VERSION)?;
        let mut deleted_terms_tables = BTreeMap::new();
        for (field, metadata) in self.term_metadata_by_field {
            if let Some(table) = Option::<DeletedTermsTable>::try_from(metadata)? {
                deleted_terms_tables.insert(field, table);
            }
        }
        let field_header_size =
            <u16>::try_from(deleted_terms_tables.len())? * SIZE_PER_FIELD_HEADER;
        out.write_u16::<LittleEndian>(field_header_size)?;
        if deleted_terms_tables.is_empty() {
            out.flush()?;
            return Ok(());
        }
        for (
            field,
            DeletedTermsTable {
                num_terms_deleted,
                term_ordinals,
                term_documents_deleted,
            },
        ) in &deleted_terms_tables
        {
            out.write_u32::<LittleEndian>(field.field_id())?;
            out.write_u64::<LittleEndian>(*num_terms_deleted)?;
            out.write_u32::<LittleEndian>(term_ordinals.size_in_bytes().try_into()?)?;
            out.write_u32::<LittleEndian>(term_documents_deleted.size_in_bytes().try_into()?)?;
        }

        for (
            _,
            DeletedTermsTable {
                term_ordinals,
                term_documents_deleted,
                ..
            },
        ) in deleted_terms_tables
        {
            term_ordinals.serialize_into(&mut out)?;
            term_documents_deleted.serialize_into(&mut out)?;
        }
        out.flush()?;
        Ok(())
    }
}

pub struct MemoryDeletionTracker {
    pub alive_bitset: BitSet,
    segment_term_metadata: SegmentTermMetadata,
}

impl MemoryDeletionTracker {
    pub fn new(num_docs: u32) -> Self {
        Self {
            alive_bitset: BitSet::with_max_value_and_full(num_docs),
            segment_term_metadata: SegmentTermMetadata::default(),
        }
    }

    #[minitrace::trace]
    pub fn load(alive_bitset_path: &Path, deleted_terms_path: &Path) -> anyhow::Result<Self> {
        let alive_bitset_reader = BufReader::new(File::open(alive_bitset_path)?);
        let alive_bitset = BitSet::deserialize(alive_bitset_reader)?;
        let deleted_terms_file = File::open(deleted_terms_path)?;
        let file_len = deleted_terms_file.metadata()?.len() as usize;
        let deleted_terms_reader = BufReader::new(deleted_terms_file);
        let deleted_terms_by_field =
            StaticDeletionTracker::load_deleted_terms(file_len, deleted_terms_reader)?;
        let term_metadata_by_field = deleted_terms_by_field
            .into_iter()
            .map(|(field, t)| (field, t.into()))
            .collect();
        let segment_term_metadata = SegmentTermMetadata {
            term_metadata_by_field,
        };
        Ok(Self {
            alive_bitset,
            segment_term_metadata,
        })
    }

    pub fn delete_document(
        &mut self,
        convex_id: InternalId,
        id_tracker: &StaticIdTracker,
    ) -> anyhow::Result<()> {
        let tantivy_id = id_tracker
            .lookup(convex_id.0)
            .with_context(|| format!("Id not found in StaticIdTracker: {:?}", convex_id))?;
        self.alive_bitset.remove(tantivy_id);
        Ok(())
    }

    pub fn update_term_metadata(&mut self, segment_term_metadata: SegmentTermMetadata) {
        self.segment_term_metadata += segment_term_metadata;
    }

    pub fn write_to_path<P: AsRef<Path>>(
        self,
        alive_bitset_path: P,
        deleted_terms_path: P,
    ) -> anyhow::Result<()> {
        let mut alive_bitset = BufWriter::new(File::create(alive_bitset_path)?);
        let mut deleted_terms = BufWriter::new(File::create(deleted_terms_path)?);
        self.write(&mut alive_bitset, &mut deleted_terms)?;
        alive_bitset.into_inner()?.sync_all()?;
        deleted_terms.into_inner()?.sync_all()?;
        Ok(())
    }

    #[minitrace::trace]
    pub fn write(
        self,
        mut alive_bitset: impl Write,
        mut deleted_terms: impl Write,
    ) -> anyhow::Result<()> {
        self.alive_bitset.serialize(&mut alive_bitset)?;
        self.segment_term_metadata.write(&mut deleted_terms)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use maplit::btreemap;
    use proptest::prelude::*;
    use tantivy::schema::Field;

    use super::{
        FieldTermMetadata,
        MemoryDeletionTracker,
    };
    use crate::tracker::{
        SegmentTermMetadata,
        StaticDeletionTracker,
    };

    #[test]
    fn test_empty_deleted_term_table_roundtrips() -> anyhow::Result<()> {
        let memory_tracker = MemoryDeletionTracker::new(10);
        let mut buf = Vec::new();
        memory_tracker.segment_term_metadata.write(&mut buf)?;
        let file_len = buf.len();
        assert!(StaticDeletionTracker::load_deleted_terms(file_len, &buf[..])?.is_empty());
        Ok(())
    }

    #[test]
    fn test_deleted_term_table_roundtrips() -> anyhow::Result<()> {
        let mut memory_tracker = MemoryDeletionTracker::new(10);
        let term_documents_deleted = btreemap! {
            5 => 2,
            3 => 1,
        };
        let field = Field::from_field_id(1);
        let segment_term_metadata = SegmentTermMetadata {
            term_metadata_by_field: btreemap! {
                field => FieldTermMetadata {
                    term_documents_deleted,
                    num_terms_deleted: 0,
                }
            },
        };
        memory_tracker.update_term_metadata(segment_term_metadata);

        let mut buf = Vec::new();
        memory_tracker.segment_term_metadata.write(&mut buf)?;

        let file_len = buf.len();
        let deleted_terms_tables = StaticDeletionTracker::load_deleted_terms(file_len, &buf[..])?;
        assert_eq!(deleted_terms_tables.len(), 1);
        let deleted_terms_table = deleted_terms_tables.get(&field).unwrap();
        assert_eq!(deleted_terms_table.term_documents_deleted(5)?, 2);
        assert_eq!(deleted_terms_table.term_documents_deleted(3)?, 1);
        Ok(())
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]
        #[test]
        fn randomized_deleted_term_table(segment_term_metadata in any::<SegmentTermMetadata>())  {
            let mut memory_tracker = MemoryDeletionTracker::new(10);
            memory_tracker.update_term_metadata(segment_term_metadata.clone());
            let mut buf = Vec::new();
            memory_tracker.segment_term_metadata.clone().write(&mut buf).unwrap();

            let file_len = buf.len();
            let deleted_terms_tables = StaticDeletionTracker::load_deleted_terms(
                file_len, &buf[..]
            ).unwrap();
            for (field, deleted_terms_table) in deleted_terms_tables {
                let field_term_metadata = segment_term_metadata
                    .term_metadata_by_field
                    .get(&field).unwrap();
                let field_term_metadata_read = FieldTermMetadata::from(deleted_terms_table);
                assert_eq!(field_term_metadata, &field_term_metadata_read);
            }

        }
    }
}
