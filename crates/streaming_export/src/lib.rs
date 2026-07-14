//! Public streaming-export ("data sync") API built on top of the low-level
//! [`table_iteration::data_sync::DataSyncIterator`]. It resolves a table filter
//! to concrete tablets, drives the iterator one page at a time, converts
//! emitted revisions into name-addressed [`SyncEntry`]s, and (de)serializes the
//! opaque cursor.
//!
//! This is the forward-looking replacement for the `list_snapshot` /
//! `document_deltas` APIs (still on [`Database`]). It lives in its own crate,
//! driving [`Database`] through its public API, so other call sites (e.g. the
//! search flusher, backups) can depend on it directly.
//!
//! See <https://app.notion.com/p/convex-dev/Robust-Streaming-Export-API-36db57ff32ab80c68d97e01c578518d4>

use std::collections::BTreeMap;

use anyhow::Context as _;
use common::{
    components::{
        ComponentId,
        ComponentPath,
    },
    persistence::DocumentLogEntry,
    runtime::Runtime,
    types::{
        IndexId,
        Timestamp,
    },
    version::ClientType,
};
use database::{
    streaming_export_selection::StreamingExportDocument,
    unauthorized_error,
    BootstrapComponentsModel,
    Database,
    IndexModel,
    StreamingExportFilter,
};
use keybroker::Identity;
use pb_data_sync::convex_data_sync as pb_ds;
use prost::Message as _;
use table_iteration::data_sync::{
    DataSyncCursor,
    DataSyncStatus,
};
use usage_tracking::{
    FunctionUsageStats,
    FunctionUsageTracker,
};
use value::{
    DeveloperDocumentId,
    InternalId,
    TableMapping,
    TableName,
    TabletId,
};

/// A single document-level entry emitted by [`data_sync`], addressed by
/// (component, table) name rather than tablet id. Table truncations are
/// reported separately, in [`SyncResult::truncates`].
#[derive(Debug)]
pub enum SyncEntry {
    /// The latest revision of a document as of its `ts`.
    Document {
        ts: Timestamp,
        component: ComponentPath,
        table: TableName,
        document: StreamingExportDocument,
    },
    /// A document was deleted at `ts`.
    Tombstone {
        ts: Timestamp,
        component: ComponentPath,
        table: TableName,
        id: DeveloperDocumentId,
    },
}

/// A table whose contents were replaced wholesale (e.g. by `npx convex
/// import`). Consumers should drop everything previously synced for the table;
/// the [`SyncEntry`]s in the same (and later) pages re-sync it from scratch.
///
/// Truncations logically apply before any [`SyncEntry`]s in the same page.
#[derive(Debug)]
pub struct SyncTruncate {
    pub component: ComponentPath,
    pub table: TableName,
}

/// Integration issuing a data sync, derived from the `Convex-Client` header.
/// A cold start prefixes its `sync_id` with this so `/data/list_active_syncs`
/// can tell integrations apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataSyncClient {
    Fivetran,
    Airbyte,
    Other,
}

impl DataSyncClient {
    /// Prefix prepended to a cold-start `sync_id`. Empty for [`Self::Other`].
    fn sync_id_prefix(self) -> &'static str {
        match self {
            Self::Fivetran => "fivetran-",
            Self::Airbyte => "airbyte-",
            Self::Other => "",
        }
    }
}

impl From<&ClientType> for DataSyncClient {
    fn from(client: &ClientType) -> Self {
        match client {
            ClientType::FivetranExport | ClientType::FivetranImport => Self::Fivetran,
            ClientType::AirbyteExport => Self::Airbyte,
            _ => Self::Other,
        }
    }
}

