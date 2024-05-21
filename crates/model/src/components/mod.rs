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
        CanonicalizedComponentModulePath,
        ComponentFunctionPath,
        ComponentId,
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
use sync_types::CanonicalizedUdfPath;
use value::identifier::Identifier;

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
        reference: &Reference,
    ) -> anyhow::Result<Resource> {
        let result = match reference {
            Reference::ComponentArgument { attributes } => {
                let attribute = match &attributes[..] {
                    [attribute] => attribute,
                    _ => anyhow::bail!("Nested component argument references unsupported"),
                };
                let component = BootstrapComponentsModel::new(self.tx)
                    .load_component(component_id)
                    .await?
                    .ok_or_else(|| {
                        ErrorMetadata::bad_request(
                            "InvalidReference",
                            format!("Component {:?} not found", component_id),
                        )
                    })?;
                let ComponentType::ChildComponent { ref args, .. } = component.component_type
                else {
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
                let definition_id = BootstrapComponentsModel::new(self.tx)
                    .component_definition(component_id)
                    .await?;
                let canonicalized = udf_path.clone().canonicalize();
                let module_path = CanonicalizedComponentModulePath {
                    component: definition_id,
                    module_path: canonicalized.module().clone(),
                };
                let module_metadata = ModuleModel::new(self.tx)
                    .get_metadata(module_path)
                    .await?
                    .ok_or_else(|| {
                        ErrorMetadata::bad_request(
                            "InvalidReference",
                            format!("Module {:?} not found", udf_path.module()),
                        )
                    })?;
                let analyze_result = module_metadata
                    .analyze_result
                    .as_ref()
                    .context("Module missing analyze result?")?;
                let function_found = analyze_result
                    .functions
                    .iter()
                    .any(|f| &f.name == canonicalized.function_name());
                if !function_found {
                    anyhow::bail!(ErrorMetadata::bad_request(
                        "InvalidReference",
                        format!(
                            "Function {:?} not found in {:?}",
                            udf_path.function_name(),
                            udf_path.module()
                        ),
                    ));
                }
                let path = ComponentFunctionPath {
                    component: component_id,
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
                        root_component.id().internal_id()
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
                let child_id = ComponentId::Child(child_component.id().internal_id());
                self.resolve_export(child_id, attributes).await?
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
        let definition = m.load_definition(definition_id).await?;

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
                    let exported_resource = self.resolve(component_id, reference).await?;
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
        let component = m.load_component(component_id).await?.ok_or_else(|| {
            ErrorMetadata::bad_request(
                "InvalidReference",
                format!("Component {:?} not found", component_id),
            )
        })?;
        let definition_id = m.component_definition(component_id).await?;
        let definition = m.load_definition(definition_id).await?;

        let mut result = BTreeMap::new();

        if let ComponentType::ChildComponent { ref args, .. } = component.component_type {
            for (name, resource) in args {
                let reference = Reference::ComponentArgument {
                    attributes: vec![name.clone()],
                };
                result.insert(reference, resource.clone());
            }
        }

        let module_metadata = ModuleModel::new(self.tx)
            .get_application_metadata(definition_id)
            .await?;
        for module in module_metadata {
            let analyze_result = module
                .analyze_result
                .as_ref()
                .context("Module missing analyze result?")?;
            for function in &analyze_result.functions {
                let udf_path =
                    CanonicalizedUdfPath::new(module.path.clone(), function.name.clone()).strip();
                result.insert(
                    Reference::Function(udf_path.clone()),
                    Resource::Function(ComponentFunctionPath {
                        component: component_id,
                        udf_path,
                    }),
                );
            }
        }

        for instantiation in &definition.child_components {
            let parent = (component.id().internal_id(), instantiation.name.clone());
            let child_component = BootstrapComponentsModel::new(self.tx)
                .component_in_parent(Some(parent))
                .await?
                .context("Missing child component")?;
            let child_component_id = ComponentId::Child(child_component.id().internal_id());
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

        Ok(result)
    }

    pub async fn preload_exported_resources(
        &mut self,
        component_id: ComponentId,
    ) -> anyhow::Result<BTreeMap<Vec<Identifier>, Resource>> {
        let mut m = BootstrapComponentsModel::new(self.tx);
        let definition_id = m.component_definition(component_id).await?;
        let definition = m.load_definition(definition_id).await?;

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
                        let resource = self.resolve(component_id, reference).await?;
                        result.insert(new_path, resource);
                    },
                }
            }
        }

        Ok(result)
    }
}
