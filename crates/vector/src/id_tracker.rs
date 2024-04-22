use std::{
    collections::BTreeMap,
    fs::File,
    io::{
        BufReader,
        BufWriter,
        Read,
        Write,
    },
    iter,
    mem,
    path::PathBuf,
};

use bitvec::{
    slice::BitSlice,
    vec::BitVec,
};
use byteorder::{
    LittleEndian,
    ReadBytesExt,
    WriteBytesExt,
};
use csf::ls::Map as CsfMap;
use qdrant_segment::{
    common::Flusher,
    entry::entry_point::{
        OperationError,
        OperationResult,
    },
    id_tracker::IdTracker,
    types::{
        ExtendedPointId,
        PointIdType,
        PointOffsetType,
        SeqNumberType,
    },
};
use uuid::Uuid;

/// Qdrant has a notion of "operation number" that it uses for tracking
/// mutations. Since we only use it as a static index, require that all points'
/// operation numbers are always 1.
pub const OP_NUM: SeqNumberType = 1;

/// Version 1 of The UUID table has the following format:
/// ```
/// [ version ] [ count ] [ index_len ] [ UUID ]* [ index ]
/// ```
/// - version (u8): version number for the file format
/// - count (little-endian u32): number of UUIDs
/// - index_len (little-endian u32): length of `index` in bytes
/// - UUIDs (dense array of 16 byte UUIDs): UUIDs in offset order
/// - index: perfect hash table mapping UUID to offset.
pub const UUID_TABLE_VERSION: u8 = 1;

/// Version 1 of the deleted bitset has the following format:
/// ```
/// [ version ] [ count ] [ num_deleted ] [ bitset block ]*
/// ```
/// - version (u8): version number for the file format
/// - count (little-endian u32): number of *bits* in the bitset
/// - num_deleted (little-endian u32): number of set bits in the bitset
/// - bitset blocks (dense array of little-endian u64s): bitset contents
pub const DELETED_BITSET_VERSION: u8 = 1;

/// Restricted implementation of `IdTracker` that assumes...
///
/// 1. All operation numbers are OP_NUM.
/// 2. The application only uses UUID point IDs.
/// 3. The set of offsets used is dense (i.e. `0..self.len()`).
pub struct MemoryIdTracker {
    by_offset: BTreeMap<u32, Uuid>,
    by_uuid: BTreeMap<Uuid, u32>,

    // We don't actually support deletes here but keep this empty bitset around
    // to use in `deleted_point_bitslice`.
    deleted: DeletedBitset,
}

impl MemoryIdTracker {
    pub fn new() -> Self {
        Self {
            deleted: DeletedBitset::new(0),
            by_offset: BTreeMap::new(),
            by_uuid: BTreeMap::new(),
        }
    }

    /// Similar to IdTracker.external_id, but will return Some even if the id
    /// has been deleted so long as the internal id is valid.
    #[cfg(test)]
    pub(crate) fn external_id_with_deleted(
        &self,
        internal_id: PointOffsetType,
    ) -> Option<PointIdType> {
        self._internal_to_external_id(internal_id)
    }

    fn _internal_to_external_id(&self, internal_id: PointOffsetType) -> Option<PointIdType> {
        self.by_offset
            .get(&internal_id)
            .map(|uuid| PointIdType::Uuid(*uuid))
    }
}

impl IdTracker for MemoryIdTracker {
    fn internal_version(&self, internal_id: PointOffsetType) -> Option<SeqNumberType> {
        self.external_id(internal_id).map(|_| OP_NUM)
    }

    fn set_internal_version(
        &mut self,
        _internal_id: PointOffsetType,
        version: SeqNumberType,
    ) -> OperationResult<()> {
        if version != OP_NUM {
            return Err(OperationError::service_error(format!(
                "Invalid version: {version}"
            )));
        }
        Ok(())
    }

    fn internal_id(&self, external_id: PointIdType) -> Option<PointOffsetType> {
        let PointIdType::Uuid(uuid) = external_id else {
            panic!("Invalid external ID: {external_id}");
        };
        self.by_uuid.get(&uuid).map(|ix| *ix as PointOffsetType)
    }

