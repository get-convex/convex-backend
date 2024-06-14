use std::{
    cmp,
    collections::BTreeMap,
};

use common::{
    document::{
        DeveloperDocument,
        ResolvedDocument,
    },
    query::CursorPosition,
    runtime::Runtime,
    types::{
        StableIndexName,
        WriteTimestamp,
    },
    version::Version,
};
use errors::ErrorMetadata;
use indexing::backend_in_memory_indexes::{
    BatchKey,
    RangeRequest,
};
use value::{
    check_user_size,
    ConvexObject,
    DeveloperDocumentId,
    Size,
    TableName,
    TableNamespace,
};

use crate::{
    metrics::{
        log_virtual_table_get,
        log_virtual_table_query,
    },
    query::{
        DeveloperIndexRangeResponse,
        IndexRangeResponse,
    },
    transaction::{
        IndexRangeRequest,
        MAX_PAGE_SIZE,
    },
    unauthorized_error,
    virtual_tables::VirtualTable,
    PatchValue,
    TableModel,
    Transaction,
};

// Low-level model struct that represents a "user facing" data model
// on the database. This view differs from the authoritative system
// state in a few ways:
//
//  1. System tables are only visible for `Identity::Admin` or
//     `Identity::System`.
//  2. We support virtual tables.
//  3. The interface is in `DeveloperDocumentId`s, not `ResolvedDocumentId`.
//  4. We track user size limits for documents, which are more restrictive than
//     the database's limits.
//  5. We support branching on the `convex` NPM package's version.
pub struct UserFacingModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
    namespace: TableNamespace,
}

