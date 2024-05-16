use anyhow::Context;
use async_recursion::async_recursion;
use common::{
    bootstrap_model::components::definition::ComponentExport,
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
                let resource = component.args.get(attribute).ok_or_else(|| {
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
}