    fn external_id(&self, internal_id: PointOffsetType) -> Option<PointIdType> {
        if self.is_deleted_point(internal_id) {
            return None;
        }
        self._internal_to_external_id(internal_id)
    }

    fn set_link(
        &mut self,
        external_id: PointIdType,
        internal_id: PointOffsetType,
    ) -> OperationResult<()> {
        let PointIdType::Uuid(uuid) = external_id else {
            panic!("Invalid external ID: {external_id}");
        };
        self.by_offset.insert(internal_id, uuid);
        self.by_uuid.insert(uuid, internal_id);
        self.deleted.resize(internal_id as usize + 1);
        Ok(())
    }

    fn drop(&mut self, external_id: PointIdType) -> OperationResult<()> {
        let Some(internal_id) = self.internal_id(external_id) else {
            panic!("Unrecognized external id: {external_id}");
        };
        self.deleted.delete(internal_id)?;
        Ok(())
    }

    fn iter_external(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        Box::new(
            self.by_offset
                .iter()
                .filter(|(key, _)| !self.is_deleted_point(**key))
                .map(|(_, uuid)| PointIdType::Uuid(*uuid)),
        )
    }

    fn iter_internal(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(self.by_offset.keys().copied())
    }

    fn iter_from(
        &self,
        external_id: Option<PointIdType>,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        // All `NumId`s sort before all `Uuid`s in qdrant's order, so effectively ignore
        // a `NumId` lower bound.
        let Some(minimum) = self.by_uuid.keys().next().copied() else {
            return Box::new(iter::empty());
        };
        let lower_bound = external_id
            .and_then(|id| match id {
                ExtendedPointId::NumId(..) => None,
                ExtendedPointId::Uuid(uuid) => Some(uuid),
            })
            .unwrap_or(minimum);
        let iter = self
            .by_uuid
            .range(lower_bound..)
            .map(|(k, v)| (PointIdType::Uuid(*k), *v as PointOffsetType));
        Box::new(iter)
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(
            self.by_offset
                .keys()
                .filter(|internal_id| !self.deleted.is_deleted_point(**internal_id))
                .copied(),
        )
    }

    fn mapping_flusher(&self) -> Flusher {
        // Do nothing when Qdrant asks us to flush the mapping to disk. We fully
        // manage ID mappings ourselves in separate index files that Qdrant isn't
        // aware of, so there's nothing to do here.
        Box::new(|| OperationResult::Ok(()))
    }

    fn versions_flusher(&self) -> Flusher {
        // We don't allow mutating points' versions (see `set_internal_version`) to
        // anything other than `OP_NUM`, so there's nothing to flush here either.
        Box::new(|| OperationResult::Ok(()))
    }

    fn total_point_count(&self) -> usize {
        self.by_offset.len()
    }

    fn deleted_point_count(&self) -> usize {
        self.deleted.num_deleted()
    }

    fn deleted_point_bitslice(&self) -> &BitSlice {
        self.deleted.deleted_point_bitslice()
    }

    fn is_deleted_point(&self, internal_id: PointOffsetType) -> bool {
        self.deleted.is_deleted_point(internal_id)
    }
}

impl MemoryIdTracker {
    pub fn check_invariants(&mut self) -> anyhow::Result<()> {
        if let Some(last_offset) = self.by_offset.keys().next_back() {
            anyhow::ensure!(
                *last_offset as usize == self.by_offset.len() - 1,
                "Non-contiguous offsets"
            );
        }

        self.deleted.check_invariants()?;

        Ok(())
    }