/// Progress reported while a sync is still [`SyncStatus::InProgress`].
#[derive(Debug)]
pub struct SyncProgress {
    pub num_tables_synced: u64,
    pub total_tables: u64,
    /// The component of the table currently being traversed. An in-progress
    /// sync always has a current table: finishing one either starts the next
    /// or completes the sync ([`SyncStatus::Synced`]).
    pub current_component: ComponentPath,
    /// The table currently being traversed.
    pub current_table: TableName,
    /// Documents emitted so far from the current table's `by_id` traversal.
    pub num_documents_in_current_table: u64,
    /// Documents in the current table at a recent snapshot. `None` while
    /// table summaries are still bootstrapping.
    pub total_documents_in_current_table: Option<u64>,
    /// Documents (including tombstones and re-emitted revisions) emitted over
    /// the sync's lifetime, so this can slightly exceed `total_documents`.
    pub num_documents_synced: u64,
    /// Documents across all target tables at a recent snapshot. `None` while
    /// table summaries are still bootstrapping.
    pub total_documents: Option<u64>,
}

/// The consistency state reported alongside a page.
#[derive(Debug)]
pub enum SyncStatus {
    /// The entries emitted so far represent a consistent snapshot at `ts`.
    Synced {
        ts: Timestamp,
        /// Whether `ts` is behind the latest timestamp — i.e. the snapshot is
        /// consistent but not fully caught up to the most recent commit.
        /// Callers use this to decide whether to keep paging.
        has_more: bool,
    },
    /// More pages are required before the view is consistent.
    InProgress { progress: SyncProgress },
}

/// One page of the data sync API.
pub struct SyncResult {
    /// Tables truncated by this page. Logically apply before `entries`.
    pub truncates: Vec<SyncTruncate>,
    pub entries: Vec<SyncEntry>,
    pub cursor: SyncCursor,
    pub status: SyncStatus,
    pub usage: FunctionUsageStats,
}

/// An opaque, forward-compatible cursor for the data sync API. It wraps the
/// low-level [`DataSyncCursor`] together with the (component, table) name each
/// captured tablet resolved to, which is used to detect table replacements and
/// emit a [`SyncTruncate`]. Serialized via protobuf; clients treat it as an
/// opaque token.
#[derive(Clone, Debug)]
pub struct SyncCursor {
    inner: DataSyncCursor,
    /// Names of every tablet captured by `inner` (its synced tables plus the
    /// in-progress table), as resolved when they were captured.
    names: BTreeMap<TabletId, (ComponentPath, TableName)>,
    /// Unique id assigned when the sync started; keys the
    /// `_data_sync_progress` row that tracks this sync's progress.
    sync_id: String,
}

