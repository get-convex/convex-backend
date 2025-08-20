mod backfill_state;
mod index_config;
mod index_state;
mod indexed_fields;

pub use self::{
    backfill_state::{
        DatabaseIndexBackfillState,
        SerializedDatabaseIndexBackfillState,
    },
    index_config::{
        DatabaseIndexSpec,
        SerializedDatabaseIndexSpec,
    },
    index_state::{
        DatabaseIndexState,
        SerializedDatabaseIndexState,
    },
    indexed_fields::IndexedFields,
};

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use value::{
        obj,
        ConvexObject,
    };

    use super::*;

    #[test]
    fn test_backfilled_metadata_is_deserialized_as_backfilled() -> anyhow::Result<()> {
        let object: ConvexObject = obj!("type" => "Backfilled2")?;
        let index_state: DatabaseIndexState = object.try_into()?;
        assert_matches!(
            index_state,
            DatabaseIndexState::Backfilled { staged: false }
        );
        Ok(())
    }

    #[test]
    fn test_backfilled_metadata_is_serialized_as_backfilled() -> anyhow::Result<()> {
        let index_state = DatabaseIndexState::Backfilled { staged: true };
        let object: ConvexObject = index_state.try_into()?;
        let index_state: DatabaseIndexState = object.try_into()?;
        assert_matches!(index_state, DatabaseIndexState::Backfilled { staged: true });
        Ok(())
    }
}
