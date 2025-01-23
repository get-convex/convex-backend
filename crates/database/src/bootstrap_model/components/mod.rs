pub mod definition;

use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use anyhow::Context;
use common::{
    bootstrap_model::components::{
        definition::{
            ComponentDefinitionMetadata,
            ComponentDefinitionType,
        },
        ComponentMetadata,
        ComponentType,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        CanonicalizedComponentModulePath,
        ComponentDefinitionId,
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
        Resource,
    },
    document::{
        ParsedDocument,
        ResolvedDocument,
    },
    query::{
        Order,
        Query,
    },
    runtime::Runtime,
    types::IndexName,
};
use errors::ErrorMetadata;
use value::{
    identifier::Identifier,
    ConvexValue,
    DeveloperDocumentId,
    FieldPath,
    ResolvedDocumentId,
    TableName,
    TableNamespace,
};

use crate::{
    defaults::{
        system_index,
        SystemIndex,
        SystemTable,
    },
    metrics,
    ResolvedQuery,
    Transaction,
    COMPONENT_DEFINITIONS_TABLE,
};

pub static COMPONENTS_TABLE: LazyLock<TableName> = LazyLock::new(|| {
    "_components"
        .parse()
        .expect("Invalid built-in _components table")
});

pub static COMPONENTS_BY_PARENT_INDEX: LazyLock<IndexName> =
    LazyLock::new(|| system_index(&COMPONENTS_TABLE, "by_parent_and_name"));
pub static PARENT_FIELD: LazyLock<FieldPath> = LazyLock::new(|| "parent".parse().unwrap());
pub static NAME_FIELD: LazyLock<FieldPath> = LazyLock::new(|| "name".parse().unwrap());

pub struct ComponentsTable;

impl SystemTable for ComponentsTable {
    fn table_name(&self) -> &'static TableName {
        &COMPONENTS_TABLE
    }

    fn indexes(&self) -> Vec<SystemIndex> {
        vec![SystemIndex {
            name: COMPONENTS_BY_PARENT_INDEX.clone(),
            fields: vec![PARENT_FIELD.clone(), NAME_FIELD.clone()]
                .try_into()
                .unwrap(),
        }]
    }

    fn validate_document(&self, document: ResolvedDocument) -> anyhow::Result<()> {
        ParsedDocument::<ComponentMetadata>::try_from(document)?;
        Ok(())
    }
}