    /// Write out a static file with the ID tracker's UUIDs and offsets.
    pub fn write_uuids(&mut self, mut out: impl Write) -> anyhow::Result<()> {
        self.check_invariants()?;

        // Build up a flat array of the UUIDs in offset order.
        let uuids = self
            .by_offset
            .values()
            .map(|uuid| uuid.as_bytes())
            .collect::<Vec<_>>();

        // Compute the number of bits needed to represent each offset.
        let offset_bits = uuids.len().next_power_of_two().trailing_zeros();

        // Build the perfect hash table.
        let map = CsfMap::try_with_fn::<_, _, ()>(&uuids, |i, _| i as u64, offset_bits as u8)
            .ok_or_else(|| anyhow::anyhow!("Failed to create CsfMap"))?;

        out.write_u8(UUID_TABLE_VERSION)?;
        out.write_u32::<LittleEndian>(uuids.len().try_into()?)?;
        out.write_u32::<LittleEndian>(map.write_bytes().try_into()?)?;
        for uuid in &uuids {
            out.write_all(&uuid[..])?;
        }
        map.write(&mut out)?;

        out.flush()?;

        Ok(())
    }

    pub fn write_deleted_bitset(&mut self, out: impl Write) -> anyhow::Result<()> {
        self.deleted.write(out)
    }
}

#[derive(Clone)]
pub struct DeletedBitset {
    deleted: BitVec,
    num_deleted: usize,
}

impl DeletedBitset {
    pub fn new(count: usize) -> Self {
        let mut result = DeletedBitset {
            deleted: BitVec::new(),
            num_deleted: 0,
        };
        result.resize(count);
        result
    }

    fn resize(&mut self, size: usize) {
        self.deleted.resize(size, false);
    }

    pub fn num_deleted(&self) -> usize {
        self.num_deleted
    }

    fn len(&self) -> usize {
        self.deleted.len()
    }

    pub fn is_deleted_point(&self, internal_id: PointOffsetType) -> bool {
        let offset = internal_id as usize;
        offset >= self.deleted.len() || self.deleted[offset]
    }

    pub fn deleted_point_bitslice(&self) -> &BitSlice {
        &self.deleted
    }

    fn check_invariants(&mut self) -> anyhow::Result<()> {
        anyhow::ensure!(self.num_deleted == self.deleted.count_ones());
        // We shouldn't hit these codepaths, but `BitVec` can have its physical blocks
        // not be aligned with the logical bit vector.
        self.deleted.force_align();
        self.deleted.set_uninitialized(false);
        Ok(())
    }

    pub fn delete(&mut self, internal_id: PointOffsetType) -> OperationResult<()> {
        if self.is_deleted_point(internal_id) {
            return OperationResult::Err(OperationError::InconsistentStorage {
                description: format!("Trying to delete {internal_id:?} twice"),
            });
        }
        self.deleted.set(internal_id as usize, true);
        self.num_deleted += 1;
        Ok(())
    }

    pub fn write_to_path(&mut self, path: PathBuf) -> anyhow::Result<()> {
        let mut out = BufWriter::new(File::create(path)?);
        self.write(&mut out)?;
        out.into_inner()?.sync_all()?;
        Ok(())
    }

    /// Write out a static file with the ID tracker's (empty) deleted bitset.
    pub fn write(&mut self, mut out: impl Write) -> anyhow::Result<()> {
        self.check_invariants()?;

        let count = self.len();
        let num_deleted = self.num_deleted as u32;
        let expected_blocks = count.next_multiple_of(64) / 64;
        anyhow::ensure!(self.deleted.as_raw_slice().len() == expected_blocks);

        // Unfortunately, `bitset` wants `usize`s but we want to serialize `u64`s out to
        // disk. Check that we're on a 64-bit platform.
        anyhow::ensure!(mem::size_of::<usize>() * 8 == 64);

        out.write_u8(DELETED_BITSET_VERSION)?;
        out.write_u32::<LittleEndian>(count.try_into()?)?;
        out.write_u32::<LittleEndian>(num_deleted)?;
        for block in self.deleted.as_raw_slice() {
            out.write_u64::<LittleEndian>(*block as u64)?;
        }
        out.flush()?;

        Ok(())
    }

    pub fn load_from_path(path: PathBuf) -> anyhow::Result<Self> {
        let deleted_file = File::open(path)?;
        Self::load(
            deleted_file.metadata()?.len() as usize,
            BufReader::new(deleted_file),
        )
    }