impl<'a, RT: Runtime> UserFacingModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>, namespace: TableNamespace) -> Self {
        Self { tx, namespace }
    }

    #[cfg(any(test, feature = "testing"))]
    pub fn new_root_for_test(tx: &'a mut Transaction<RT>) -> Self {
        Self {
            tx,
            namespace: TableNamespace::test_user(),
        }
    }

    #[cfg(any(test, feature = "testing"))]
    #[convex_macro::instrument_future]
    pub async fn get(
        &mut self,
        id: DeveloperDocumentId,
        version: Option<Version>,
    ) -> anyhow::Result<Option<DeveloperDocument>> {
        Ok(self
            .get_with_ts(id, version)
            .await?
            .map(|(document, _)| document))
    }

    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn get_with_ts(
        &mut self,
        id: DeveloperDocumentId,
        version: Option<Version>,
    ) -> anyhow::Result<Option<(DeveloperDocument, WriteTimestamp)>> {
        if self.tx.virtual_table_mapping().number_exists(id.table()) {
            log_virtual_table_get();
            let result = VirtualTable::new(self.tx).get(id, version).await;
            if let Ok(Some((document, _))) = &result {
                let table_name = self
                    .tx
                    .virtual_table_mapping()
                    .name(*document.id().table())?;
                self.tx.reads.record_read_document(
                    table_name,
                    document.size(),
                    &self.tx.usage_tracker,
                    true,
                )?;
            }
            result
        } else {
            if !self
                .tx
                .table_mapping()
                .namespace(self.namespace)
                .table_number_exists()(*id.table())
            {
                return Ok(None);
            }
            let id_ = id.map_table(
                self.tx
                    .table_mapping()
                    .namespace(self.namespace)
                    .inject_table_id(),
            )?;
            let table_name = self.tx.table_mapping().tablet_name(id_.table().tablet_id)?;
            let result = self.tx.get_inner(id_, table_name).await?;
            Ok(result.map(|(doc, ts)| (doc.to_developer(), ts)))
        }
    }

    /// Creates a new document with given value in the specified table.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn insert(
        &mut self,
        table: TableName,
        value: ConvexObject,
    ) -> anyhow::Result<DeveloperDocumentId> {
        if self.tx.virtual_system_mapping().is_virtual_table(&table) {
            anyhow::bail!(ErrorMetadata::bad_request(
                "ReadOnlyTable",
                format!("{table} is a read-only table"),
            ));
        }

        check_user_size(value.size())?;
        self.tx.retention_validator.fail_if_falling_behind()?;
        let id = self.tx.id_generator.generate(&table);

        let creation_time = self.tx.next_creation_time.increment()?;

        if table.is_system() {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidTableName",
                format!("Invalid table name {table} starts with metadata prefix '_'")
            ));
        }

        // Note that the index and document store updates within `self.insert_document`
        // below are fallible, and since the layers above still have access to
        // the `Transaction` in that case (we only have `&mut self` here, not a
        // consuming `self`), we need to make sure we leave the transaction in a
        // consistent state on error.
        //
        // It's okay for us to insert the table write here and fail below: At worse the
        // transaction will contain an insertion for an empty table's `_tables`
        // record. On the other hand, it's not okay for us to succeed an
        // insertion into the index/document store and then fail to insert the
        // table metadata. If the user then subsequently commits that transaction,
        // they'll have a record that points to a nonexistent table.
        TableModel::new(self.tx)
            .insert_table_metadata(self.namespace, &table)
            .await?;
        let document = ResolvedDocument::new(
            id.clone().map_table(
                self.tx
                    .table_mapping()
                    .namespace(self.namespace)
                    .name_to_id_user_input(),
            )?,
            creation_time,
            value,
        )?;
        let document_id = self.tx.insert_document(document).await?;

        Ok(document_id.into())
    }

    /// Merges the existing document with the given object. Will overwrite any
    /// conflicting fields.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn patch(
        &mut self,
        id: DeveloperDocumentId,
        value: PatchValue,
    ) -> anyhow::Result<DeveloperDocument> {
        if self.tx.is_system(*id.table())
            && !(self.tx.identity.is_admin() || self.tx.identity.is_system())
        {
            anyhow::bail!(unauthorized_error("patch"))
        }
        self.tx.retention_validator.fail_if_falling_behind()?;

        let id_ = id.map_table(
            self.tx
                .table_mapping()
                .namespace(self.namespace)
                .inject_table_id(),
        )?;

        let new_document = self.tx.patch_inner(id_, value).await?;

        // Check the size of the patched document.
        if !self.tx.is_system(*id.table()) {
            check_user_size(new_document.size())?;
        }

        let developer_document = new_document.to_developer();
        Ok(developer_document)
    }

    /// Replace the document with the given value.
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn replace(
        &mut self,
        id: DeveloperDocumentId,
        value: ConvexObject,
    ) -> anyhow::Result<DeveloperDocument> {
        if self.tx.is_system(*id.table())
            && !(self.tx.identity.is_admin() || self.tx.identity.is_system())
        {
            anyhow::bail!(unauthorized_error("replace"))
        }
        if !self.tx.is_system(*id.table()) {
            check_user_size(value.size())?;
        }
        self.tx.retention_validator.fail_if_falling_behind()?;
        let id_ = id.map_table(
            self.tx
                .table_mapping()
                .namespace(self.namespace)
                .inject_table_id(),
        )?;

        let new_document = self.tx.replace_inner(id_, value).await?;
        let developer_document = new_document.to_developer();
        Ok(developer_document)
    }

    /// Delete the document at the given path -- called from user facing APIs
    /// (e.g. syscalls)
    #[minitrace::trace]
    #[convex_macro::instrument_future]
    pub async fn delete(&mut self, id: DeveloperDocumentId) -> anyhow::Result<DeveloperDocument> {
        if self.tx.is_system(*id.table())
            && !(self.tx.identity.is_admin() || self.tx.identity.is_system())
        {
            anyhow::bail!(unauthorized_error("delete"))
        }
        self.tx.retention_validator.fail_if_falling_behind()?;

        let id_ = id.map_table(
            &self
                .tx
                .table_mapping()
                .namespace(self.namespace)
                .inject_table_id(),
        )?;
        let document = self.tx.delete_inner(id_).await?;
        Ok(document.to_developer())
    }

    pub fn record_read_document(
        &mut self,
        document: &DeveloperDocument,
        table_name: &TableName,
    ) -> anyhow::Result<()> {
        let is_virtual_table = self
            .tx
            .virtual_system_mapping()
            .is_virtual_table(table_name);
        self.tx.reads.record_read_document(
            table_name.clone(),
            document.size(),
            &self.tx.usage_tracker,
            is_virtual_table,
        )
    }
}

