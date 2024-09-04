use std::collections::BTreeMap;

use common::{
    bootstrap_model::{
        components::{
            ComponentMetadata,
            ComponentType,
        },
        index::database_index::IndexedFields,
    },
    components::{
        ComponentId,
        ComponentPath,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    index::IndexKey,
    interval::Interval,
    types::TabletIndexName,
    value::ResolvedDocumentId,
};
use imbl::OrdMap;
use value::{
    DeveloperDocumentId,
    TableMapping,
    TableNamespace,
    TabletId,
};

use crate::{
    TransactionReadSet,
    COMPONENTS_TABLE,
};

/// This structure is an index over the `_components` tables.
/// TODO: Make the data structures more efficient. For now we just care about
/// correctness, since the main gain is keeping the parsed metadata in memory.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentRegistry {
    components_tablet: TabletId,
    components: OrdMap<DeveloperDocumentId, ParsedDocument<ComponentMetadata>>,
}

impl ComponentRegistry {
    pub fn bootstrap(
        table_mapping: &TableMapping,
        component_docs: Vec<ParsedDocument<ComponentMetadata>>,
    ) -> anyhow::Result<Self> {
        let components_tablet = table_mapping
            .namespace(TableNamespace::Global)
            .id(&COMPONENTS_TABLE)?
            .tablet_id;
        let components: OrdMap<_, _> = component_docs
            .into_iter()
            .map(|component| (component.developer_id(), component))
            .collect();
        Ok(Self {
            components_tablet,
            components,
        })
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
        let mut component_update = None;
        if table_mapping
            .namespace(TableNamespace::Global)
            .tablet_matches_name(id.tablet_id, &COMPONENTS_TABLE)
        {
            let old_component = match old_doc {
                None => None,
                Some(old_doc) => Some(ParsedDocument::try_from(old_doc.clone())?),
            };
            anyhow::ensure!(self.components.get(&id.developer_id) == old_component.as_ref());
            let new_component = match new_doc {
                None => None,
                Some(new_doc) => Some(ParsedDocument::try_from(new_doc.clone())?),
            };
            component_update = Some(ComponentUpdate {
                old_component,
                new_component,
            });
        }
        Ok(Update {
            registry: self,
            update: component_update,
        })
    }

    pub fn get_component_path(
        &self,
        mut component_id: ComponentId,
        reads: &mut TransactionReadSet,
    ) -> Option<ComponentPath> {
        let mut path = Vec::new();
        while let ComponentId::Child(internal_id) = component_id {
            let component_doc = self.get_component(internal_id, reads)?;
            // TODO: consider returning None if unmounted.
            component_id = match &component_doc.component_type {
                ComponentType::App => ComponentId::Root,
                ComponentType::ChildComponent { parent, name, .. } => {
                    path.push(name.clone());
                    ComponentId::Child(*parent)
                },
            };
        }
        path.reverse();
        Some(ComponentPath::from(path))
    }

    pub fn all_component_paths(
        &self,
        reads: &mut TransactionReadSet,
    ) -> BTreeMap<ComponentId, ComponentPath> {
        reads.record_indexed_derived(
            TabletIndexName::by_id(self.components_tablet),
            IndexedFields::by_id(),
            Interval::all(),
        );
        let mut paths = BTreeMap::new();
        for id in self.components.keys() {
            let path = self.get_component_path(ComponentId::Child(*id), reads);
            if let Some(path) = path {
                if path.is_root() {
                    paths.insert(ComponentId::Root, path);
                } else {
                    paths.insert(ComponentId::Child(*id), path);
                }
            }
        }
        // In case the component doesn't exist, we still want to return the root path.
        paths.insert(ComponentId::Root, ComponentPath::root());
        paths
    }

    fn get_component(
        &self,
        id: DeveloperDocumentId,
        reads: &mut TransactionReadSet,
    ) -> Option<&ParsedDocument<ComponentMetadata>> {
        let index_key = IndexKey::new(vec![], id).into_bytes().into();
        reads.record_indexed_derived(
            TabletIndexName::by_id(self.components_tablet),
            IndexedFields::by_id(),
            Interval::prefix(index_key),
        );
        self.components.get(&id)
    }
}

pub(crate) struct ComponentUpdate {
    pub old_component: Option<ParsedDocument<ComponentMetadata>>,
    pub new_component: Option<ParsedDocument<ComponentMetadata>>,
}

pub(crate) struct Update<'a> {
    registry: &'a mut ComponentRegistry,
    update: Option<ComponentUpdate>,
}

impl<'a> Update<'a> {
    pub(crate) fn apply(self) {
        if let Some(update) = self.update {
            let components = &mut self.registry.components;
            if let Some(old_component) = update.old_component {
                components.remove(&old_component.developer_id());
            }
            if let Some(new_component) = update.new_component {
                components.insert(new_component.developer_id(), new_component);
            }
        }
    }
}