impl SyncCursor {
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        Ok(self.to_proto()?.encode_to_vec())
    }

    pub fn from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        Self::from_proto(pb_ds::DataSyncCursor::decode(bytes)?)
    }

    /// Unique id of the sync this cursor belongs to.
    pub fn sync_id(&self) -> &str {
        &self.sync_id
    }

    /// Documents (including tombstones and re-emitted revisions) emitted over
    /// the sync's lifetime.
    pub fn num_docs_synced(&self) -> u64 {
        self.inner.num_docs_synced()
    }

    /// Tables whose entire ID space has been traversed. When the sync is
    /// [`SyncStatus::Synced`] this is every target table.
    pub fn num_synced_tables(&self) -> u64 {
        self.inner.synced_tables().len() as u64
    }

    fn name_of(&self, tablet_id: &TabletId) -> anyhow::Result<(String, String)> {
        let (component, table) = self
            .names
            .get(tablet_id)
            .ok_or_else(|| anyhow::anyhow!("missing name for captured tablet {tablet_id:?}"))?;
        Ok((String::from(component.clone()), table.to_string()))
    }

    fn to_proto(&self) -> anyhow::Result<pb_ds::DataSyncCursor> {
        let synced_tablets = self
            .inner
            .synced_tables()
            .iter()
            .map(|tablet_id| -> anyhow::Result<_> {
                let (component_path, table_name) = self.name_of(tablet_id)?;
                Ok(pb_ds::SyncedTablet {
                    tablet_id: Some(Vec::from(tablet_id.0)),
                    component_path: Some(component_path),
                    table_name: Some(table_name),
                })
            })
            .collect::<anyhow::Result<_>>()?;
        let table_cursor = match self.inner.in_progress_table() {
            None => pb_ds::data_sync_cursor::TableCursor::Synced(()),
            Some((tablet_id, current_id)) => {
                let (component_path, table_name) = self.name_of(&tablet_id)?;
                pb_ds::data_sync_cursor::TableCursor::InProgress(pb_ds::InProgressTablet {
                    tablet_id: Some(Vec::from(tablet_id.0)),
                    component_path: Some(component_path),
                    table_name: Some(table_name),
                    current_id: current_id.map(|id| id.into()),
                    docs_synced: Some(self.inner.current_table_docs_synced()),
                })
            },
        };
        Ok(pb_ds::DataSyncCursor {
            synced_ts: Some(u64::from(self.inner.synced_ts())),
            synced_tablets,
            table_cursor: Some(table_cursor),
            sync_id: Some(self.sync_id.clone()),
            num_docs_synced: Some(self.inner.num_docs_synced()),
        })
    }

    fn from_proto(proto: pb_ds::DataSyncCursor) -> anyhow::Result<Self> {
        let pb_ds::DataSyncCursor {
            synced_ts,
            synced_tablets,
            table_cursor,
            sync_id,
            num_docs_synced,
        } = proto;
        let synced_ts =
            Timestamp::try_from(synced_ts.ok_or_else(|| anyhow::anyhow!("missing synced_ts"))?)?;

        let mut names = BTreeMap::new();
        let mut synced_tables = std::collections::BTreeSet::new();
        for tablet in synced_tablets {
            let (tablet_id, component, table) =
                parse_named_tablet(tablet.tablet_id, tablet.component_path, tablet.table_name)?;
            synced_tables.insert(tablet_id);
            names.insert(tablet_id, (component, table));
        }

        let mut current_table_docs_synced = 0;
        let in_progress =
            match table_cursor.ok_or_else(|| anyhow::anyhow!("missing table_cursor"))? {
                pb_ds::data_sync_cursor::TableCursor::Synced(()) => None,
                pb_ds::data_sync_cursor::TableCursor::InProgress(in_progress) => {
                    let (tablet_id, component, table) = parse_named_tablet(
                        in_progress.tablet_id,
                        in_progress.component_path,
                        in_progress.table_name,
                    )?;
                    let current_id = in_progress
                        .current_id
                        .map(DeveloperDocumentId::try_from)
                        .transpose()?;
                    names.insert(tablet_id, (component, table));
                    current_table_docs_synced = in_progress.docs_synced.unwrap_or(0);
                    Some((tablet_id, current_id))
                },
            };

        Ok(Self {
            inner: DataSyncCursor::from_parts(
                synced_ts,
                synced_tables,
                in_progress,
                current_table_docs_synced,
                num_docs_synced.unwrap_or(0),
            ),
            names,
            sync_id: sync_id.ok_or_else(|| anyhow::anyhow!("missing sync_id"))?,
        })
    }
}

fn parse_named_tablet(
    tablet_id: Option<Vec<u8>>,
    component_path: Option<String>,
    table_name: Option<String>,
) -> anyhow::Result<(TabletId, ComponentPath, TableName)> {
    let tablet_id = TabletId(InternalId::try_from(
        tablet_id.ok_or_else(|| anyhow::anyhow!("missing tablet_id"))?,
    )?);
    let component: ComponentPath = component_path
        .ok_or_else(|| anyhow::anyhow!("missing component_path"))?
        .parse()?;
    let table: TableName = table_name
        .ok_or_else(|| anyhow::anyhow!("missing table_name"))?
        .parse()?;
    Ok((tablet_id, component, table))
}

