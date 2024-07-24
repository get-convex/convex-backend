pub mod config;
pub mod file_based_routing;
pub mod handles;
pub mod type_checking;
pub mod types;

use std::collections::BTreeMap;

use anyhow::Context;
use async_recursion::async_recursion;
use common::{
    bootstrap_model::components::{
        definition::ComponentExport,
        ComponentType,
    },
    components::{
        CanonicalizedComponentFunctionPath,
        ComponentId,
        ComponentPath,
        Reference,
        Resource,
    },
    runtime::Runtime,
};
use database::{
    BootstrapComponentsModel,
    Transaction,
};
use errors::ErrorMetadata;
use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};
use value::{
    identifier::Identifier,
    TableNamespace,
};

use crate::modules::ModuleModel;

pub struct ComponentsModel<'a, RT: Runtime> {
    pub tx: &'a mut Transaction<RT>,
}

impl<'a, RT: Runtime> ComponentsModel<'a, RT> {
    pub fn new(tx: &'a mut Transaction<RT>) -> Self {
        Self { tx }
    }

    #[async_recursion]
    pub async fn resolve(
        &mut self,
        component_id: ComponentId,
        current_udf_path: Option<UdfPath>,
        reference: &Reference,
    ) -> anyhow::Result<Resource> {
        let result = match reference {
            Reference::ComponentArgument { attributes } => {
                let attribute = match &attributes[..] {
                    [attribute] => attribute,
                    _ => anyhow::bail!("Nested component argument references unsupported"),
                };
                let component_type = BootstrapComponentsModel::new(self.tx)
                    .load_component_type(component_id)
                    .await?;
                let ComponentType::ChildComponent { ref args, .. } = component_type else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        "Can't use an argument reference in the app"
                    ))
                };
                let resource = args.get(attribute).ok_or_else(|| {
                    ErrorMetadata::bad_request(
                        "InvalidReference",
                        format!("Component argument '{attribute}' not found"),
                    )
                })?;
                resource.clone()
            },
            Reference::Function(udf_path) => {
                let mut m = BootstrapComponentsModel::new(self.tx);
                let component_path = m.get_component_path(component_id).await?;

                let path = CanonicalizedComponentFunctionPath {
                    component: component_path,
                    udf_path: udf_path.clone(),
                };
                Resource::Function(path)
            },
            Reference::ChildComponent {
                component: child_component,
                attributes,
            } => {
                let mut m = BootstrapComponentsModel::new(self.tx);
                let internal_id = match component_id {
                    ComponentId::Root => {
                        let root_component = m
                            .root_component()
                            .await?
                            .context("Missing root component")?;
                        root_component.id().into()
                    },
                    ComponentId::Child(id) => id,
                };
                let parent = (internal_id, child_component.clone());
                let child_component =
                    m.component_in_parent(Some(parent)).await?.ok_or_else(|| {
                        ErrorMetadata::bad_request(
                            "InvalidReference",
                            format!("Child component {:?} not found", child_component),
                        )
                    })?;
                let child_id = ComponentId::Child(child_component.id().into());
                self.resolve_export(child_id, attributes).await?
            },
            Reference::CurrentSystemUdfInComponent {
                component_id: component_by_id,
            } => {
                if !component_id.is_root() {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        "CurrentSystemUdfInComponent only available in root component"
                    ));
                }
                let Some(current_udf_path) = current_udf_path else {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        "CurrentSystemUdfInComponent must be called from a UDF",
                    ));
                };
                if !current_udf_path.is_system() {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        "CurrentSystemUdfInComponent must be called from a system UDF",
                    ));
                }
                let mut m = BootstrapComponentsModel::new(self.tx);
                // TODO(lee) this should work for disconnected components too.
                let component_path = m
                    .get_component_path(ComponentId::Child(*component_by_id))
                    .await?;
                Resource::Function(CanonicalizedComponentFunctionPath {
                    component: component_path,
                    udf_path: current_udf_path.canonicalize(),
                })
            },
        };
        Ok(result)
    }

    #[async_recursion]
    pub async fn resolve_export(
        &mut self,
        component_id: ComponentId,
        attributes: &[Identifier],
    ) -> anyhow::Result<Resource> {
        let mut m = BootstrapComponentsModel::new(self.tx);
        let definition_id = m.component_definition(component_id).await?;
        let definition = m.load_definition_metadata(definition_id).await?;

        let mut current = &definition.exports;
        let mut attribute_iter = attributes.iter();
        while let Some(attribute) = attribute_iter.next() {
            let export = current.get(attribute).ok_or_else(|| {
                ErrorMetadata::bad_request(
                    "InvalidReference",
                    format!("Export '{attribute}' not found"),
                )
            })?;
            match export {
                ComponentExport::Branch(ref next) => {
                    current = next;
                    continue;
                },
                ComponentExport::Leaf(ref reference) => {
                    let exported_resource = self.resolve(component_id, None, reference).await?;
                    if !attribute_iter.as_slice().is_empty() {
                        anyhow::bail!("Component references currently unsupported");
                    }
                    return Ok(exported_resource);
                },
            }
        }
        anyhow::bail!("Intermediate export references unsupported");
    }

    pub async fn preload_resources(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<BTreeMap<Reference, Resource>> {
        let mut m = BootstrapComponentsModel::new(self.tx);
        let component = m.load_component(component_id).await?;
        let component_type = m.load_component_type(component_id).await?;
        let definition_id = m.component_definition(component_id).await?;
        let definition = m.load_definition_metadata(definition_id).await?;
        let component_path = m.get_component_path(component_id).await?;

        let mut result = BTreeMap::new();

        if let ComponentType::ChildComponent { ref args, .. } = component_type {
            for (name, resource) in args {
                let reference = Reference::ComponentArgument {
                    attributes: vec![name.clone()],
                };
                result.insert(reference, resource.clone());
            }
        }

        let module_metadata = ModuleModel::new(self.tx)
            .get_application_metadata(component_id)
            .await?;
        for module in module_metadata {
            let Some(ref analyze_result) = module.analyze_result else {
                continue;
            };
            for function in &analyze_result.functions {
                let udf_path =
                    CanonicalizedUdfPath::new(module.path.clone(), function.name.clone());
                let function_path = CanonicalizedComponentFunctionPath {
                    component: component_path.clone(),
                    udf_path: udf_path.clone(),
                };
                result.insert(
                    Reference::Function(udf_path),
                    Resource::Function(function_path),
                );
            }
        }

        if let Some(component) = component {
            for instantiation in &definition.child_components {
                let parent = (component.id().into(), instantiation.name.clone());
                let child_component = BootstrapComponentsModel::new(self.tx)
                    .component_in_parent(Some(parent))
                    .await?
                    .context("Missing child component")?;
                let child_component_id = ComponentId::Child(child_component.id().into());
                for (attributes, resource) in
                    self.preload_exported_resources(child_component_id).await?
                {
                    let reference = Reference::ChildComponent {
                        component: instantiation.name.clone(),
                        attributes,
                    };
                    result.insert(reference, resource);
                }
            }
        }

        Ok(result)
    }

    pub async fn preload_exported_resources(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<BTreeMap<Vec<Identifier>, Resource>> {
        let mut m = BootstrapComponentsModel::new(self.tx);
        let definition_id = m.component_definition(component_id).await?;
        let definition = m.load_definition_metadata(definition_id).await?;

        let mut stack = vec![(vec![], &definition.exports)];
        let mut result = BTreeMap::new();
        while let Some((path, internal_node)) = stack.pop() {
            for (name, export) in internal_node {
                match export {
                    ComponentExport::Branch(ref children) => {
                        let mut new_path = path.clone();
                        new_path.push(name.clone());
                        stack.push((new_path, children));
                    },
                    ComponentExport::Leaf(ref reference) => {
                        let mut new_path = path.clone();
                        new_path.push(name.clone());
                        let resource = self.resolve(component_id, None, reference).await?;
                        result.insert(new_path, resource);
                    },
                }
            }
        }

        Ok(result)
    }

    pub async fn get_component_path_for_namespace(
        &mut self,
        namespace: TableNamespace,
    ) -> anyhow::Result<ComponentPath> {
        let component_id = match namespace {
            TableNamespace::Global => ComponentId::Root,
            TableNamespace::ByComponent(id) => ComponentId::Child(id),
        };
        BootstrapComponentsModel::new(self.tx)
            .get_component_path(component_id)
            .await
    }
}

#[cfg(test)]
mod tests {
    use common::{
        bootstrap_model::index::IndexMetadata,
        components::{
            CanonicalizedComponentModulePath,
            ComponentId,
        },
    };
    use database::{
        defaults::SystemTable,
        test_helpers::DbFixtures,
        IndexModel,
        SystemMetadataModel,
        COMPONENTS_TABLE,
    };
    use keybroker::Identity;
    use runtime::testing::TestRuntime;
    use value::obj;

    use crate::{
        modules::{
            ModuleModel,
            ModulesTable,
        },
        DEFAULT_TABLE_NUMBERS,
    };

    #[convex_macro::test_runtime]
    async fn test_create_and_use_module_table(rt: TestRuntime) -> anyhow::Result<()> {
        let DbFixtures { db, .. } = DbFixtures::new(&rt).await?;

        let mut tx = db.begin(Identity::system()).await?;
        let id = SystemMetadataModel::new_global(&mut tx)
            .insert(&COMPONENTS_TABLE, obj!()?)
            .await?;
        let component_id = ComponentId::Child(id.into());

        let namespace = component_id.into();
        let table = ModulesTable;
        let is_new = tx
            .create_system_table(
                namespace,
                table.table_name(),
                DEFAULT_TABLE_NUMBERS.get(table.table_name()).cloned(),
            )
            .await?;
        assert!(is_new);

        for index in table.indexes() {
            let index_metadata = IndexMetadata::new_enabled(index.name, index.fields);
            IndexModel::new(&mut tx)
                .add_system_index(namespace, index_metadata)
                .await?;
        }

        let m = ModuleModel::new(&mut tx)
            .get_metadata(CanonicalizedComponentModulePath {
                component: component_id,
                module_path: "a.js".parse()?,
            })
            .await?;
        assert!(m.is_none());

        Ok(())
    }
}
