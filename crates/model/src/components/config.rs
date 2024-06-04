use std::collections::BTreeMap;

use anyhow::Context;
use common::{
    bootstrap_model::components::{
        definition::ComponentDefinitionMetadata,
        ComponentMetadata,
        ComponentType,
    },
    components::{
        ComponentDefinitionId,
        ComponentDefinitionPath,
        ComponentId,
        ComponentName,
        ComponentPath,
    },
    document::ParsedDocument,
    runtime::Runtime,
};
use database::{
    BootstrapComponentsModel,
    SchemasTable,
    SystemMetadataModel,
    Transaction,
    COMPONENTS_TABLE,
    COMPONENT_DEFINITIONS_TABLE,
};
use errors::ErrorMetadata;
use serde::Serialize;
use serde_json::Value as JsonValue;
use sync_types::CanonicalizedModulePath;
use value::{
    ConvexObject,
    InternalId,
};

use super::{
    type_checking::CheckedComponent,
    types::EvaluatedComponentDefinition,
};
use crate::{
    component_definition_system_tables,
    component_system_tables,
    config::types::{
        ModuleConfig,
        ModuleDiff,
    },
    initialize_application_system_table,
    modules::{
        module_versions::AnalyzedModule,
        ModuleModel,
    },
    source_packages::{
        types::{
            SourcePackage,
            SourcePackageId,
        },
        SourcePackageModel,
    },
    DEFAULT_TABLE_NUMBERS,
};

pub struct ComponentDefinitionConfigModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ComponentDefinitionConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn diff_component_definitions(
        &mut self,
        new_definitions: &BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,
        source_packages: &BTreeMap<ComponentDefinitionPath, SourcePackage>,
        downloaded_source_packages: &BTreeMap<
            ComponentDefinitionPath,
            BTreeMap<CanonicalizedModulePath, ModuleConfig>,
        >,
    ) -> anyhow::Result<BTreeMap<ComponentDefinitionPath, ComponentDefinitionDiff>> {
        let mut definition_diffs = BTreeMap::new();

        let existing_definitions = BootstrapComponentsModel::new(self.tx)
            .load_all_definitions()
            .await?;

        // Delete all definitions that aren't in the new set.
        for (definition_path, existing_definition) in &existing_definitions {
            if new_definitions.contains_key(definition_path) {
                continue;
            }
            let diff = self
                .delete_component_definition(existing_definition)
                .await?;
            definition_diffs.insert(definition_path.clone(), diff);
        }

        for (definition_path, new_definition) in new_definitions {
            let source_package = source_packages.get(definition_path).ok_or_else(|| {
                ErrorMetadata::bad_request(
                    "MissingSourcePackage",
                    "Missing source package for component",
                )
            })?;
            let source_package_id = SourcePackageModel::new(self.tx)
                .put(source_package.clone())
                .await?;

            let downloaded_source_package = downloaded_source_packages
                .get(definition_path)
                .context("Missing downloaded source package for component")?;

            let mut functions = new_definition.functions.clone();
            let mut new_modules = vec![];

            for (module_path, module) in downloaded_source_package {
                // NB: The source package here may contain more modules (e.g. `_deps/*`) that
                // aren't in `new_definition.functions`.
                if !functions.contains_key(module_path) {
                    // TODO: It's a bit kludgy that we're filling in a default value here rather
                    // than earlier in the push pipeline.
                    tracing::warn!("Module not in functions: {:?}", module_path);
                    functions.insert(module_path.clone(), AnalyzedModule::default());
                }
                new_modules.push(module.clone());
            }

            let diff = match existing_definitions.get(definition_path) {
                Some(existing_definition) => {
                    self.modify_component_definition(
                        existing_definition,
                        source_package_id,
                        new_modules,
                        new_definition.definition.clone(),
                        functions,
                    )
                    .await?
                },
                None => {
                    self.create_component_definition(
                        new_definition.definition.clone(),
                        source_package_id,
                        new_modules,
                        functions,
                    )
                    .await?
                },
            };
            definition_diffs.insert(definition_path.clone(), diff);
        }

        Ok(definition_diffs)
    }

    pub async fn create_component_definition(
        &mut self,
        definition: ComponentDefinitionMetadata,
        source_package_id: SourcePackageId,
        new_modules: Vec<ModuleConfig>,
        functions: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<ComponentDefinitionDiff> {
        let is_root = definition.path.is_root();
        let id = SystemMetadataModel::new_global(self.tx)
            .insert(&COMPONENT_DEFINITIONS_TABLE, definition.clone().try_into()?)
            .await?;
        let definition_id = if is_root {
            ComponentDefinitionId::Root
        } else {
            ComponentDefinitionId::Child(id.internal_id())
        };

        initialize_application_system_table(
            self.tx,
            &SchemasTable,
            definition_id.into(),
            &DEFAULT_TABLE_NUMBERS,
        )
        .await?;
        for table in component_definition_system_tables() {
            initialize_application_system_table(
                self.tx,
                table,
                definition_id.into(),
                &DEFAULT_TABLE_NUMBERS,
            )
            .await?;
        }
        let module_diff = ModuleModel::new(self.tx)
            .apply(
                definition_id,
                new_modules,
                Some(source_package_id),
                functions,
            )
            .await?;
        let diff = ComponentDefinitionDiff { module_diff };
        Ok(diff)
    }

    pub async fn modify_component_definition(
        &mut self,
        existing: &ParsedDocument<ComponentDefinitionMetadata>,
        source_package_id: SourcePackageId,
        new_modules: Vec<ModuleConfig>,
        new_definition: ComponentDefinitionMetadata,
        new_functions: BTreeMap<CanonicalizedModulePath, AnalyzedModule>,
    ) -> anyhow::Result<ComponentDefinitionDiff> {
        SystemMetadataModel::new_global(self.tx)
            .replace(existing.id(), new_definition.clone().try_into()?)
            .await?;
        let definition_id = if existing.path.is_root() {
            ComponentDefinitionId::Root
        } else {
            ComponentDefinitionId::Child(existing.id().internal_id())
        };
        let module_diff = ModuleModel::new(self.tx)
            .apply(
                definition_id,
                new_modules,
                Some(source_package_id),
                new_functions,
            )
            .await?;
        let diff = ComponentDefinitionDiff { module_diff };
        Ok(diff)
    }

    pub async fn delete_component_definition(
        &mut self,
        existing: &ParsedDocument<ComponentDefinitionMetadata>,
    ) -> anyhow::Result<ComponentDefinitionDiff> {
        SystemMetadataModel::new_global(self.tx)
            .delete(existing.id())
            .await?;
        let definition_id = if existing.path.is_root() {
            ComponentDefinitionId::Root
        } else {
            ComponentDefinitionId::Child(existing.id().internal_id())
        };
        let module_diff = ModuleModel::new(self.tx)
            .apply(definition_id, vec![], None, BTreeMap::new())
            .await?;

        // TODO: Delete the module system tables.
        Ok(ComponentDefinitionDiff { module_diff })
    }
}