/// Whether `tablet_id` is included in the streaming export given `filter`.
/// (Mirrors the equivalent check used by `list_snapshot`/`document_deltas`.)
fn table_included(
    filter: &StreamingExportFilter,
    tablet_id: TabletId,
    table_mapping: &TableMapping,
    component_paths: &BTreeMap<ComponentId, ComponentPath>,
) -> anyhow::Result<bool> {
    if !table_mapping.id_exists(tablet_id) {
        // Always exclude deleted tablets.
        return Ok(false);
    }
    if !filter.include_system && table_mapping.is_system_tablet(tablet_id) {
        return Ok(false);
    }
    if !filter.include_hidden && !table_mapping.is_active(tablet_id) {
        return Ok(false);
    }
    let (table_namespace, _, table_name) = table_mapping
        .get_table_metadata(tablet_id)
        .with_context(|| format!("Can't find the table entry for the tablet id {tablet_id}"))?;
    let Some(component_path) = component_paths.get(&ComponentId::from(*table_namespace)) else {
        tracing::warn!("Ignoring orphaned table in streaming export: {table_namespace:?}");
        return Ok(false);
    };
    Ok(filter
        .selection
        .is_table_included(component_path, table_name))
}

/// Produce the next page of a streaming export ("data sync").
///
/// `cursor: None` starts a fresh sync. `filter` selects the components, tables
/// and columns to export; it is compared against the cursor on every call so
/// tables can be added or removed between pages (a removed-then-re-added table,
/// e.g. from `npx convex import`, yields a [`SyncTruncate`]).
#[fastrace::trace]
pub async fn data_sync<RT: Runtime>(
    database: &Database<RT>,
    identity: Identity,
    cursor: Option<SyncCursor>,
    filter: StreamingExportFilter,
    // Integration issuing the sync, prepended to the freshly-minted sync id on
    // a cold start so callers can tell apart syncs from different sources.
    sync_client: DataSyncClient,
) -> anyhow::Result<SyncResult> {
    let usage = FunctionUsageTracker::new();
    anyhow::ensure!(
        identity.is_system() || identity.is_admin(),
        unauthorized_error("data_sync")
    );

    // Resolve the filter to concrete tablets at a recent, consistent snapshot.
    // Tablet ids are stable, so this mapping is valid for the iterator's own
    // (possibly slightly newer) `latest` timestamp.
    let (table_mapping, component_paths, by_id_indexes, table_counts) = {
        let mut tx = database.begin(identity).await?;
        let table_mapping = tx.table_mapping().clone();
        let component_paths = BootstrapComponentsModel::new(&mut tx).all_component_paths();
        let by_id_indexes = IndexModel::new(&mut tx).by_id_indexes().await?;
        // Incrementally-maintained per-table document counts, used only for
        // progress reporting. `None` while table summaries are bootstrapping.
        let table_counts = database.snapshot(tx.begin_timestamp())?.table_counts;
        (table_mapping, component_paths, by_id_indexes, table_counts)
    };
    let resolve_name = |tablet_id: TabletId| -> anyhow::Result<(ComponentPath, TableName)> {
        let table_name = table_mapping.tablet_name(tablet_id)?;
        let component_id = ComponentId::from(table_mapping.tablet_namespace(tablet_id)?);
        let component_path = component_paths
            .get(&component_id)
            .cloned()
            .unwrap_or_else(ComponentPath::root);
        Ok((component_path, table_name))
    };

    let mut target_tables: BTreeMap<TabletId, IndexId> = BTreeMap::new();
    // (component, table) -> tablet id, to detect a table being replaced.
    let mut current_by_name: BTreeMap<(ComponentPath, TableName), TabletId> = BTreeMap::new();
    for (tablet_id, ..) in table_mapping.iter() {
        if !table_included(&filter, tablet_id, &table_mapping, &component_paths)? {
            continue;
        }
        let by_id = *by_id_indexes
            .get(&tablet_id)
            .ok_or_else(|| anyhow::anyhow!("by_id index for {tablet_id:?} missing"))?;
        target_tables.insert(tablet_id, by_id);
        current_by_name.insert(resolve_name(tablet_id)?, tablet_id);
    }

    // Detect tables the cursor had already captured that have since been
    // replaced (same name, different tablet). Report a truncate for each; the
    // iterator's own reconciliation drops the stale tablet and re-syncs the new
    // one.
    let mut truncates = Vec::new();
    if let Some(cursor) = &cursor {
        for (old_tablet, (component, table)) in &cursor.names {
            if target_tables.contains_key(old_tablet) {
                continue;
            }
            if let Some(new_tablet) = current_by_name.get(&(component.clone(), table.clone()))
                && new_tablet != old_tablet
            {
                truncates.push(SyncTruncate {
                    component: component.clone(),
                    table: table.clone(),
                });
            }
        }
    }

    // Adopt the cursor's sync id, assigning a fresh one on cold start.
    let sync_id = cursor
        .as_ref()
        .map(|c| c.sync_id.clone())
        .unwrap_or_else(|| {
            format!(
                "{}{}",
                sync_client.sync_id_prefix(),
                database.runtime().new_uuid_v4()
            )
        });

    let mut entries = Vec::new();
    let iterator = database.data_sync_iterator()?;
    let page = iterator
        .next_page(cursor.map(|c| c.inner), &target_tables)
        .await?;

    for DocumentLogEntry { ts, id, value, .. } in page.entries {
        let tablet_id = id.table();
        let (component, table) = resolve_name(tablet_id)?;
        match value {
            Some(doc) => {
                let column_filter = filter.selection.column_filter(&component, &table)?;
                let document = column_filter.filter_document(doc.to_developer())?;
                usage.track_database_egress_v2(
                    component.clone(),
                    table.to_string(),
                    document.size() as u64,
                    false,
                );
                usage.track_database_egress_rows(component.clone(), table.to_string(), 1, false);
                entries.push(SyncEntry::Document {
                    ts,
                    component,
                    table,
                    document,
                });
            },
            None => {
                let table_number = table_mapping.tablet_number(tablet_id)?;
                let developer_id = DeveloperDocumentId::new(table_number, id.internal_id());
                entries.push(SyncEntry::Tombstone {
                    ts,
                    component,
                    table,
                    id: developer_id,
                });
            },
        }
    }

    // Re-resolve names for every tablet the new cursor still captures. After
    // reconciliation these are all live tablets present in `table_mapping`.
    let mut names = BTreeMap::new();
    for tablet_id in page.cursor.synced_tables() {
        names.insert(*tablet_id, resolve_name(*tablet_id)?);
    }
    if let Some((tablet_id, _)) = page.cursor.in_progress_table() {
        names.insert(tablet_id, resolve_name(tablet_id)?);
    }

    let status = match page.status {
        DataSyncStatus::Synced { ts, has_more } => SyncStatus::Synced { ts, has_more },
        DataSyncStatus::InProgress { progress } => {
            let (current_component, current_table) = resolve_name(progress.current_table)?;
            // Progress denominators from the incrementally-maintained table
            // counts: no table scans, just map lookups.
            let total_documents_in_current_table = table_counts
                .as_ref()
                .map(|counts| counts.tablet_count(&progress.current_table).num_values());
            let total_documents = table_counts.as_ref().map(|counts| {
                target_tables
                    .keys()
                    .map(|tablet_id| counts.tablet_count(tablet_id).num_values())
                    .sum()
            });
            SyncStatus::InProgress {
                progress: SyncProgress {
                    num_tables_synced: progress.num_tables_synced,
                    total_tables: progress.total_tables,
                    current_component,
                    current_table,
                    num_documents_in_current_table: progress.num_documents_in_current_table,
                    total_documents_in_current_table,
                    num_documents_synced: progress.num_documents_synced,
                    total_documents,
                },
            }
        },
    };

    Ok(SyncResult {
        truncates,
        entries,
        cursor: SyncCursor {
            inner: page.cursor,
            names,
            sync_id,
        },
        status,
        usage: usage.gather_user_stats(),
    })
}