pub struct BootstrapComponentsModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> BootstrapComponentsModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub fn component_in_parent(
        &mut self,
        parent_and_name: Option<(DeveloperDocumentId, ComponentName)>,
    ) -> anyhow::Result<Option<ParsedDocument<ComponentMetadata>>> {
        self.tx
            .component_registry
            .component_in_parent(parent_and_name, &mut self.tx.reads)
    }

    pub fn component_children(
        &mut self,
        parent_id: DeveloperDocumentId,
    ) -> anyhow::Result<Vec<ParsedDocument<ComponentMetadata>>> {
        self.tx
            .component_registry
            .component_children(parent_id, &mut self.tx.reads)
    }

    pub fn root_component(&mut self) -> anyhow::Result<Option<ParsedDocument<ComponentMetadata>>> {
        self.component_in_parent(None)
    }

    pub fn resolve_path(
        &mut self,
        path: &ComponentPath,
    ) -> anyhow::Result<Option<ParsedDocument<ComponentMetadata>>> {
        self.tx
            .component_registry
            .resolve_path(path, &mut self.tx.reads)
    }

    pub fn all_component_paths(&mut self) -> BTreeMap<ComponentId, ComponentPath> {
        self.tx
            .component_registry
            .all_component_paths(&mut self.tx.reads)
    }

    #[fastrace::trace]
    pub async fn load_all_components(
        &mut self,
    ) -> anyhow::Result<Vec<ParsedDocument<ComponentMetadata>>> {
        let mut query = ResolvedQuery::new(
            self.tx,
            TableNamespace::Global,
            Query::full_table_scan(COMPONENTS_TABLE.clone(), Order::Asc),
        )?;
        let mut components = Vec::new();
        while let Some(doc) = query.next(self.tx, None).await? {
            components.push(doc.try_into()?);
        }
        Ok(components)
    }

    pub fn resolve_component_id(
        &mut self,
        component_id: DeveloperDocumentId,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let component_table = self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(&COMPONENTS_TABLE)?;
        Ok(ResolvedDocumentId::new(
            component_table.tablet_id,
            component_id,
        ))
    }

    pub fn resolve_component_definition_id(
        &mut self,
        component_definition_id: DeveloperDocumentId,
    ) -> anyhow::Result<ResolvedDocumentId> {
        let component_definitions_table = self
            .tx
            .table_mapping()
            .namespace(TableNamespace::Global)
            .id(&COMPONENT_DEFINITIONS_TABLE)?;
        Ok(ResolvedDocumentId::new(
            component_definitions_table.tablet_id,
            component_definition_id,
        ))
    }

    pub fn get_component_path(&mut self, component_id: ComponentId) -> Option<ComponentPath> {
        self.tx.get_component_path(component_id)
    }

    pub fn must_component_path(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<ComponentPath> {
        self.get_component_path(component_id)
            .with_context(|| format!("component {component_id:?} missing"))
    }

    pub async fn component_definition(
        &mut self,
        component: ComponentId,
    ) -> anyhow::Result<ComponentDefinitionId> {
        let component_definition = match component {
            ComponentId::Root => ComponentDefinitionId::Root,
            ComponentId::Child(component_id) => {
                let component_doc_id = self.resolve_component_id(component_id)?;
                let component_doc: ParsedDocument<ComponentMetadata> = self
                    .tx
                    .get(component_doc_id)
                    .await?
                    .context("component missing")?
                    .try_into()?;
                ComponentDefinitionId::Child(component_doc.definition_id)
            },
        };
        Ok(component_definition)
    }

    pub async fn load_component(
        &mut self,
        id: ComponentId,
    ) -> anyhow::Result<Option<ParsedDocument<ComponentMetadata>>> {
        let result = match id {
            ComponentId::Root => self.root_component()?,
            ComponentId::Child(internal_id) => {
                let component_doc_id = self.resolve_component_id(internal_id)?;
                self.tx
                    .get(component_doc_id)
                    .await?
                    .map(TryInto::try_into)
                    .transpose()?
            },
        };
        Ok(result)
    }

    pub async fn load_component_type(&mut self, id: ComponentId) -> anyhow::Result<ComponentType> {
        match self.load_component(id).await? {
            None => {
                if id.is_root() {
                    // The root component's metadata document may be missing if the app hasn't been
                    // updated to use components.
                    Ok(ComponentType::App)
                } else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        format!("Component {:?} not found", id),
                    ))
                }
            },
            Some(component) => Ok(component.into_value().component_type),
        }
    }

    pub async fn load_component_args(
        &mut self,
        id: ComponentId,
    ) -> anyhow::Result<BTreeMap<Identifier, ConvexValue>> {
        let component = self
            .load_component(id)
            .await?
            .context(format!(
                "Component not found for ComponentId {:?}",
                id.serialize_to_string()
            ))?
            .into_value();
        let args = match component.component_type {
            ComponentType::App => anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidComponentType",
                "Can't load component args within the app",
            )),
            ComponentType::ChildComponent { args, .. } => args,
        };
        let mut result = BTreeMap::new();
        for (name, value) in args {
            let Resource::Value(value) = value else {
                anyhow::bail!("Non-value resource within component args");
            };
            result.insert(name, value);
        }
        Ok(result)
    }

    pub async fn load_definition(
        &mut self,
        id: ComponentDefinitionId,
    ) -> anyhow::Result<Option<ParsedDocument<ComponentDefinitionMetadata>>> {
        let internal_id = match id {
            ComponentDefinitionId::Root => match self.root_component()? {
                Some(root_component) => root_component.definition_id,
                None => return Ok(None),
            },
            ComponentDefinitionId::Child(id) => id,
        };
        let component_definition_doc_id = self.resolve_component_definition_id(internal_id)?;
        let Some(doc) = self.tx.get(component_definition_doc_id).await? else {
            return Ok(None);
        };
        let mut doc: ParsedDocument<ComponentDefinitionMetadata> = doc.try_into()?;
        if !doc.exports.is_empty() {
            metrics::log_nonempty_component_exports();
            doc.exports = BTreeMap::new();
        }
        Ok(Some(doc))
    }

    pub async fn load_definition_metadata(
        &mut self,
        id: ComponentDefinitionId,
    ) -> anyhow::Result<ComponentDefinitionMetadata> {
        match self.load_definition(id).await? {
            Some(doc) => Ok(doc.into_value()),
            None => {
                if id.is_root() {
                    // The root component's metadata document may be missing if the app hasn't been
                    // updated to use components.
                    Ok(ComponentDefinitionMetadata {
                        path: ComponentDefinitionPath::root(),
                        definition_type: ComponentDefinitionType::App,
                        child_components: Vec::new(),
                        exports: BTreeMap::new(),
                        http_mounts: BTreeMap::new(),
                    })
                } else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        format!("Component definition {:?} not found", id),
                    ))
                }
            },
        }
    }

    #[fastrace::trace]
    pub async fn load_all_definitions(
        &mut self,
    ) -> anyhow::Result<
        BTreeMap<ComponentDefinitionPath, ParsedDocument<ComponentDefinitionMetadata>>,
    > {
        let mut query = ResolvedQuery::new(
            self.tx,
            TableNamespace::Global,
            Query::full_table_scan(COMPONENT_DEFINITIONS_TABLE.clone(), Order::Asc),
        )?;
        let mut definitions = BTreeMap::new();
        while let Some(doc) = query.next(self.tx, None).await? {
            let mut definition: ParsedDocument<ComponentDefinitionMetadata> = doc.try_into()?;
            if !definition.exports.is_empty() {
                metrics::log_nonempty_component_exports();
                definition.exports = BTreeMap::new();
            }
            anyhow::ensure!(definitions
                .insert(definition.path.clone(), definition)
                .is_none());
        }
        Ok(definitions)
    }

    pub fn component_path_to_ids(
        &mut self,
        path: &ComponentPath,
    ) -> anyhow::Result<Option<(ComponentDefinitionId, ComponentId)>> {
        self.tx
            .component_registry
            .component_path_to_ids(path, &mut self.tx.reads)
    }

    pub fn must_component_path_to_ids(
        &mut self,
        path: &ComponentPath,
    ) -> anyhow::Result<(ComponentDefinitionId, ComponentId)> {
        self.component_path_to_ids(path)?
            .context("Component path not found")
    }

    pub fn function_path_to_module(
        &mut self,
        path: &CanonicalizedComponentFunctionPath,
    ) -> anyhow::Result<CanonicalizedComponentModulePath> {
        let (_, component) = self.must_component_path_to_ids(&path.component)?;
        Ok(CanonicalizedComponentModulePath {
            component,
            module_path: path.udf_path.module().clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use common::{
        bootstrap_model::components::{
            definition::{
                ComponentDefinitionMetadata,
                ComponentDefinitionType,
                ComponentInstantiation,
            },
            ComponentMetadata,
            ComponentState,
            ComponentType,
        },
        components::{
            ComponentDefinitionPath,
            ComponentId,
            ComponentPath,
        },
    };
    use keybroker::Identity;
    use runtime::testing::TestRuntime;

    use super::definition::COMPONENT_DEFINITIONS_TABLE;
    use crate::{
        bootstrap_model::components::{
            BootstrapComponentsModel,
            COMPONENTS_TABLE,
        },
        test_helpers::new_test_database,
        SystemMetadataModel,
    };

    #[convex_macro::test_runtime]
    async fn test_component_path(rt: TestRuntime) -> anyhow::Result<()> {
        let db = new_test_database(rt.clone()).await;
        let mut tx = db.begin(Identity::system()).await?;
        let child_definition_path: ComponentDefinitionPath = "../app/child".parse().unwrap();
        let child_definition_id = SystemMetadataModel::new_global(&mut tx)
            .insert(
                &COMPONENT_DEFINITIONS_TABLE,
                ComponentDefinitionMetadata {
                    path: child_definition_path.clone(),
                    definition_type: ComponentDefinitionType::ChildComponent {
                        name: "child".parse().unwrap(),
                        args: BTreeMap::new(),
                    },
                    child_components: Vec::new(),
                    http_mounts: BTreeMap::new(),
                    exports: BTreeMap::new(),
                }
                .try_into()?,
            )
            .await?;
        let root_definition_id = SystemMetadataModel::new_global(&mut tx)
            .insert(
                &COMPONENT_DEFINITIONS_TABLE,
                ComponentDefinitionMetadata {
                    path: "".parse().unwrap(),
                    definition_type: ComponentDefinitionType::App,
                    child_components: vec![ComponentInstantiation {
                        name: "child_subcomponent".parse().unwrap(),
                        path: child_definition_path,
                        args: Some(BTreeMap::new()),
                    }],
                    http_mounts: BTreeMap::new(),
                    exports: BTreeMap::new(),
                }
                .try_into()?,
            )
            .await?;
        let root_id = SystemMetadataModel::new_global(&mut tx)
            .insert(
                &COMPONENTS_TABLE,
                ComponentMetadata {
                    definition_id: root_definition_id.into(),
                    component_type: ComponentType::App,
                    state: ComponentState::Active,
                }
                .try_into()?,
            )
            .await?;
        let child_id = SystemMetadataModel::new_global(&mut tx)
            .insert(
                &COMPONENTS_TABLE,
                ComponentMetadata {
                    definition_id: child_definition_id.into(),
                    component_type: ComponentType::ChildComponent {
                        parent: root_id.into(),
                        name: "subcomponent_child".parse()?,
                        args: Default::default(),
                    },
                    state: ComponentState::Active,
                }
                .try_into()?,
            )
            .await?;
        let resolved_path = BootstrapComponentsModel::new(&mut tx)
            .resolve_path(&ComponentPath::from(vec!["subcomponent_child".parse()?]))?;
        assert_eq!(resolved_path.unwrap().id(), child_id);
        let path = BootstrapComponentsModel::new(&mut tx)
            .must_component_path(ComponentId::Child(child_id.into()))?;
        assert_eq!(
            path,
            ComponentPath::from(vec!["subcomponent_child".parse()?]),
        );
        Ok(())
    }
}
