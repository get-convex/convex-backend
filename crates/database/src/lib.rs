#![feature(assert_matches)]
#![feature(lint_reasons)]
#![feature(coroutines)]
#![feature(result_flattening)]
#![feature(iter_advance_by)]
#![feature(type_alias_impl_trait)]
#![feature(let_chains)]
#![feature(lazy_cell)]
#![feature(const_option)]
#![feature(is_sorted)]
#![feature(iterator_try_collect)]
#![feature(never_type)]
#![feature(try_blocks)]
#![feature(exclusive_range_pattern)]
#![feature(async_closure)]
#![feature(trait_upcasting)]
#![feature(impl_trait_in_assoc_type)]
#![feature(cow_is_borrowed)]
#![feature(try_find)]

mod bootstrap_model;
mod committer;
mod database;
mod execution_size;
mod index_worker;
mod index_workers;
mod metrics;
pub mod patch;
pub mod persistence_helpers;
mod preloaded;
pub mod query;
pub mod reads;
mod retention;
mod search_index_bootstrap;
mod snapshot_manager;
mod stack_traces;
pub mod subscription;
mod table_registry;
pub mod table_summary;
mod token;
mod transaction;
mod transaction_id_generator;
mod transaction_index;
pub mod vector_index_worker;
mod virtual_tables;
mod write_limits;
mod write_log;
mod writes;

mod table_iteration;
#[cfg(any(test, feature = "testing"))]
pub mod test_helpers;
#[cfg(test)]
pub mod tests;
pub mod text_index_worker;
pub use execution_size::FunctionExecutionSize;
pub use index_worker::IndexWorker;
pub use index_workers::{
    fast_forward::FastForwardIndexWorker,
    search_worker::SearchIndexWorkers,
};
pub use patch::PatchValue;
pub use preloaded::PreloadedIndexRange;
pub use reads::{
    ReadSet,
    TransactionReadSet,
    TransactionReadSize,
    OVER_LIMIT_HELP,
};
pub use table_registry::TableRegistry;
pub use token::{
    SerializedToken,
    Token,
};
pub use transaction::{
    TableCountSnapshot,
    Transaction,
};
pub use transaction_index::{
    TextIndexManagerSnapshot,
    TransactionTextSnapshot,
};
pub use vector_index_worker::flusher::VectorIndexFlusher;
pub use write_limits::BiggestDocumentWrites;
pub use write_log::{
    LogReader,
    WriteSource,
};
pub use writes::{
    DocumentWrite,
    TransactionWriteSize,
    Writes,
};

pub use self::{
    bootstrap_model::{
        components::{
            definition::{
                ComponentDefinitionsTable,
                COMPONENT_DEFINITIONS_TABLE,
            },
            BootstrapComponentsModel,
            ComponentsTable,
            COMPONENTS_BY_PARENT_INDEX,
            COMPONENTS_TABLE,
        },
        defaults,
        import_facing::ImportFacingModel,
        index::{
            IndexModel,
            IndexTable,
            LegacyIndexDiff,
        },
        index_workers::{
            IndexWorkerMetadataTable,
            INDEX_DOC_ID_INDEX,
            INDEX_WORKER_METADATA_TABLE,
        },
        schema::{
            types::{
                SchemaDiff,
                SerializedSchemaDiff,
            },
            SchemaModel,
            SchemasTable,
            SCHEMAS_STATE_INDEX,
            SCHEMAS_TABLE,
            SCHEMA_STATE_FIELD,
        },
        system_metadata::SystemMetadataModel,
        table::{
            TableModel,
            TablesTable,
            NUM_RESERVED_LEGACY_TABLE_NUMBERS,
            NUM_RESERVED_SYSTEM_TABLE_NUMBERS,
            TABLES_INDEX,
        },
        user_facing::UserFacingModel,
        virtual_tables::{
            types::VirtualTableMetadata,
            VirtualTablesTable,
            VIRTUAL_TABLES_TABLE,
        },
    },
    database::{
        unauthorized_error,
        BootstrapMetadata,
        Database,
        DatabaseSnapshot,
        DocumentDeltas,
        OccRetryStats,
        ShutdownSignal,
        SnapshotPage,
        StreamingExportTableFilter,
        MAX_OCC_FAILURES,
    },
    index_worker::{
        IndexSelector,
        IndexWriter,
    },
    query::{
        soft_data_limit,
        DeveloperQuery,
        ResolvedQuery,
    },
    retention::{
        latest_retention_min_snapshot_ts,
        FollowerRetentionManager,
        LeaderRetentionManager,
        RetentionType,
    },
    snapshot_manager::{
        Snapshot,
        TableSummaries,
    },
    subscription::Subscription,
    table_iteration::TableIterator,
    table_summary::{
        TableSummary,
        TableSummaryWriter,
    },
    transaction::DEFAULT_PAGE_SIZE,
    transaction_id_generator::TransactionIdGenerator,
    transaction_index::TransactionIndex,
};
#[cfg(any(test, feature = "testing"))]
pub use crate::bootstrap_model::test_facing::TestFacingModel;
pub use crate::metrics::shutdown_error;
