use std::{
    io::Write,
    iter,
    ops::RangeFrom,
};

use bitvec::slice::BitSlice;
use common::{
    deleted_bitset::DeletedBitset,
    id_tracker::{
        MemoryIdTracker,
        StaticIdTracker,
    },
};
use qdrant_common::types::PointOffsetType;
use qdrant_segment::{
    common::{
        operation_error::{
            OperationError,
            OperationResult,
        },
        Flusher,
    },
    id_tracker::IdTracker,
    types::{
        ExtendedPointId,
        PointIdType,
        SeqNumberType,
    },
};
use uuid::Uuid;

/// Qdrant has a notion of "operation number" that it uses for tracking
/// mutations. Since we only use it as a static index, require that all points'
/// operation numbers are always 1.
pub const OP_NUM: SeqNumberType = 1;

/// Restricted implementation of `IdTracker` that assumes...
///
/// 1. All operation numbers are OP_NUM.
/// 2. The application only uses UUID point IDs.
/// 3. The set of offsets used is dense (i.e. `0..self.len()`).
pub struct VectorMemoryIdTracker {
    memory_id_tracker: MemoryIdTracker,

    // We don't actually support deletes here but keep this empty bitset around
    // to use in `deleted_point_bitslice`.
    deleted: DeletedBitset,
}

impl VectorMemoryIdTracker {
    pub fn new() -> Self {
        Self {
            memory_id_tracker: MemoryIdTracker::default(),
            deleted: DeletedBitset::new(0),
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
        self.memory_id_tracker
            .convex_id(internal_id)
            .map(|bytes| PointIdType::Uuid(Uuid::from_slice(&bytes).unwrap()))
    }
}

impl IdTracker for VectorMemoryIdTracker {
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
        self.memory_id_tracker
            .index_id(*uuid.as_bytes())
            .map(|ix| ix as PointOffsetType)
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
        self.memory_id_tracker.insert(internal_id, *uuid.as_bytes());
        self.deleted.resize(internal_id as usize + 1);
        Ok(())
    }

    fn drop(&mut self, external_id: PointIdType) -> OperationResult<()> {
        let Some(internal_id) = self.internal_id(external_id) else {
            panic!("Unrecognized external id: {external_id}");
        };
        self.deleted
            .delete(internal_id)
            .map_err(|e| OperationError::InconsistentStorage {
                description: e.to_string(),
            })?;
        Ok(())
    }

    fn iter_external(&self) -> Box<dyn Iterator<Item = PointIdType> + '_> {
        Box::new(
            self.memory_id_tracker
                .by_index_id
                .iter()
                .filter(|(key, _)| !self.is_deleted_point(**key))
                .map(|(_, uuid)| PointIdType::Uuid(Uuid::from_slice(uuid).unwrap())),
        )
    }

    fn iter_internal(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(self.memory_id_tracker.by_index_id.keys().copied())
    }

    fn iter_from(
        &self,
        external_id: Option<PointIdType>,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        // All `NumId`s sort before all `Uuid`s in qdrant's order, so effectively ignore
        // a `NumId` lower bound.
        let Some(minimum) = self.memory_id_tracker.by_convex_id.keys().next().copied() else {
            return Box::new(iter::empty());
        };
        let lower_bound = external_id
            .and_then(|id| match id {
                ExtendedPointId::NumId(..) => None,
                ExtendedPointId::Uuid(uuid) => Some(uuid.into_bytes()),
            })
            .unwrap_or(minimum);
        let iter = self
            .memory_id_tracker
            .by_convex_id
            .range::<[u8; 16], RangeFrom<&[u8; 16]>>(&lower_bound..)
            .map(|(k, v)| {
                (
                    PointIdType::Uuid(Uuid::from_slice(k).unwrap()),
                    *v as PointOffsetType,
                )
            });
        Box::new(iter)
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(
            self.memory_id_tracker
                .by_index_id
                .keys()
                .filter(|internal_id| !self.deleted.is_deleted(**internal_id))
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
        self.memory_id_tracker.by_index_id.len()
    }

    fn deleted_point_count(&self) -> usize {
        self.deleted.num_deleted()
    }

    fn deleted_point_bitslice(&self) -> &BitSlice {
        self.deleted.deleted_id_bitslice()
    }

    fn is_deleted_point(&self, internal_id: PointOffsetType) -> bool {
        self.deleted.is_deleted(internal_id)
    }
}

impl VectorMemoryIdTracker {
    pub fn write_uuids(&mut self, out: impl Write) -> anyhow::Result<()> {
        self.memory_id_tracker.write_id_tracker(out)
    }

    pub fn write_deleted_bitset(&mut self, out: impl Write) -> anyhow::Result<()> {
        self.deleted.write(out)
    }
}