pub struct ComponentDefinitionDiff {
    pub module_diff: ModuleDiff,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerializedComponentDefinitionDiff {
    module_diff: JsonValue,
}

impl TryFrom<ComponentDefinitionDiff> for SerializedComponentDefinitionDiff {
    type Error = anyhow::Error;

    fn try_from(value: ComponentDefinitionDiff) -> Result<Self, Self::Error> {
        Ok(Self {
            module_diff: JsonValue::from(ConvexObject::try_from(value.module_diff)?),
        })
    }
}

pub struct ComponentConfigModel<'a, RT: Runtime> {
    tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ComponentConfigModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    pub async fn diff_component_tree(
        &mut self,
        app: &CheckedComponent,
    ) -> anyhow::Result<BTreeMap<ComponentPath, ComponentDiff>> {
        let definition_id_by_path = BootstrapComponentsModel::new(self.tx)
            .load_all_definitions()
            .await?
            .into_iter()
            .map(|(path, d)| (path, d.id().internal_id()))
            .collect::<BTreeMap<_, _>>();

        let existing_components_by_parent = BootstrapComponentsModel::new(self.tx)
            .load_all_components()
            .await?
            .into_iter()
            .map(|c| (c.parent_and_name(), c))
            .collect::<BTreeMap<_, _>>();

        let existing_root = existing_components_by_parent.get(&None);
        let mut stack = vec![(ComponentPath::root(), None, existing_root, Some(app))];
        let mut diffs = BTreeMap::new();
        while let Some((path, parent_and_name, existing_node, new_node)) = stack.pop() {
            let new_metadata = match new_node {
                Some(new_node) => {
                    let definition_id = *definition_id_by_path
                        .get(&new_node.definition_path)
                        .context("Missing definition ID for component")?;
                    let component_type = match parent_and_name {
                        None => {
                            anyhow::ensure!(new_node.args.is_empty());
                            ComponentType::App
                        },
                        Some((parent, name)) => ComponentType::ChildComponent {
                            parent,
                            name,
                            args: new_node.args.clone(),
                        },
                    };
                    Some(ComponentMetadata {
                        definition_id,
                        component_type,
                    })
                },
                None => None,
            };

            // Diff the node itself.
            let (internal_id, diff) = match (existing_node, new_metadata) {
                // Create a new node.
                (None, Some(new_metadata)) => self.create_component(new_metadata).await?,
                // Update a node.
                (Some(existing_node), Some(new_metadata)) => {
                    self.modify_component(existing_node, new_metadata).await?
                },
                // Delete an existing node.
                (Some(existing_node), None) => self.delete_component(existing_node).await?,
                (None, None) => anyhow::bail!("Unexpected None/None in stack"),
            };
            diffs.insert(path.clone(), diff);

            // After diffing the node, push children of the existing node onto the stack.
            for (parent_and_name, existing_child) in
                existing_components_by_parent.range(Some((internal_id, ComponentName::min()))..)
            {
                let Some((parent, name)) = parent_and_name else {
                    break;
                };
                if parent != &internal_id {
                    break;
                }
                let new_node = new_node.and_then(|new_node| new_node.child_components.get(name));
                stack.push((
                    path.join(name.clone()),
                    Some((internal_id, name.clone())),
                    Some(existing_child),
                    new_node,
                ));
            }

            // Then, push children of the new node that aren't in the existing node.
            if let Some(new_node) = new_node {
                for (name, new_child) in &new_node.child_components {
                    if existing_components_by_parent
                        .contains_key(&Some((internal_id, name.clone())))
                    {
                        continue;
                    }
                    stack.push((
                        path.join(name.clone()),
                        Some((internal_id, name.clone())),
                        None,
                        Some(new_child),
                    ));
                }
            }
        }
        Ok(diffs)
    }

