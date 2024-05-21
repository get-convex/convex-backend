use std::{
    collections::BTreeMap,
    str::FromStr,
    sync::{
        Arc,
        LazyLock,
    },
};

use anyhow::Context;
use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
        Query,
    },
    runtime::Runtime,
    types::{
        GenericIndexName,
        IndexName,
    },
};
use database::{
    defaults::system_index,
    query::{
        resolved_query_batch_next,
        TableFilter,
    },
    unauthorized_error,
    ResolvedQuery,
    SystemMetadataModel,
    TableModel,
    Transaction,
    VirtualSystemDocMapper,
};
use errors::ErrorMetadata;
use keybroker::Identity;
use maplit::btreemap;
use pb::storage::{
    file_storage_id::StorageIdType as FileStorageIdTypeProto,
    FileStorageId as FileStorageIdProto,
};
use value::{
    id_v6::DeveloperDocumentId,
    ConvexValue,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use self::virtual_table::FileStorageDocMapper;
use crate::{
    file_storage::types::{
        FileStorageEntry,
        StorageUuid,
    },
    SystemIndex,
    SystemTable,
};

pub mod types;
pub mod virtual_table;

pub type BatchKey = usize;

pub static FILE_STORAGE_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_file_storage"
        .parse()
        .expect("invalid built-in file storage table")
});
pub static FILE_STORAGE_VIRTUAL_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_storage"
        .parse()
        .expect("_storage is not a valid virtual table name")
});

pub static FILE_STORAGE_INDEX_BY_ID: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_id(FILE_STORAGE_TABLE.clone()));
pub static FILE_STORAGE_INDEX_BY_CREATION_TIME: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_creation_time(FILE_STORAGE_TABLE.clone()));
pub static FILE_STORAGE_VIRTUAL_INDEX_BY_ID: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_id(FILE_STORAGE_VIRTUAL_TABLE.clone()));
static FILE_STORAGE_VIRTUAL_INDEX_BY_CREATION_TIME: LazyLock<IndexName> =
    LazyLock::new(|| GenericIndexName::by_creation_time(FILE_STORAGE_VIRTUAL_TABLE.clone()));

static FILE_STORAGE_ID_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "storageId".parse().expect("invalid storageId field"));
pub static FILE_STORAGE_ID_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&FILE_STORAGE_TABLE, "by_storage_id"));

pub struct FileStorageTable;
impl SystemTable for FileStorageTable {
    fn table_name(&self) -> &'static TableName {
        &FILE_STORAGE_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: FILE_STORAGE_ID_INDEX.clone(),
            fields: vec![FILE_STORAGE_ID_FIELD.clone()].try_into().unwrap(),
        }]
    }

    fn virtual_table(
        &self,
    ) -> Option<(
        &'static TableName,
        BTreeMap<IndexName, IndexName>,
        Arc<dyn VirtualSystemDocMapper>,
    )> {
        Some((
            &FILE_STORAGE_VIRTUAL_TABLE,
            btreemap! {
                FILE_STORAGE_VIRTUAL_INDEX_BY_CREATION_TIME.clone() =>
                    FILE_STORAGE_INDEX_BY_CREATION_TIME.clone(),
                FILE_STORAGE_VIRTUAL_INDEX_BY_ID.clone() =>
                    FILE_STORAGE_INDEX_BY_ID.clone(),
            },
            Arc::new(FileStorageDocMapper),
        ))
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<FileStorageEntry>::try_from(document).map(|_| ())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, derive_more::Display)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub enum FileStorageId {
    LegacyStorageId(StorageUuid),
    DocumentId(DeveloperDocumentId),
}

impl FromStr for FileStorageId {
    type Err = anyhow::Error;

    fn from_str(storage_id: &str) -> Result<Self, Self::Err> {
        let decoded_id = DeveloperDocumentId::decode(storage_id);
        match decoded_id {
            Ok(decoded_id) => Ok(FileStorageId::DocumentId(decoded_id)),
            Err(_) => Ok(FileStorageId::LegacyStorageId(storage_id.parse().context(
                ErrorMetadata::bad_request(
                    "InvalidStorageId",
                    format!(
                        "Invalid storage ID: \"{storage_id}\". Storage ID should be an
                    Id of '_storage' table, or a UUID string.",
                    ),
                ),
            )?)),
        }
    }
}

impl TryFrom<FileStorageIdProto> for FileStorageId {
    type Error = anyhow::Error;

    fn try_from(storage_id: FileStorageIdProto) -> Result<Self, Self::Error> {
        let storage_id = match storage_id.storage_id_type {
            Some(FileStorageIdTypeProto::LegacyStorageId(storage_id)) => {
                FileStorageId::LegacyStorageId(storage_id.parse()?)
            },
            Some(FileStorageIdTypeProto::DocumentId(storage_id)) => {
                FileStorageId::DocumentId(DeveloperDocumentId::decode(storage_id.as_str())?)
            },
            None => anyhow::bail!("Missing `storage_id_type` field"),
        };
        Ok(storage_id)
    }
}

impl From<FileStorageId> for FileStorageIdProto {
    fn from(storage_id: FileStorageId) -> Self {
        let storage_id_type = match storage_id {
            FileStorageId::LegacyStorageId(storage_id) => {
                FileStorageIdTypeProto::LegacyStorageId(storage_id.to_string())
            },
            FileStorageId::DocumentId(storage_id) => {
                FileStorageIdTypeProto::DocumentId(storage_id.encode())
            },
        };
        FileStorageIdProto {
            storage_id_type: Some(storage_id_type),
        }
    }
}