    pub fn load(file_len: usize, mut reader: impl Read) -> anyhow::Result<Self> {
        // As when writing out the index, we need to ensure we're on a 64-bit platform,
        // since `byteorder` wants `u64`s but `BitVec` wants `usize`s.
        anyhow::ensure!(mem::size_of::<usize>() * 8 == 64);

        anyhow::ensure!(reader.read_u8()? == DELETED_BITSET_VERSION);
        let num_bits = reader.read_u32::<LittleEndian>()? as usize;
        let num_deleted = reader.read_u32::<LittleEndian>()? as usize;
        anyhow::ensure!(num_deleted <= num_bits);

        // Compute the number of blocks in the bitset.
        let num_blocks = num_bits.next_multiple_of(64) / 64;

        // As with `Self::load_uuids`, check that the file's lengths match up.
        let mut expected_len = 0;
        expected_len += 1; // version
        expected_len += 4; // count
        expected_len += 4; // num_deleted
        expected_len += num_blocks * 8; // bitset blocks
        anyhow::ensure!(expected_len == file_len);

        let mut deleted_buf = vec![0u64; num_blocks];
        reader.read_u64_into::<LittleEndian>(&mut deleted_buf)?;

        // Unfortunately, we have to do a copy here to change the `u64`s we read via
        // `byteorder` to `usize`s. The deleted bitset is a tiny piece of the
        // index, so we'll probably not need to optimize this for a while.
        let mut deleted = BitVec::from_vec(deleted_buf.into_iter().map(|b| b as usize).collect());

        // Trim the last block to our desired length, if needed.
        deleted.resize(num_bits, false);

        // While we're reading the whole file into memory, check that `num_deleted`
        // matches the bitset.
        anyhow::ensure!(deleted.count_ones() == num_deleted);

        Ok(Self {
            deleted,
            num_deleted,
        })
    }
}

/// Static implementation of `IdTracker` based on loading the index files
/// produced by `MemoryIdTracker`.
///
/// This implementation currently loads the full indexes into memory, but we
/// could eventually change this to use `mmap(2)` to lazily load pieces as
/// necessary.
pub struct StaticIdTracker {
    count: usize,
    uuid_buf: Vec<u8>,
    csf_map: CsfMap,
    deleted: DeletedBitset,
}

impl StaticIdTracker {
    pub fn load_from_path(
        uuid_path: PathBuf,
        deleted_bitset: DeletedBitset,
    ) -> anyhow::Result<Self> {
        let uuid_file = File::open(uuid_path)?;
        StaticIdTracker::load(
            (
                uuid_file.metadata()?.len() as usize,
                BufReader::new(uuid_file),
            ),
            deleted_bitset,
        )
    }

    pub fn load(
        uuid_file: (usize, impl Read),
        deleted_bitset: DeletedBitset,
    ) -> anyhow::Result<Self> {
        let (count, uuid_buf, csf_map) = Self::load_uuids(uuid_file.0, uuid_file.1)?;
        anyhow::ensure!(count == deleted_bitset.len());
        Ok(Self {
            count,
            uuid_buf,
            csf_map,
            deleted: deleted_bitset,
        })
    }

    fn load_uuids(
        file_len: usize,
        mut reader: impl Read,
    ) -> anyhow::Result<(usize, Vec<u8>, CsfMap)> {
        anyhow::ensure!(reader.read_u8()? == UUID_TABLE_VERSION);
        let count = reader.read_u32::<LittleEndian>()? as usize;
        let index_len = reader.read_u32::<LittleEndian>()? as usize;

        // Even though the file is self-describing, check that our lengths match up to
        // be defensive against truncations.
        let mut expected_len = 0;
        expected_len += 1; // version
        expected_len += 4; // count
        expected_len += 4; // index_len
        expected_len += 16 * count; // UUIDs
        expected_len += index_len; // index
        anyhow::ensure!(expected_len == file_len);

        let mut uuid_buf = vec![0u8; 16 * count];
        reader.read_exact(&mut uuid_buf)?;

        let csf_map = CsfMap::read(&mut reader)?;

        Ok((count, uuid_buf, csf_map))
    }
}

impl StaticIdTracker {
    fn get_uuid(&self, offset: usize) -> Option<Uuid> {
        if offset >= self.count {
            return None;
        }
        let buf = self.slice_from_offset(offset);
        Some(Uuid::from_slice(buf).unwrap())
    }

