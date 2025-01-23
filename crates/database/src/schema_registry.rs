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
    schemas::DatabaseSchema,
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
    namespaced: OrdMap<TableNamespace, NamespacedSchemaRegistry>,
}
#[derive(Debug, Clone, PartialEq)]
struct SchemaRegistryEntry {
    metadata: SchemaMetadata,
    database_schema: Option<DatabaseSchema>,
}

#[derive(Debug, Clone, PartialEq, Default)]
struct NamespacedSchemaRegistry {
    schemas_by_state: OrdMap<SchemaState, ResolvedDocumentId>,
    database_schemas: BTreeMap<ResolvedDocumentId, SchemaRegistryEntry>,
}

impl NamespacedSchemaRegistry {
    fn get_metadata(&self, state: &SchemaState) -> Option<&SchemaMetadata> {
        self.schemas_by_state
            .get(state)
            .and_then(|id| self.database_schemas.get(id).map(|entry| &entry.metadata))
    }

    pub fn get(
        &mut self,
        state: &SchemaState,
    ) -> anyhow::Result<Option<(ResolvedDocumentId, DatabaseSchema)>> {
        let doc_id = self.schemas_by_state.get(state);
        let Some(doc_id) = doc_id else {
            return Ok(None);
        };
        let Some(entry) = self.database_schemas.get_mut(doc_id) else {
            anyhow::bail!(
                "Schema registry missing database schema for document {}",
                doc_id
            );
        };
        let database_schema = match &mut entry.database_schema {
            None => {
                let schema = entry.metadata.database_schema()?;
                entry.database_schema = Some(schema.clone());
                schema
            },
            Some(schema) => schema.clone(),
        };
        Ok(Some((*doc_id, database_schema)))
    }

    pub fn update(&mut self, update: SchemaUpdate) {
        if let Some(old_schema) = update.old_schema {
            if !old_schema.state.is_unique() {
                return;
            }
            let old_schema_state = old_schema.state.clone();
            if let Some(cached) = self.get_metadata(&old_schema_state)
                && cached == &old_schema.into_value()
            {
                self.remove(&old_schema_state);
            }
        }
        if let Some(new_schema) = update.new_schema {
            if new_schema.state.is_unique() {
                self.insert(new_schema.state.clone(), new_schema);
            }
        }
    }

    fn remove(&mut self, state: &SchemaState) -> Option<ResolvedDocumentId> {
        let doc_id = self.schemas_by_state.remove(state);
        doc_id.and_then(|id| self.database_schemas.remove(&id).map(|_| id))
    }

    fn insert(&mut self, state: SchemaState, doc: ParsedDocument<SchemaMetadata>) {
        self.schemas_by_state.insert(state, doc.id());
        self.database_schemas.insert(
            doc.id(),
            SchemaRegistryEntry {
                metadata: doc.into_value(),
                database_schema: None,
            },
        );
    }
}

impl SchemaRegistry {
    pub fn bootstrap(
        schema_docs: BTreeMap<TableNamespace, Vec<ParsedDocument<SchemaMetadata>>>,
    ) -> Self {
        let namespaced = schema_docs
            .into_iter()
            .map(|(namespace, docs)| {
                let relevant_schemas: Vec<_> =
                    docs.into_iter().filter(|s| s.state.is_unique()).collect();
                let schemas_by_state: OrdMap<_, _> = relevant_schemas
                    .iter()
                    .map(|s| (s.state.clone(), s.id()))
                    .collect();
                let database_schemas: BTreeMap<_, _> = relevant_schemas
                    .into_iter()
                    .map(|s| {
                        (
                            s.id(),
                            SchemaRegistryEntry {
                                metadata: s.into_value(),
                                database_schema: None,
                            },
                        )
                    })
                    .collect();
                (
                    namespace,
                    NamespacedSchemaRegistry {
                        schemas_by_state,
                        database_schemas,
                    },
                )
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
        &mut self,
        namespace: TableNamespace,
        state: SchemaState,
        schema_tablet: TabletId,
        reads: &mut TransactionReadSet,
    ) -> anyhow::Result<Option<(ResolvedDocumentId, DatabaseSchema)>> {
        // Reading from the schema_registry, so take read dependency
        // directly.
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

        let namespaced_registry = self.namespaced.get_mut(&namespace);
        let Some(namespaced_registry) = namespaced_registry else {
            return Ok(None);
        };

        let schema = namespaced_registry.get(&state)?;
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

impl Update<'_> {
    pub(crate) fn apply(self) {
        if let Some(update) = self.update {
            let namespaced_registry = self
                .registry
                .namespaced
                .entry(update.namespace)
                .or_default();
            namespaced_registry.update(update);
        }
    }
}