fn start_index_range<RT: Runtime>(
    tx: &mut Transaction<RT>,
    request: IndexRangeRequest,
) -> anyhow::Result<Result<DeveloperIndexRangeResponse, RangeRequest>> {
    if request.interval.is_empty() {
        return Ok(Ok(DeveloperIndexRangeResponse {
            page: vec![],
            cursor: CursorPosition::End,
        }));
    }

    let max_rows = cmp::min(request.max_rows, MAX_PAGE_SIZE);

    match request.stable_index_name {
        StableIndexName::Physical(tablet_index_name) => {
            let index_name = tablet_index_name
                .clone()
                .map_table(&tx.table_mapping().tablet_to_name())?;
            Ok(Err(RangeRequest {
                index_name: tablet_index_name.clone(),
                printable_index_name: index_name,
                interval: request.interval.clone(),
                order: request.order,
                max_size: max_rows,
            }))
        },
        StableIndexName::Virtual(index_name, tablet_index_name) => {
            log_virtual_table_query();
            Ok(Err(RangeRequest {
                index_name: tablet_index_name.clone(),
                printable_index_name: index_name.clone(),
                interval: request.interval.clone(),
                order: request.order,
                max_size: max_rows,
            }))
        },
        StableIndexName::Missing => Ok(Ok(DeveloperIndexRangeResponse {
            page: vec![],
            cursor: CursorPosition::End,
        })),
    }
}

/// NOTE: returns a page of results. Callers must call record_read_document +
/// record_indexed_directly for all documents returned from the index stream.
#[minitrace::trace]
#[convex_macro::instrument_future]
pub async fn index_range_batch<RT: Runtime>(
    tx: &mut Transaction<RT>,
    requests: BTreeMap<BatchKey, IndexRangeRequest>,
) -> BTreeMap<BatchKey, anyhow::Result<DeveloperIndexRangeResponse>> {
    let batch_size = requests.len();
    let mut results = BTreeMap::new();
    let mut fetch_requests = BTreeMap::new();
    let mut virtual_table_versions = BTreeMap::new();
    for (batch_key, request) in requests {
        if matches!(request.stable_index_name, StableIndexName::Virtual(_, _)) {
            virtual_table_versions.insert(batch_key, request.version.clone());
        }
        match start_index_range(tx, request) {
            Err(e) => {
                results.insert(batch_key, Err(e));
            },
            Ok(Ok(result)) => {
                results.insert(batch_key, Ok(result));
            },
            Ok(Err(request)) => {
                fetch_requests.insert(batch_key, request);
            },
        }
    }

    let fetch_results = tx.index.range_batch(&mut tx.reads, fetch_requests).await;

    for (batch_key, fetch_result) in fetch_results {
        let virtual_table_version = virtual_table_versions.get(&batch_key).cloned();
        let result = fetch_result.and_then(|IndexRangeResponse { page, cursor }| {
            let developer_results = match virtual_table_version {
                Some(version) => page
                    .into_iter()
                    .map(|(key, doc, ts)| {
                        let doc = VirtualTable::new(tx)
                            .map_system_doc_to_virtual_doc(doc, version.clone())?;
                        anyhow::Ok((key, doc, ts))
                    })
                    .try_collect()?,
                None => page
                    .into_iter()
                    .map(|(key, doc, ts)| (key, doc.to_developer(), ts))
                    .collect(),
            };
            anyhow::Ok(DeveloperIndexRangeResponse {
                page: developer_results,
                cursor,
            })
        });
        results.insert(batch_key, result);
    }
    assert_eq!(results.len(), batch_size);
    results
}