    // You must verify that the offset is valid (<= count-16 at least) before
    // calling this method or it will panic.
    fn slice_from_offset(&self, offset: usize) -> &[u8; 16] {
        self.uuid_buf[offset * 16..(offset + 1) * 16]
            .try_into()
            .unwrap()
    }
}

impl IdTracker for StaticIdTracker {
    fn internal_version(&self, internal_id: PointOffsetType) -> Option<SeqNumberType> {
        self.external_id(internal_id).map(|_| OP_NUM)
    }

    fn set_internal_version(
        &mut self,
        _internal_id: PointOffsetType,
        _version: SeqNumberType,
    ) -> OperationResult<()> {
        panic!("set_internal_version() unsupported");
    }

    fn internal_id(&self, external_id: PointIdType) -> Option<PointOffsetType> {
        let PointIdType::Uuid(uuid) = external_id else {
            panic!("Invalid external ID: {external_id}");
        };
        let offset = self.csf_map.get(uuid.as_bytes());
        let returned_uuid = self.get_uuid(offset as usize)?;

        // NB: `offset` could be completely bogus if `uuid` isn't in the map. Check that
        // it points to our desired UUID before returning `offset`.
        if uuid != returned_uuid {
            return None;
        }
        Some(offset as u32)
    }

    fn external_id(&self, internal_id: PointOffsetType) -> Option<PointIdType> {
        self.get_uuid(internal_id as usize)
            .map(ExtendedPointId::Uuid)
    }

    fn set_link(
        &mut self,
        _external_id: PointIdType,
        _internal_id: PointOffsetType,
    ) -> OperationResult<()> {
        panic!("set_link() unsupported")
    }

    fn drop(&mut self, _external_id: PointIdType) -> OperationResult<()> {
        panic!("drop() unsupported")
    }

    fn iter_external(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        panic!("iter_external() unsupported")
    }

    fn iter_internal(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(0..self.count as u32)
    }

    fn iter_from(
        &self,
        _external_id: Option<PointIdType>,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        panic!("iter_from() unsupported")
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(
            (0..self.count)
                .map(|value| value as u32)
                .filter(|internal_id| !self.deleted.is_deleted_point(*internal_id)),
        )
    }

    fn mapping_flusher(&self) -> Flusher {
        panic!("mapping_flusher() unsupported")
    }

    fn versions_flusher(&self) -> Flusher {
        panic!("versions_flusher() unsupported")
    }

    fn total_point_count(&self) -> usize {
        self.count
    }

    fn deleted_point_count(&self) -> usize {
        self.deleted.num_deleted()
    }

    fn deleted_point_bitslice(&self) -> &BitSlice {
        self.deleted.deleted_point_bitslice()
    }

    fn is_deleted_point(&self, internal_id: PointOffsetType) -> bool {
        self.deleted.is_deleted_point(internal_id)
    }
}

#[cfg(test)]
mod tests {
    use bitvec::vec::BitVec;
    use must_let::must_let;
    use proptest::prelude::*;
    use qdrant_segment::{
        id_tracker::IdTracker,
        types::{
            ExtendedPointId,
            PointIdType,
        },
    };
    use uuid::Uuid;

    use crate::id_tracker::{
        DeletedBitset,
        MemoryIdTracker,
        StaticIdTracker,
        OP_NUM,
    };

    prop_compose! {
        fn uuids_with_some_deleted() (
            all in prop::collection::btree_set(any::<[u8; 16]>(), 0..=16)
                .prop_map(|set| {
                    set
                        .into_iter()
                        .map(|buf| Uuid::from_slice(&buf).unwrap())
                        .collect::<Vec<_>>()
                })
                .prop_shuffle()
            )
            (
                all in Just(all.clone()),
                deleted in proptest::sample::subsequence(all.clone(), all.len())
            ) -> (Vec<Uuid>, Vec<Uuid>) {
            (all, deleted)
        }
    }