pub struct VectorStaticIdTracker {
    pub id_tracker: StaticIdTracker,
    pub deleted_bitset: DeletedBitset,
}

impl VectorStaticIdTracker {
    fn get_uuid(&self, offset: usize) -> Option<Uuid> {
        self.id_tracker
            .get_convex_id(offset)
            .map(|v| Uuid::from_slice(&v).unwrap())
    }
}

impl IdTracker for VectorStaticIdTracker {
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
        self.id_tracker.lookup(*uuid.as_bytes())
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
        Box::new(0..self.id_tracker.count() as u32)
    }

    fn iter_from(
        &self,
        _external_id: Option<PointIdType>,
    ) -> Box<dyn Iterator<Item = (PointIdType, PointOffsetType)> + '_> {
        panic!("iter_from() unsupported")
    }

    fn iter_ids(&self) -> Box<dyn Iterator<Item = PointOffsetType> + '_> {
        Box::new(
            (0..self.id_tracker.count())
                .map(|value| value as u32)
                .filter(|internal_id| !self.deleted_bitset.is_deleted(*internal_id)),
        )
    }

    fn mapping_flusher(&self) -> Flusher {
        panic!("mapping_flusher() unsupported")
    }

    fn versions_flusher(&self) -> Flusher {
        panic!("versions_flusher() unsupported")
    }

    fn total_point_count(&self) -> usize {
        self.id_tracker.count()
    }

    fn deleted_point_count(&self) -> usize {
        self.deleted_bitset.num_deleted()
    }

    fn deleted_point_bitslice(&self) -> &BitSlice {
        self.deleted_bitset.deleted_id_bitslice()
    }

    fn is_deleted_point(&self, internal_id: PointOffsetType) -> bool {
        self.deleted_bitset.is_deleted(internal_id)
    }
}

#[cfg(test)]
mod tests {
    use bitvec::vec::BitVec;
    use common::deleted_bitset::DeletedBitset;
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
        StaticIdTracker,
        VectorMemoryIdTracker,
        VectorStaticIdTracker,
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

    fn memory_id_tracker(all: Vec<Uuid>, deleted: Vec<Uuid>) -> VectorMemoryIdTracker {
        let mut tracker = VectorMemoryIdTracker::new();
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

    fn static_id_tracker(all: Vec<Uuid>, deleted: Vec<Uuid>) -> VectorStaticIdTracker {
        let mut deleted_bitset = DeletedBitset::new(all.len());
        let mut tracker = memory_id_tracker(all, vec![]);
        for uuid in deleted {
            let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
            deleted_bitset.delete(internal_id).unwrap();
        }

        let mut uuid_buf = vec![];
        tracker.write_uuids(&mut uuid_buf).unwrap();

        let id_tracker = StaticIdTracker::load(uuid_buf.len(), &uuid_buf[..]).unwrap();
        VectorStaticIdTracker {
            id_tracker,
            deleted_bitset,
        }
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

            let id_tracker = StaticIdTracker::load(
                uuid_buf.len(), &uuid_buf[..],
            ).unwrap();
            let tracker = VectorStaticIdTracker {
                id_tracker,
                deleted_bitset,
            };

            for uuid in all {
                let internal_id = tracker.internal_id(ExtendedPointId::Uuid(uuid)).unwrap();
                assert_eq!(deleted.contains(&uuid), tracker.deleted_bitset.is_deleted(internal_id));
            }
        }

        #[test]
        fn static_tracker_iter_ids_excludes_removed_points(
            (all, deleted) in uuids_with_some_deleted()
        ) {
            let tracker = static_id_tracker(all, deleted.clone());

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
            let mut tracker = VectorMemoryIdTracker::new();
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
            let mut memory_ids = VectorMemoryIdTracker::new();
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
            let id_tracker = StaticIdTracker::load(
                uuid_buf.len(), &uuid_buf[..],
            ).unwrap();
            let static_ids = VectorStaticIdTracker {
                id_tracker,
                deleted_bitset,
            };

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
            let mut memory_ids = VectorMemoryIdTracker::new();
            for (i, uuid) in uuids.iter().enumerate() {
                memory_ids.set_link(PointIdType::Uuid(*uuid), i as u32).unwrap();
            }

            let mut uuid_buf = vec![];
            memory_ids.write_uuids(&mut uuid_buf).unwrap();

            let mut deleted_bitset = DeletedBitset::new(uuids.len());
            let mut deleted_buf = vec![];
            deleted_bitset.write(&mut deleted_buf).unwrap();


            let deleted_bitset = DeletedBitset::load(deleted_buf.len(), &deleted_buf[..]).unwrap();
            let id_tracker = StaticIdTracker::load(
                uuid_buf.len(), &uuid_buf[..],
            ).unwrap();
            let static_ids = VectorStaticIdTracker {
                id_tracker,
                deleted_bitset,
            };

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
