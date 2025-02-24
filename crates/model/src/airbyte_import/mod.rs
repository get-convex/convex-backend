use std::{
    collections::BTreeSet,
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    runtime::Runtime,
    types::{
        IndexDescriptor,
        IndexName,
    },
};
use database::{
    IndexModel,
    Transaction,
};
use errors::ErrorMetadata;
use value::{
    TableName,
    TableNamespace,
};

pub static AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR: LazyLock<IndexDescriptor> =
    LazyLock::new(|| IndexDescriptor::new("_by_airbyte_primary_key").unwrap());

pub struct AirbyteImportModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> AirbyteImportModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn primary_key_indexes_ready(
        &mut self,
        indexes: &BTreeSet<TableName>,
    ) -> anyhow::Result<bool> {
        let index_metadata = indexes
            .iter()
            .map(|table_name| {
                let index_name = IndexName::new_reserved(
                    table_name.clone(),
                    AIRBYTE_PRIMARY_KEY_INDEX_DESCRIPTOR.clone(),
                )?;
                // We really just want pending indexes here, but since it's convenient, we're
                // also verifying that all requested tables have the expected
                // index using enabled_index_metadata.
                let mut model = IndexModel::new(self.tx);
                let index_metadata = model
                    .pending_index_metadata(TableNamespace::by_component_TODO(), &index_name)?
                    .or(model
                        .enabled_index_metadata(TableNamespace::by_component_TODO(), &index_name)?)
                    .context(ErrorMetadata::bad_request(
                        "MissingIndex",
                        format!("Missing index: {index_name}"),
                    ))?;
                Ok(index_metadata)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        let are_all_indexes_ready = index_metadata
            .iter()
            .all(|metadata| !metadata.config.is_backfilling());
        Ok(are_all_indexes_ready)
    }
}
