use std::sync::LazyLock;

use common::{
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    types::IndexName,
};
use database::defaults::system_index;
use value::{
    FieldPath,
    TableName,
};

use self::types::Export;
use crate::{
    SystemIndex,
    SystemTable,
};

pub mod types;

pub static EXPORTS_TABLE: LazyLock<TableName> =
    LazyLock::new(|| "_exports".parse().expect("Invalid built-in exports table"));

// TODO(lee): replace with by_state_and_ts, and delete this index.
pub static EXPORTS_BY_STATE_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&EXPORTS_TABLE, "by_state"));

pub static EXPORTS_BY_STATE_AND_TS_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&EXPORTS_TABLE, "by_state_and_ts"));

pub static EXPORTS_STATE_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "state".parse().expect("Invalid built-in field"));

pub static EXPORTS_TS_FIELD: LazyLock<FieldPath> =
    LazyLock::new(|| "start_ts".parse().expect("Invalid built-in field"));

pub struct ExportsTable;
impl SystemTable for ExportsTable {
    fn table_name(&self) -> &'static TableName {
        &EXPORTS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![
            SystemIndex {
                name: EXPORTS_BY_STATE_INDEX.clone(),
                fields: vec![EXPORTS_STATE_FIELD.clone()].try_into().unwrap(),
            },
            SystemIndex {
                name: EXPORTS_BY_STATE_AND_TS_INDEX.clone(),
                fields: vec![EXPORTS_STATE_FIELD.clone(), EXPORTS_TS_FIELD.clone()]
                    .try_into()
                    .unwrap(),
            },
        ]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<Export>::try_from(document).map(|_| ())
    }
}
