use std::{
    collections::BTreeMap,
    fs::File,
    io::{
        BufReader,
        BufWriter,
        Read,
        Write,
    },
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

/// Version 1 of the deletion tracker has the following format:
/// ```
/// [ version ] [ num_terms_deleted ] [ deleted_term_ordinals_size ] [ counts_size ] [ deleted_term_ordinals ] [ counts ]
/// ```
/// - version (u8): version number for the file format
/// - num_terms_deleted (little-endian u32): number of terms that are completely
///   deleted from the segment
/// - deleted_term_ordinals_size (little-endian u32): size of the term ordinals
///   EliasFano
/// - counts_size (little-endian u32): size of the DacsOpt encoded counts of
///   deleted documents per term
/// - deleted_term_ordinals: EliasFano-encoded list of term ordinals from
///   deleted documents
/// - counts (DacsOpt): number of deleted documents per term, indexed in the
///   same order as `deleted_term_ordinals`
pub const DELETED_TERMS_TABLE_VERSION: u8 = 1;

pub struct StaticDeletionTracker {
    alive_bitset: AliveBitSet,
    /// Number of terms that are completed deleted from the segment
    num_terms_deleted: u32,
    deleted_terms_table: Option<DeletedTermsTable>,
}

struct DeletedTermsTable {
    term_ordinals: EliasFano,
    term_documents_deleted: DacsOpt,
}

impl DeletedTermsTable {
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

pub fn load_alive_bitset(path: &Path) -> anyhow::Result<AliveBitSet> {
    let mut file = File::open(path)?;
    let mut buf = vec![];
    file.read_to_end(&mut buf)?;
    let owned = OwnedBytes::new(buf);
    let alive_bitset = AliveBitSet::open(owned);
    Ok(alive_bitset)
}

impl StaticDeletionTracker {
    pub fn load(alive_bitset: AliveBitSet, deleted_terms_path: &Path) -> anyhow::Result<Self> {
        let deleted_terms_file = File::open(deleted_terms_path)?;
        let deleted_terms_file_len = deleted_terms_file.metadata()?.len() as usize;
        let deleted_terms_reader = BufReader::new(deleted_terms_file);
        let (num_terms_deleted, deleted_terms_table) =
            Self::load_deleted_terms_table(deleted_terms_file_len, deleted_terms_reader)?;

        Ok(Self {
            alive_bitset,
            num_terms_deleted,
            deleted_terms_table,
        })
    }

    pub fn doc_frequency(
        &self,
        term_dict: &TermDictionary,
        term_ord: TermOrdinal,
    ) -> anyhow::Result<u64> {
        let term_info = term_dict.term_info_from_ord(term_ord);
        let term_documents_deleted = self.term_documents_deleted(term_ord)?;
        (term_info.doc_freq as u64)
            .checked_sub(term_documents_deleted as u64)
            .context("doc_frequency underflow")
    }

    /// How many terms have been completely deleted from the segment?
    pub fn num_terms_deleted(&self) -> u32 {
        self.num_terms_deleted
    }

    /// How many documents in the segment are not deleted?
    pub fn num_alive_docs(&self) -> usize {
        self.alive_bitset.num_alive_docs()
    }

    /// How many of a term's documents have been deleted?
    pub fn term_documents_deleted(&self, term_ord: TermOrdinal) -> anyhow::Result<u32> {
        if let Some(deleted_terms) = &self.deleted_terms_table {
            deleted_terms.term_documents_deleted(term_ord)
        } else {
            Ok(0)
        }
    }

    /// Which documents have been deleted in the segment?
    pub fn alive_bitset(&self) -> &AliveBitSet {
        &self.alive_bitset
    }

    fn load_deleted_terms_table(
        file_len: usize,
        mut reader: impl Read,
    ) -> anyhow::Result<(u32, Option<DeletedTermsTable>)> {
        let mut expected_len = 0;
        let version = reader.read_u8()?;
        expected_len += 1;
        anyhow::ensure!(version == DELETED_TERMS_TABLE_VERSION);

        let num_terms_deleted = reader.read_u32::<LittleEndian>()?;
        expected_len += 4;

        let deleted_term_ordinals_size = reader.read_u32::<LittleEndian>()? as usize;
        expected_len += 4;
        if deleted_term_ordinals_size == 0 {
            return Ok((num_terms_deleted, None));
        }

        let counts_size = reader.read_u32::<LittleEndian>()? as usize;
        expected_len += 4;

        let mut elias_fano_buf = vec![0; deleted_term_ordinals_size];
        reader.read_exact(&mut elias_fano_buf).with_context(|| {
            format!("Expected to fill EliasFano buffer with {deleted_term_ordinals_size} bytes")
        })?;
        expected_len += deleted_term_ordinals_size; // deleted_term_ordinals
        let term_ordinals = EliasFano::deserialize_from(&elias_fano_buf[..])?;
        let mut counts_buf = vec![0; counts_size];
        reader.read_exact(&mut counts_buf)?;
        expected_len += counts_size;
        let term_documents_deleted = DacsOpt::deserialize_from(&counts_buf[..])?;

        anyhow::ensure!(
            file_len == expected_len,
            "Deleted terms file length mismatch, expected {expected_len}, got {file_len}"
        );
        Ok((
            num_terms_deleted,
            Some(DeletedTermsTable {
                term_ordinals,
                term_documents_deleted,
            }),
        ))
    }
}

#[derive(Default)]
pub struct SearchMemoryIdTracker(MemoryIdTracker);
impl SearchMemoryIdTracker {
    pub fn set_link(&mut self, convex_id: InternalId, tantivy_id: DocId) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.0.index_id(convex_id.0).is_none(),
            "Id {convex_id} already exists in SearchIdTracker"
        );
        self.0.insert(tantivy_id, convex_id.0);
        Ok(())
    }

    pub fn write<P: AsRef<Path>>(mut self, id_tracker_path: P) -> anyhow::Result<()> {
        let mut out = BufWriter::new(File::create(id_tracker_path)?);
        self.0.write_id_tracker(&mut out)?;
        out.into_inner()?.sync_all()?;
        Ok(())
    }
}

