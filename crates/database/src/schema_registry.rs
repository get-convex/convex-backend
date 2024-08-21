use std::collections::BTreeMap;

use common::{
    bootstrap_model::{
        index::database_index::IndexedFields,
        schema::{
            SchemaMetadata,
            SchemaState,
        },
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        IndexRange,
        IndexRangeExpression,
        Order,
    },
    types::TabletIndexName,
    value::ResolvedDocumentId,
};
use imbl::OrdMap;
use value::{
    val,
    TableMapping,
    TableNamespace,
    TabletId,
};

use crate::{
    TransactionReadSet,
    SCHEMAS_STATE_INDEX,
    SCHEMAS_TABLE,
    SCHEMA_STATE_FIELD,
};

/// This structure is an index over the `_schemas` tables.
#[derive(Debug, Clone, PartialEq)]
pub struct SchemaRegistry {
    // Stores schemas where state.is_unique() is true.
    namespaced: OrdMap<TableNamespace, OrdMap<SchemaState, ParsedDocument<SchemaMetadata>>>,
}

impl SchemaRegistry {
    pub fn bootstrap(
        schema_docs: BTreeMap<TableNamespace, Vec<ParsedDocument<SchemaMetadata>>>,
    ) -> Self {
        let namespaced = schema_docs
            .into_iter()
            .map(|(namespace, docs)| {
                let schemas: OrdMap<_, _> = docs
                    .into_iter()
                    .filter(|schema| schema.state.is_unique())
                    .map(|schema| (schema.state.clone(), schema))
                    .collect();
                (namespace, schemas)
            })
            .collect();
        Self { namespaced }
    }

    pub(crate) fn update(
        &mut self,
        table_mapping: &TableMapping,
        id: ResolvedDocumentId,
        old_doc: Option<&ResolvedDocument>,
        new_doc: Option<&ResolvedDocument>,
    ) -> anyhow::Result<()> {
        self.begin_update(table_mapping, id, old_doc, new_doc)?
            .apply();
        Ok(())
    }

    pub(crate) fn begin_update<'a>(
        &'a mut self,
        table_mapping: &TableMapping,
        id: ResolvedDocumentId,
        old_doc: Option<&ResolvedDocument>,
        new_doc: Option<&ResolvedDocument>,
    ) -> anyhow::Result<Update<'a>> {
        let mut schema_update = None;
        let namespace = table_mapping.tablet_namespace(id.tablet_id)?;
        if table_mapping
            .namespace(namespace)
            .tablet_matches_name(id.tablet_id, &SCHEMAS_TABLE)
        {
            let old_schema = match old_doc {
                None => None,
                Some(old_doc) => Some(ParsedDocument::try_from(old_doc.clone())?),
            };
            let new_schema = match new_doc {
                None => None,
                Some(new_doc) => Some(ParsedDocument::try_from(new_doc.clone())?),
            };
            schema_update = Some(SchemaUpdate {
                namespace,
                old_schema,
                new_schema,
            });
        }
        Ok(Update {
            registry: self,
            update: schema_update,
        })
    }

    pub fn get_by_state(
        &self,
        namespace: TableNamespace,
        state: SchemaState,
        schema_tablet: TabletId,
        reads: &mut TransactionReadSet,
    ) -> anyhow::Result<Option<ParsedDocument<SchemaMetadata>>> {
        // Reading from the schema_registry, so take read dependency directly.
        let state_value = val!(state.clone());
        let index_range = IndexRange {
            index_name: SCHEMAS_STATE_INDEX.clone(),
            range: vec![IndexRangeExpression::Eq(
                SCHEMA_STATE_FIELD.clone(),
                state_value.into(),
            )],
            order: Order::Asc,
        };
        let fields = IndexedFields::try_from(vec![SCHEMA_STATE_FIELD.clone()])?;
        let interval = index_range.compile(fields.clone())?;
        reads.record_indexed_derived(TabletIndexName::by_id(schema_tablet), fields, interval);

        let schema = self
            .namespaced
            .get(&namespace)
            .and_then(|registry| registry.get(&state))
            .cloned();
        Ok(schema)
    }
}

pub(crate) struct SchemaUpdate {
    pub namespace: TableNamespace,
    pub old_schema: Option<ParsedDocument<SchemaMetadata>>,
    pub new_schema: Option<ParsedDocument<SchemaMetadata>>,
}

pub(crate) struct Update<'a> {
    registry: &'a mut SchemaRegistry,
    update: Option<SchemaUpdate>,
}

impl<'a> Update<'a> {
    pub(crate) fn apply(self) {
        if let Some(update) = self.update {
            let namespaced_registry = self
                .registry
                .namespaced
                .entry(update.namespace)
                .or_default();
            if let Some(old_schema) = update.old_schema {
                if let Some(cached) = namespaced_registry.get(&old_schema.state)
                    && cached == &old_schema
                {
                    namespaced_registry.remove(&old_schema.state);
                }
            }
            if let Some(new_schema) = update.new_schema {
                if new_schema.state.is_unique() {
                    namespaced_registry.insert(new_schema.state.clone(), new_schema);
                }
            }
        }
    }
}
