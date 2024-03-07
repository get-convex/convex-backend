use std::collections::BTreeMap;

use common::{
    document::ResolvedDocument,
    interval::{
        BinaryKey,
        Interval,
    },
    runtime::Runtime,
    types::TabletIndexName,
};
use value::{
    values_to_bytes,
    ConvexValue,
    FieldPath,
    TableName,
};

use crate::Transaction;

pub struct PreloadedIndexRange {
    table_name: TableName,
    tablet_index_name: TabletIndexName,
    indexed_field: FieldPath,
    range: BTreeMap<Option<ConvexValue>, ResolvedDocument>,
}

impl PreloadedIndexRange {
    pub(crate) fn new(
        table_name: TableName,
        tablet_index_name: TabletIndexName,
        indexed_field: FieldPath,
        range: BTreeMap<Option<ConvexValue>, ResolvedDocument>,
    ) -> Self {
        Self {
            table_name,
            tablet_index_name,
            indexed_field,
            range,
        }
    }

    pub fn get<RT: Runtime>(
        &self,
        tx: &mut Transaction<RT>,
        key: &Option<ConvexValue>,
    ) -> anyhow::Result<Option<&ResolvedDocument>> {
        tx.reads.record_indexed_directly(
            self.tablet_index_name.clone(),
            vec![self.indexed_field.clone()].try_into()?,
            Interval::prefix(BinaryKey::from(values_to_bytes(&[key.clone()]))),
        )?;
        let result = self.range.get(key);
        if let Some(document) = result {
            tx.record_read_document(document, &self.table_name)?;
        }
        Ok(result)
    }
}