pub struct MemoryDeletionTracker {
    alive_bitset: BitSet,
    term_to_deleted_documents: BTreeMap<TermOrdinal, u32>,
    num_deleted_terms: u32,
}

impl MemoryDeletionTracker {
    pub fn new(num_docs: u32) -> Self {
        Self {
            alive_bitset: BitSet::with_max_value_and_full(num_docs),
            term_to_deleted_documents: BTreeMap::new(),
            num_deleted_terms: 0,
        }
    }

    pub fn delete_document(
        &mut self,
        convex_id: InternalId,
        id_tracker: StaticIdTracker,
    ) -> anyhow::Result<()> {
        let tantivy_id = id_tracker
            .lookup(convex_id.0)
            .with_context(|| format!("Id not found in StaticIdTracker: {:?}", convex_id))?;
        self.alive_bitset.remove(tantivy_id);
        Ok(())
    }

    pub fn increment_deleted_documents_for_term(&mut self, term_ord: TermOrdinal, count: u32) {
        self.term_to_deleted_documents
            .entry(term_ord)
            .and_modify(|n| *n += count)
            .or_insert(count);
    }

    pub fn set_num_deleted_terms(&mut self, num_deleted_terms: u32) {
        self.num_deleted_terms = num_deleted_terms;
    }

    pub fn write<P: AsRef<Path>>(
        self,
        alive_bitset_path: P,
        deleted_terms_path: P,
    ) -> anyhow::Result<()> {
        {
            let mut out = BufWriter::new(File::create(alive_bitset_path)?);
            self.alive_bitset.serialize(&mut out)?;
            out.into_inner()?.sync_all()?;
        }
        {
            let mut out = BufWriter::new(File::create(deleted_terms_path)?);
            Self::write_deleted_terms(
                self.term_to_deleted_documents,
                self.num_deleted_terms,
                &mut out,
            )?;
            out.into_inner()?.sync_all()?;
        }
        Ok(())
    }

    fn write_deleted_terms(
        term_to_deleted_documents: BTreeMap<TermOrdinal, u32>,
        num_deleted_terms: u32,
        mut out: impl Write,
    ) -> anyhow::Result<()> {
        out.write_u8(DELETED_TERMS_TABLE_VERSION)?;
        out.write_u32::<LittleEndian>(num_deleted_terms)?;
        let (term_ordinals, counts): (Vec<_>, Vec<_>) =
            term_to_deleted_documents.into_iter().unzip();
        let dacs_opt = DacsOpt::build_from_slice(&counts)?;
        let maybe_elias_fano = term_ordinals
            .last()
            .map(|last| {
                let mut elias_fano_builder =
                    EliasFanoBuilder::new((*last + 1) as usize, term_ordinals.len())?;
                elias_fano_builder.extend(term_ordinals.iter().map(|x| *x as usize))?;
                let elias_fano = elias_fano_builder.build();
                anyhow::Ok(elias_fano)
            })
            .transpose()?;
        let elias_fano_size = maybe_elias_fano
            .as_ref()
            .map_or(0, |elias_fano| elias_fano.size_in_bytes());
        out.write_u32::<LittleEndian>(elias_fano_size.try_into()?)?;
        out.write_u32::<LittleEndian>(dacs_opt.size_in_bytes().try_into()?)?;
        if let Some(elias_fano) = maybe_elias_fano {
            elias_fano.serialize_into(&mut out)?;
        }
        dacs_opt.serialize_into(&mut out)?;
        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::{
        MemoryDeletionTracker,
        StaticDeletionTracker,
    };

    #[test]
    fn test_empty_deleted_term_table_roundtrips() -> anyhow::Result<()> {
        let memory_tracker = MemoryDeletionTracker::new(10);
        let mut buf = Vec::new();
        MemoryDeletionTracker::write_deleted_terms(
            memory_tracker.term_to_deleted_documents,
            memory_tracker.num_deleted_terms,
            &mut buf,
        )?;
        let file_len = buf.len();
        assert!(
            StaticDeletionTracker::load_deleted_terms_table(file_len, &buf[..])?
                .1
                .is_none()
        );
        Ok(())
    }

    #[test]
    fn test_deleted_term_table_roundtrips() -> anyhow::Result<()> {
        let mut memory_tracker = MemoryDeletionTracker::new(10);
        let term_ord_1 = 5;
        memory_tracker.increment_deleted_documents_for_term(term_ord_1, 2);
        let term_ord_2 = 3;
        memory_tracker.increment_deleted_documents_for_term(term_ord_2, 1);

        let mut buf = Vec::new();
        MemoryDeletionTracker::write_deleted_terms(
            memory_tracker.term_to_deleted_documents,
            memory_tracker.num_deleted_terms,
            &mut buf,
        )?;

        let file_len = buf.len();
        let (num_deleted_terms, deleted_terms_table) =
            StaticDeletionTracker::load_deleted_terms_table(file_len, &buf[..])?;
        assert_eq!(num_deleted_terms, 0);
        let deleted_terms_table = deleted_terms_table.unwrap();
        assert_eq!(deleted_terms_table.term_documents_deleted(term_ord_1)?, 2);
        assert_eq!(deleted_terms_table.term_documents_deleted(term_ord_2)?, 1);
        Ok(())
    }
}
