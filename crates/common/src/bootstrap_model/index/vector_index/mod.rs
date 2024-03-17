mod backfill_state;
mod dimensions;
mod index_config;
mod index_snapshot;
mod index_state;
mod segment;

pub use self::{
    backfill_state::VectorIndexBackfillState,
    dimensions::{
        VectorDimensions,
        MAX_VECTOR_DIMENSIONS,
        MIN_VECTOR_DIMENSIONS,
    },
    index_config::{
        DeveloperVectorIndexConfig,
        SerializedDeveloperVectorIndexConfig,
    },
    index_snapshot::{
        VectorIndexSnapshot,
        VectorIndexSnapshotData,
    },
    index_state::{
        SerializedVectorIndexState,
        VectorIndexState,
    },
    segment::FragmentedVectorSegment,
};

#[cfg(test)]
mod tests {
    use cmd_util::env::env_config;
    use must_let::must_let;
    use proptest::prelude::*;
    use sync_types::{
        testing::assert_roundtrips,
        Timestamp,
    };
    use value::{
        assert_obj,
        ConvexValue,
    };

    use super::*;
    use crate::types::ObjectKey;

    fn serialized_index_state_name_having_data() -> impl Strategy<Value = String> {
        prop::string::string_regex("backfilled|snapshotted").unwrap()
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 64 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, .. ProptestConfig::default() })]

        #[test]
        fn test_developer_vector_index_config_roundtrips(v in any::<DeveloperVectorIndexConfig>()) {
            assert_roundtrips::<
                DeveloperVectorIndexConfig,
                pb::searchlight::VectorIndexConfig
            >(v);
        }

        #[test]
        fn from_legacy_resolved_object_fails(
            key in any::<ObjectKey>(),
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "index" => key.to_string(),
                "ts" => ConvexValue::Int64(ts.into()),
                "version" => 0,
            );
            // We don't have an unknown field at the state level, only for data, so we have to let
            // this error.
            assert!(VectorIndexState::try_from(legacy_object).is_err());
        }

        #[test]
        fn missing_data_type_defaults_to_unknown(
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "data" => {"something" => "invalid"},
                "ts" => ConvexValue::Int64(ts.into()),
            );
            let state: VectorIndexState = legacy_object.try_into().unwrap();
            let snapshot = extract_snapshot(serialized_index_state_name, state);

            must_let!(let VectorIndexSnapshotData::Unknown(_) = snapshot.data);
        }

        #[test]
        fn unrecognized_data_type_defaults_to_unknown(
            ts in any::<Timestamp>(),
            serialized_index_state_name in serialized_index_state_name_having_data(),
        ) {
            let legacy_object = assert_obj!(
                "state" => serialized_index_state_name.as_str(),
                "data" => {"data_type" => "invalid"},
                "ts" => ConvexValue::Int64(ts.into()),
            );
            let state: VectorIndexState = legacy_object.try_into().unwrap();
            let snapshot = extract_snapshot(serialized_index_state_name, state);

            must_let!(let VectorIndexSnapshotData::Unknown(_) = snapshot.data);
        }
    }

    fn extract_snapshot(
        expected_index_state: String,
        state: VectorIndexState,
    ) -> VectorIndexSnapshot {
        if expected_index_state == "backfilled" {
            must_let!(let VectorIndexState::Backfilled(snapshot) = state);
            snapshot
        } else {
            must_let!(let VectorIndexState::SnapshottedAt(snapshot) = state);
            snapshot
        }
    }
}