    prop_compose! {
        fn uuids_with_some_deleted_and_excluded() (
            (all, deleted) in uuids_with_some_deleted()
        )
        (
            all in Just(all.clone()),
            deleted in Just(deleted),
            excluded in proptest::sample::subsequence(all.clone(), all.len())
        ) -> (Vec<Uuid>, Vec<Uuid>, Vec<Uuid>) {
            (all, deleted, excluded)
        }
    }

    prop_compose! {
        fn uuids_with_overlapping_subsets() (
            all in prop::collection::btree_set(any::<[u8; 16]>(), 0..=16)
                .prop_map(|set| {
                    set
                        .into_iter()
                        .map(|buf| Uuid::from_slice(&buf).unwrap())
                        .collect::<Vec<_>>()
                })
                .prop_shuffle()
        )
        (
            first in proptest::sample::subsequence(all.clone(), all.len()),
            second in proptest::sample::subsequence(all.clone(), all.len()),
        ) -> (Vec<Uuid>, Vec<Uuid>) {
            (first, second)
        }
    }

    prop_compose! {
        fn overlapping_uuid_and_deleted_pairs() (
            (first, second) in uuids_with_overlapping_subsets()
        )
        (
            first in Just(first.clone()),
            first_deleted in proptest::sample::subsequence(first.clone(), first.len()),
            second in Just(second.clone()),
            second_deleted in proptest::sample::subsequence(second.clone(), second.len()),
        ) -> ((Vec<Uuid>, Vec<Uuid>), (Vec<Uuid>, Vec<Uuid>)) {
            ((first, first_deleted), (second, second_deleted))
        }
    }

    prop_compose! {
        fn uuids_with_some_deleted_twice() (
            (all, deleted) in uuids_with_some_deleted()
        )
        (
            all in Just(all),
            deleted in Just(deleted.clone()),
            deleted_twice in proptest::sample::subsequence(deleted.clone(), deleted.len())
        ) -> (Vec<Uuid>, Vec<Uuid>, Vec<Uuid>) {
            (all, deleted, deleted_twice)
        }
    }

    fn memory_id_tracker(all: Vec<Uuid>, deleted: Vec<Uuid>) -> MemoryIdTracker {
        let mut tracker = MemoryIdTracker::new();
        for (i, uuid) in all.iter().enumerate() {
            let internal_id = i as u32;
            let external_id = PointIdType::Uuid(*uuid);
            tracker.set_link(external_id, internal_id).unwrap();
            if deleted.contains(uuid) {
                tracker.drop(external_id).unwrap();
            }
        }
        tracker
    }

