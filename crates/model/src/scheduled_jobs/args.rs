use std::sync::LazyLock;

use common::virtual_system_mapping::AssociatedVirtualTable;
use database::system_tables::{
    SystemIndex,
    SystemTable,
};
use value::TableName;

use crate::scheduled_jobs::{
    types::ScheduledJobArgs,
    SCHEDULED_JOBS_VIRTUAL_TABLE,
};

pub static SCHEDULED_JOBS_ARGS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_scheduled_job_args"
        .parse()
        .expect("_scheduled_job_args is not a valid system table name")
});

pub struct ScheduledJobArgsTable;

impl SystemTable for ScheduledJobArgsTable {
    type Metadata = ScheduledJobArgs;

    fn table_name() -> &'static TableName {
        &SCHEDULED_JOBS_ARGS_TABLE
    }

    fn indexes() -> Vec<SystemIndex<Self>> {
        vec![]
    }

    fn virtual_table() -> Option<AssociatedVirtualTable> {
        Some(AssociatedVirtualTable::Secondary(
            SCHEDULED_JOBS_VIRTUAL_TABLE.clone(),
        ))
    }
}