    async fn create_component(
        &mut self,
        metadata: ComponentMetadata,
    ) -> anyhow::Result<(InternalId, ComponentDiff)> {
        let is_root = metadata.component_type.is_root();
        let document_id = SystemMetadataModel::new_global(self.tx)
            .insert(&COMPONENTS_TABLE, metadata.try_into()?)
            .await?;
        let component_id = if is_root {
            ComponentId::Root
        } else {
            ComponentId::Child(document_id.internal_id())
        };

        initialize_application_system_table(
            self.tx,
            &SchemasTable,
            component_id.into(),
            &DEFAULT_TABLE_NUMBERS,
        )
        .await?;
        for table in component_system_tables() {
            initialize_application_system_table(
                self.tx,
                table,
                component_id.into(),
                &DEFAULT_TABLE_NUMBERS,
            )
            .await?;
        }

        // TODO: Diff crons.

        Ok((document_id.internal_id(), ComponentDiff::Create))
    }

    async fn modify_component(
        &mut self,
        existing: &ParsedDocument<ComponentMetadata>,
        new_metadata: ComponentMetadata,
    ) -> anyhow::Result<(InternalId, ComponentDiff)> {
        SystemMetadataModel::new_global(self.tx)
            .replace(existing.id(), new_metadata.try_into()?)
            .await?;
        // TODO: Diff crons.
        Ok((existing.id().internal_id(), ComponentDiff::Modify))
    }

    async fn delete_component(
        &mut self,
        existing: &ParsedDocument<ComponentMetadata>,
    ) -> anyhow::Result<(InternalId, ComponentDiff)> {
        // TODO: Diff crons.
        // TODO: Delete the component's system tables.
        SystemMetadataModel::new_global(self.tx)
            .delete(existing.id())
            .await?;
        Ok((existing.id().internal_id(), ComponentDiff::Delete))
    }
}

pub enum ComponentDiff {
    Create,
    Modify,
    Delete,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum SerializedComponentDiff {
    Create,
    Modify,
    Delete,
}

impl TryFrom<ComponentDiff> for SerializedComponentDiff {
    type Error = anyhow::Error;

    fn try_from(value: ComponentDiff) -> Result<Self, Self::Error> {
        Ok(match value {
            ComponentDiff::Create => Self::Create,
            ComponentDiff::Modify => Self::Modify,
            ComponentDiff::Delete => Self::Delete,
        })
    }
}