    fn static_id_tracker(all: Vec<Uuid>, deleted: Vec<Uuid>) -> (StaticIdTracker, DeletedBitset) {
        let mut deleted_bitset = DeletedBitset::new(all.len());
        let mut tracker = memory_id_tracker(all, vec![]);
        for uuid in deleted {
            let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
            deleted_bitset.delete(internal_id).unwrap();
        }

        let mut uuid_buf = vec![];
        tracker.write_uuids(&mut uuid_buf).unwrap();

        (
            StaticIdTracker::load((uuid_buf.len(), &uuid_buf[..]), deleted_bitset.clone()).unwrap(),
            deleted_bitset,
        )
    }

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn memory_tracker_external_id_returns_none_for_deleted(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all, deleted.clone());
            for uuid in deleted {
                // internal -> external fails but
                // external -> internal succeeds for deleted points in qdrant's implementation.
                let internal_id = tracker.internal_id(PointIdType::Uuid(uuid)).unwrap();
                assert!(tracker.external_id(internal_id).is_none());
            }
        }

        #[test]
        fn memory_tracker_available_point_count_excludes_deleted(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all.clone(), deleted.clone());
            assert_eq!(tracker.available_point_count(), all.len() - deleted.len());
        }

        #[test]
        fn deleted_bitset_num_deleted_returns_deleted_count(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all.clone(), deleted.clone());
            let mut deleted_bitset = DeletedBitset::new(all.len());
            let expected_count = deleted.len();
            for uuid in deleted {
                let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
                deleted_bitset.delete(internal_id).unwrap();
            }

            assert_eq!(expected_count, deleted_bitset.num_deleted());
        }

        #[test]
        fn memory_tracker_iter_external_excludes_deleted_ids(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all.clone(), deleted.clone());

            let mut total = 0;
            for point in tracker.iter_external() {
                must_let!(let PointIdType::Uuid(external_id) = point);
                total += 1;
                assert!(all.contains(&external_id));
                assert!(!deleted.contains(&external_id));
            }
            assert_eq!(total, all.len() - deleted.len());
        }

        #[test]
        fn memory_tracker_iter_internal_includes_deleted_ids(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all.clone(), deleted);

            let mut total = 0;
            for point in tracker.iter_internal() {
                must_let!(let PointIdType::Uuid(external_id) =
                    tracker.external_id_with_deleted(point).unwrap());
                total += 1;
                assert!(all.contains(&external_id));
            }
            assert_eq!(total, all.len());
        }

        #[test]
        fn memory_tracker_iter_ids_excluding_excludes_points(
            (all, deleted, excluded) in uuids_with_some_deleted_and_excluded()
        ) {
            let tracker = memory_id_tracker(all, deleted.clone());
            let mut excluded_bitvec = BitVec::new();
            excluded_bitvec.resize(excluded.len(), false);
            for uuid in &excluded {
                let external_id = PointIdType::Uuid(*uuid);
                let internal_id = tracker.internal_id(external_id).unwrap();
                excluded_bitvec.set(internal_id as usize, true);
            }

            for point in tracker.iter_ids_excluding(&excluded_bitvec) {
                must_let!(let PointIdType::Uuid(uuid) = tracker.external_id(point).unwrap());

                assert!(!deleted.contains(&uuid));
                assert!(!excluded.contains(&uuid));
            }
        }

        #[test]
        fn memory_tracker_allows_writes_with_deleted_points(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let mut deleted_bitset = DeletedBitset::new(all.len());
            let mut tracker = memory_id_tracker(all.clone(), deleted.clone());
            for uuid in deleted.clone() {
                let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
                deleted_bitset.delete(internal_id).unwrap();
            }

            let mut uuid_buf = vec![];
            tracker.write_uuids(&mut uuid_buf).unwrap();

            let tracker = StaticIdTracker::load(
                (uuid_buf.len(), &uuid_buf[..]),
                deleted_bitset,
            ).unwrap();

            for uuid in all {
                let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
                assert_eq!(deleted.contains(&uuid), tracker.is_deleted_point(internal_id));
            }
        }

        #[test]
        fn static_tracker_iter_ids_excludes_removed_points(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let (tracker, _) = static_id_tracker(all, deleted.clone());

            for point in tracker.iter_ids() {
                must_let!(let PointIdType::Uuid(uuid) = tracker.external_id(point).unwrap());

                assert!(!deleted.contains(&uuid))
            }
        }

        #[test]
        fn memory_tracker_iter_ids_excludes_removed_points(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = memory_id_tracker(all, deleted.clone());

            for point in tracker.iter_ids() {
                must_let!(let PointIdType::Uuid(uuid) = tracker.external_id(point).unwrap());

                assert!(!deleted.contains(&uuid))
            }
        }

        #[test]
        fn memory_tracker_iter_ids_throws_on_multiple_deletes(
            (all, deleted, deleted_twice) in uuids_with_some_deleted_twice()
        ) {
            let mut tracker = memory_id_tracker(all, deleted);

            for point in deleted_twice {
                assert!(tracker.drop(PointIdType::Uuid(point)).is_err());
            }
        }

        #[test]
        fn memory_tracker_is_deleted_point((all, deleted) in uuids_with_some_deleted()) {
            let tracker = memory_id_tracker(all.clone(), deleted.clone());

            for uuid in all {
                let external_id = PointIdType::Uuid(uuid);
                let internal_id = tracker.internal_id(external_id).unwrap();
                assert_eq!(tracker.is_deleted_point(internal_id), deleted.contains(&uuid));
            }
        }

        #[test]
        fn memory_tracker_counts_deleted_points((all, deleted) in uuids_with_some_deleted()) {
            let expected = deleted.len();
            let tracker = memory_id_tracker(all, deleted);

            assert_eq!(tracker.deleted_point_count(), expected);
        }

        #[test]
        fn memory_tracker_supports_delete_point((all, deleted) in uuids_with_some_deleted()) {
            let mut tracker = MemoryIdTracker::new();
            for (i, uuid) in all.iter().enumerate() {
                let internal_id = i as u32;
                let external_id = PointIdType::Uuid(*uuid);
                tracker.set_link(external_id, internal_id).unwrap();
                if deleted.contains(uuid) {
                    tracker.drop(external_id).unwrap();
                }
            }
        }

        #[test]
        fn test_deleted_bitset((all, deleted) in uuids_with_some_deleted()) {
            let mut memory_ids = MemoryIdTracker::new();
            let mut deleted_bitset = DeletedBitset::new(all.len());
            for (i, uuid) in all.iter().enumerate() {
                let internal_id = i as u32;
                memory_ids.set_link(PointIdType::Uuid(*uuid), internal_id).unwrap();
                if deleted.contains(uuid) {
                    deleted_bitset.delete(internal_id).unwrap();
                }
            }

            let mut uuid_buf = vec![];
            memory_ids.write_uuids(&mut uuid_buf).unwrap();

            let mut deleted_buf = vec![];
            deleted_bitset.write(&mut deleted_buf).unwrap();

            let deleted_bitset = DeletedBitset::load(deleted_buf.len(), &deleted_buf[..]).unwrap();
            let static_ids = StaticIdTracker::load(
                (uuid_buf.len(), &uuid_buf[..]),
                deleted_bitset,
            ).unwrap();

            for uuid in all.iter() {
                let external_id = PointIdType::Uuid(*uuid);
                let is_deleted =
                    static_ids.is_deleted_point(static_ids.internal_id(external_id).unwrap());
                assert_eq!(is_deleted, deleted.contains(uuid));
            }
        }

        #[test]
        fn test_id_tracker(
            uuids in prop::collection::btree_set(any::<[u8; 16]>(), 0..=16)
                .prop_map(|set| {
                    set
                        .into_iter()
                        .map(|buf| Uuid::from_slice(&buf).unwrap())
                        .collect::<Vec<_>>()
                })
                .prop_shuffle()
        ) {
            let mut memory_ids = MemoryIdTracker::new();
            for (i, uuid) in uuids.iter().enumerate() {
                memory_ids.set_link(PointIdType::Uuid(*uuid), i as u32).unwrap();
            }

            let mut uuid_buf = vec![];
            memory_ids.write_uuids(&mut uuid_buf).unwrap();

            let mut deleted_bitset = DeletedBitset::new(uuids.len());
            let mut deleted_buf = vec![];
            deleted_bitset.write(&mut deleted_buf).unwrap();


            let deleted_bitset = DeletedBitset::load(deleted_buf.len(), &deleted_buf[..]).unwrap();
            let static_ids = StaticIdTracker::load(
                (uuid_buf.len(), &uuid_buf[..]),
                deleted_bitset,
            ).unwrap();

            let ids_impls: [&dyn IdTracker; 2] = [&memory_ids, &static_ids];
            for ids in ids_impls {
                for (i, uuid) in uuids.iter().enumerate() {
                    let offset = i as u32;
                    let uuid = PointIdType::Uuid(*uuid);

                    assert_eq!(ids.internal_version(offset), Some(OP_NUM));
                    assert_eq!(ids.internal_id(uuid), Some(offset));
                    assert_eq!(ids.external_id(offset), Some(uuid));
                }

                // Try a few offsets out of range.
                for i in 0..8 {
                    let offset = uuids.len() as u32 + 7 * i;
                    assert_eq!(ids.internal_version(offset), None);
                    assert_eq!(ids.external_id(offset), None);
                }
                // Try a nonexistent UUID.
                let uuid = uuids.iter().map(|uuid| uuid.as_u128()).max().unwrap_or(0) + 1;
                assert_eq!(ids.internal_id(PointIdType::Uuid(Uuid::from_u128(uuid))), None);
            }
        }
    }
}