pub struct FileStorageModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> FileStorageModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn store_file(
        &mut self,
        entry: FileStorageEntry,
    ) -> anyhow::Result<ResolvedDocumentId> {
        // Call insert_metadata rather than insert because we already
        // did access check on `identity` rather than `self.identity`
        SystemMetadataModel::new(self.tx, TableNamespace::Global)
            .insert_metadata(&FILE_STORAGE_TABLE, entry.try_into()?)
            .await
    }

    pub async fn get_file(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<Option<ParsedDocument<FileStorageEntry>>> {
        self.get_file_batch(btreemap! {0 => storage_id})
            .await
            .remove(&0)
            .context("batch_key missing")?
    }

    pub async fn get_file_batch(
        &mut self,
        storage_ids: BTreeMap<BatchKey, FileStorageId>,
    ) -> BTreeMap<BatchKey, anyhow::Result<Option<ParsedDocument<FileStorageEntry>>>> {
        let batch_size = storage_ids.len();
        let mut results = BTreeMap::new();
        let mut queries = BTreeMap::new();
        for (batch_key, storage_id) in storage_ids {
            match self.query_for_storage_id(storage_id) {
                Ok(query) => {
                    queries.insert(batch_key, query);
                },
                Err(e) => {
                    results.insert(batch_key, Err(e));
                },
            }
        }
        let queries_to_fetch = queries
            .iter_mut()
            .map(|(batch_key, query)| (*batch_key, (query, Some(1))))
            .collect();
        for (batch_key, fetch_result) in resolved_query_batch_next(queries_to_fetch, self.tx).await
        {
            let parsed_result = match fetch_result {
                Err(e) => Err(e),
                Ok(None) => Ok(None),
                Ok(Some((doc, _))) => ParsedDocument::try_from(doc).map(Some),
            };
            results.insert(batch_key, parsed_result);
        }
        assert_eq!(results.len(), batch_size);
        results
    }

    fn query_for_storage_id(
        &mut self,
        storage_id: FileStorageId,
    ) -> anyhow::Result<ResolvedQuery<RT>> {
        let index_query = match storage_id {
            FileStorageId::LegacyStorageId(storage_id) => Query::index_range(IndexRange {
                index_name: FILE_STORAGE_ID_INDEX.clone(),
                range: vec![IndexRangeExpression::Eq(
                    FILE_STORAGE_ID_FIELD.clone(),
                    ConvexValue::try_from(storage_id)?.into(),
                )],
                order: Order::Asc,
            }),
            FileStorageId::DocumentId(document_id) => {
                let table_name = self
                    .tx
                    .resolve_idv6(document_id, TableFilter::ExcludePrivateSystemTables)
                    .context(ErrorMetadata::bad_request(
                        "InvalidArgument",
                        format!(
                            "Invalid storage ID. Storage ID cannot be an ID on any table other \
                             than '_storage'.",
                        ),
                    ))?;
                anyhow::ensure!(
                    table_name == *FILE_STORAGE_VIRTUAL_TABLE,
                    ErrorMetadata::bad_request(
                        "InvalidArgument",
                        format!(
                            "Invalid storage ID. Storage ID cannot be an ID on any table other \
                             than '_storage'.",
                        ),
                    )
                );
                let table_mapping = self.tx.table_mapping().clone();
                let document_id = self
                    .tx
                    .virtual_system_mapping()
                    .virtual_id_v6_to_system_resolved_doc_id(
                        &document_id,
                        &table_mapping,
                        &self.tx.virtual_table_mapping().clone(),
                    )?;
                Query::get(FILE_STORAGE_TABLE.clone(), document_id.into())
            },
        };
        ResolvedQuery::new(self.tx, TableNamespace::Global, index_query)
    }

    pub async fn delete_file(
        &mut self,
        storage_id: FileStorageId,
        identity: Identity,
    ) -> anyhow::Result<Option<FileStorageEntry>> {
        // We only expect this function to be called by the framework as part
        // of a storage syscall. We require passing in a system identity to confirm
        // that the caller isn't letting a user call this directly
        if !identity.is_system() {
            anyhow::bail!(unauthorized_error("delete_file"))
        }
        let Some(entry) = self.get_file(storage_id).await? else {
            return Ok(None);
        };
        let document_id = entry.id();
        SystemMetadataModel::new(self.tx, TableNamespace::Global)
            .delete(document_id)
            .await?;
        Ok(Some(entry.into_value()))
    }

    pub async fn get_total_storage_count(&mut self) -> anyhow::Result<u64> {
        TableModel::new(self.tx)
            .count(&FILE_STORAGE_TABLE.clone())
            .await
    }

    pub async fn get_total_storage_size(&mut self) -> anyhow::Result<u64> {
        if !self.tx.identity().is_system() {
            anyhow::bail!(unauthorized_error("get_total_storage_size"))
        }

        let query = Query::full_table_scan(FILE_STORAGE_TABLE.to_owned(), Order::Asc);
        let mut query_stream = ResolvedQuery::new(self.tx, TableNamespace::Global, query)?;
        let mut total_size = 0;
        while let Some(storage_document) = query_stream.next(self.tx, None).await? {
            let storage_entry: ParsedDocument<FileStorageEntry> = storage_document.try_into()?;
            total_size += storage_entry.size as u64;
        }
        Ok(total_size)
    }
}

#[cfg(test)]
mod tests {
    use pb::storage::FileStorageId as FileStorageIdProto;
    use proptest::prelude::*;
    use sync_types::testing::assert_roundtrips;

    use super::FileStorageId;

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_function_result_proto_roundtrips(left in any::<FileStorageId>()) {
            assert_roundtrips::<FileStorageId, FileStorageIdProto>(left);
        }
    }
}
