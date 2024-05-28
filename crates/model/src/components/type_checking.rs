use std::collections::BTreeMap;

use common::{
    bootstrap_model::components::definition::{
        ComponentArgument::Value,
        ComponentArgumentValidator,
        ComponentDefinitionType,
        ComponentExport,
    },
    components::{
        ComponentDefinitionPath,
        ComponentFunctionPath,
        ComponentName,
        ComponentPath,
        Reference,
        Resource,
    },
    schemas::validator::ValidationError,
};
use thiserror::Error;
use value::{
    identifier::Identifier,
    TableMapping,
    TableNamespace,
    VirtualTableMapping,
};

use super::types::EvaluatedComponentDefinition;

#[derive(Debug)]
pub struct CheckedComponent {
    pub definition_path: ComponentDefinitionPath,
    pub component_path: ComponentPath,

    pub args: BTreeMap<Identifier, Resource>,
    pub child_components: BTreeMap<ComponentName, CheckedComponent>,
    pub exports: BTreeMap<Identifier, CheckedExport>,
}

#[derive(Debug)]
pub enum CheckedExport {
    Branch(BTreeMap<Identifier, CheckedExport>),
    Leaf(Resource),
}

pub struct TypecheckContext<'a> {
    evaluated_definitions: &'a BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,
}

impl<'a> TypecheckContext<'a> {
    pub fn new(
        definitions: &'a BTreeMap<ComponentDefinitionPath, EvaluatedComponentDefinition>,
    ) -> Self {
        Self {
            evaluated_definitions: definitions,
        }
    }

    pub fn instantiate_root(&self) -> Result<CheckedComponent, TypecheckError> {
        let definition_path = ComponentDefinitionPath::root();
        let component_path = ComponentPath::root();
        let args = BTreeMap::new();
        self.instantiate(definition_path, component_path, args)
    }

    pub fn instantiate(
        &self,
        definition_path: ComponentDefinitionPath,
        component_path: ComponentPath,
        args: BTreeMap<Identifier, Resource>,
    ) -> Result<CheckedComponent, TypecheckError> {
        let evaluated = self
            .evaluated_definitions
            .get(&definition_path)
            .ok_or_else(|| TypecheckError::MissingComponentDefinition {
                definition_path: definition_path.clone(),
            })?;

        let mut builder = CheckedComponentBuilder::check_args(
            &definition_path,
            &component_path,
            evaluated,
            args,
        )?;

        // Instantiate our children in order, since we'd like to support one
        // instantiation depending on another (e.g. passing a function reference
        // from one component as an argument to another).
        for instantiation in &evaluated.definition.child_components {
            let mut resolved_args = BTreeMap::new();
            for (name, parameter) in &instantiation.args {
                let resource = match parameter {
                    Value(value) => Resource::Value(value.clone()),
                };
                resolved_args.insert(name.clone(), resource);
            }
            let child_component_path = component_path.push(instantiation.name.clone());
            let child_component = self.instantiate(
                instantiation.path.clone(),
                child_component_path,
                resolved_args,
            )?;
            builder.insert_child_component(instantiation.name.clone(), child_component)?;
        }

        // Finally, resolve our exports and build the component.
        let component = builder.check_exports(&evaluated.definition.exports)?;

        Ok(component)
    }
}

struct CheckedComponentBuilder<'a> {
    definition_path: &'a ComponentDefinitionPath,
    component_path: &'a ComponentPath,
    evaluated: &'a EvaluatedComponentDefinition,

    // Phase 1: Arguments are checked immediately at construction time.
    args: BTreeMap<Identifier, Resource>,

    // Phase 2: The layer above adds in child components one at a time, and instantiating a child
    // component may depend on arguments or previous child components.
    child_components: BTreeMap<ComponentName, CheckedComponent>,
    //
    // Phase 3: The layer above finalizes via `build`, passing in exports, which may depend on args
    // or any child component.
}

impl<'a> CheckedComponentBuilder<'a> {
    pub fn check_args(
        definition_path: &'a ComponentDefinitionPath,
        component_path: &'a ComponentPath,
        evaluated: &'a EvaluatedComponentDefinition,
        args: BTreeMap<Identifier, Resource>,
    ) -> Result<Self, TypecheckError> {
        match &evaluated.definition.definition_type {
            ComponentDefinitionType::App => {
                if !args.is_empty() {
                    return Err(TypecheckError::InvalidComponentArgumentCount {
                        component_path: component_path.clone(),
                        expected: 0,
                        actual: args.len(),
                    });
                }
            },
            ComponentDefinitionType::ChildComponent {
                args: arg_validators,
                ..
            } => {
                for (arg_name, arg_value) in &args {
                    let validator = arg_validators.get(arg_name).ok_or_else(|| {
                        TypecheckError::InvalidComponentArgumentName {
                            component_path: component_path.clone(),
                            arg_name: arg_name.clone(),
                        }
                    })?;
                    match (arg_value, validator) {
                        (
                            Resource::Value(ref value),
                            ComponentArgumentValidator::Value(ref validator),
                        ) => {
                            // TODO(CX-6540): Remove hack where we pass in empty mappings.
                            let table_mapping =
                                TableMapping::new().namespace(TableNamespace::Global);
                            let virtual_table_mapping = VirtualTableMapping::new();
                            validator
                                .check_value(value, &table_mapping, &virtual_table_mapping)
                                .map_err(|validator_error| {
                                    TypecheckError::InvalidComponentArgument {
                                        component_path: component_path.clone(),
                                        arg_name: arg_name.clone(),
                                        validator_error,
                                    }
                                })?;
                        },
                        (Resource::Function { .. }, _) => {
                            return Err(TypecheckError::Unsupported("function references"))
                        },
                    }
                }
            },
        }
        Ok(Self {
            definition_path,
            component_path,
            evaluated,

            args,
            child_components: BTreeMap::new(),
        })
    }

    fn insert_child_component(
        &mut self,
        name: ComponentName,
        component: CheckedComponent,
    ) -> Result<(), TypecheckError> {
        if self.child_components.contains_key(&name) {
            return Err(TypecheckError::DuplicateChildComponent {
                definition_path: self.definition_path.clone(),
                name: name.into(),
            });
        }
        self.child_components.insert(name, component);
        Ok(())
    }

    fn check_exports(
        self,
        exports: &BTreeMap<Identifier, ComponentExport>,
    ) -> Result<CheckedComponent, TypecheckError> {
        let exports = self.resolve_exports(exports)?;
        Ok(CheckedComponent {
            definition_path: self.definition_path.clone(),
            component_path: self.component_path.clone(),
            args: self.args,
            child_components: self.child_components,
            exports,
        })
    }

    fn resolve_exports(
        &self,
        exports: &BTreeMap<Identifier, ComponentExport>,
    ) -> Result<BTreeMap<Identifier, CheckedExport>, TypecheckError> {
        let mut result = BTreeMap::new();
        for (name, export) in exports {
            let node = match export {
                ComponentExport::Branch(ref exports) => {
                    CheckedExport::Branch(self.resolve_exports(exports)?)
                },
                ComponentExport::Leaf(ref reference) => {
                    let resource = self.resolve(reference)?;
                    CheckedExport::Leaf(resource)
                },
            };
            result.insert(name.clone(), node);
        }
        Ok(result)
    }

    fn resolve(&self, reference: &Reference) -> Result<Resource, TypecheckError> {
        let unresolved_err = || TypecheckError::UnresolvedExport {
            definition_path: self.definition_path.clone(),
            reference: reference.clone(),
        };
        let result = match reference {
            Reference::ComponentArgument { attributes } => {
                if attributes.len() != 1 {
                    return Err(TypecheckError::Unsupported("Nested argument references"));
                }
                self.args
                    .get(&attributes[0])
                    .ok_or_else(unresolved_err)?
                    .clone()
            },
            Reference::Function(path) => {
                let canonicalized = path.clone().canonicalize();
                let module = self
                    .evaluated
                    .functions
                    .get(canonicalized.module())
                    .ok_or_else(unresolved_err)?;
                let _function = module
                    .functions
                    .iter()
                    .find(|f| &f.name == canonicalized.function_name())
                    .ok_or_else(unresolved_err)?;
                Resource::Function(ComponentFunctionPath {
                    component: self.component_path.clone(),
                    udf_path: path.clone(),
                })
            },
            Reference::ChildComponent {
                component,
                attributes,
            } => {
                let child_component = self
                    .child_components
                    .get(component)
                    .ok_or_else(unresolved_err)?;
                child_component
                    .resolve_export(attributes)?
                    .ok_or_else(unresolved_err)?
            },
        };
        Ok(result)
    }
}

impl CheckedComponent {
    pub fn resolve_export(
        &self,
        attributes: &[Identifier],
    ) -> Result<Option<Resource>, TypecheckError> {
        let mut current = &self.exports;
        let mut attribute_iter = attributes.iter();
        while let Some(attribute) = attribute_iter.next() {
            let Some(export) = current.get(attribute) else {
                return Ok(None);
            };
            match export {
                CheckedExport::Branch(ref next) => {
                    current = next;
                    continue;
                },
                CheckedExport::Leaf(ref resource) => {
                    if !attribute_iter.as_slice().is_empty() {
                        return Err(TypecheckError::Unsupported("Component references"));
                    }
                    return Ok(Some(resource.clone()));
                },
            }
        }
        Err(TypecheckError::Unsupported(
            "Intermediate export references",
        ))
    }
}

#[derive(Error, Debug)]
pub enum TypecheckError {
    #[error("Component definition not found: {definition_path:?}")]
    MissingComponentDefinition {
        definition_path: ComponentDefinitionPath,
    },
    #[error(
        "Component {component_path:?} has {expected} parameters, but instantiation has {actual}"
    )]
    InvalidComponentArgumentCount {
        component_path: ComponentPath,
        expected: usize,
        actual: usize,
    },
    #[error("Component {component_path:?} has no parameter named {arg_name:?}")]
    InvalidComponentArgumentName {
        component_path: ComponentPath,
        arg_name: Identifier,
    },
    #[error(
        "Component {component_path:?} has an invalid value for argument {arg_name:?}: \
         {validator_error:?}"
    )]
    InvalidComponentArgument {
        component_path: ComponentPath,
        arg_name: Identifier,
        validator_error: ValidationError,
    },
    #[error(
        "Component {definition_path:?} has multiple child components with the same name {name:?}"
    )]
    DuplicateChildComponent {
        definition_path: ComponentDefinitionPath,
        name: Identifier,
    },
    #[error("Component {definition_path:?} has an unresolved export {reference:?}")]
    UnresolvedExport {
        definition_path: ComponentDefinitionPath,
        reference: Reference,
    },
    #[error("{0} currently unsupported")]
    Unsupported(&'static str),
}

mod json {
    use std::collections::BTreeMap;

    use common::components::SerializedResource;
    use serde::{
        Deserialize,
        Serialize,
    };

    use super::{
        CheckedComponent,
        CheckedExport,
    };

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SerializedCheckedComponent {
        definition_path: String,
        component_path: String,

        args: BTreeMap<String, SerializedResource>,
        child_components: BTreeMap<String, SerializedCheckedComponent>,
        exports: BTreeMap<String, SerializedCheckedExport>,
    }

    impl TryFrom<CheckedComponent> for SerializedCheckedComponent {
        type Error = anyhow::Error;

        fn try_from(value: CheckedComponent) -> Result<Self, Self::Error> {
            Ok(Self {
                definition_path: String::from(value.definition_path),
                component_path: String::from(value.component_path),
                args: value
                    .args
                    .into_iter()
                    .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
                child_components: value
                    .child_components
                    .into_iter()
                    .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
                exports: value
                    .exports
                    .into_iter()
                    .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
            })
        }
    }

    impl TryFrom<SerializedCheckedComponent> for CheckedComponent {
        type Error = anyhow::Error;

        fn try_from(value: SerializedCheckedComponent) -> Result<Self, Self::Error> {
            Ok(Self {
                definition_path: value.definition_path.parse()?,
                component_path: value.component_path.parse()?,
                args: value
                    .args
                    .into_iter()
                    .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
                child_components: value
                    .child_components
                    .into_iter()
                    .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
                exports: value
                    .exports
                    .into_iter()
                    .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                    .collect::<anyhow::Result<_>>()?,
            })
        }
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", tag = "type")]
    pub enum SerializedCheckedExport {
        Branch {
            children: BTreeMap<String, SerializedCheckedExport>,
        },
        Leaf {
            resource: SerializedResource,
        },
    }

    impl TryFrom<CheckedExport> for SerializedCheckedExport {
        type Error = anyhow::Error;

        fn try_from(value: CheckedExport) -> Result<Self, Self::Error> {
            Ok(match value {
                CheckedExport::Branch(branch) => Self::Branch {
                    children: branch
                        .into_iter()
                        .map(|(k, v)| Ok((String::from(k), v.try_into()?)))
                        .collect::<anyhow::Result<_>>()?,
                },
                CheckedExport::Leaf(leaf) => Self::Leaf {
                    resource: leaf.try_into()?,
                },
            })
        }
    }

    impl TryFrom<SerializedCheckedExport> for CheckedExport {
        type Error = anyhow::Error;

        fn try_from(value: SerializedCheckedExport) -> Result<Self, Self::Error> {
            Ok(match value {
                SerializedCheckedExport::Branch { children } => Self::Branch(
                    children
                        .into_iter()
                        .map(|(k, v)| Ok((k.parse()?, v.try_into()?)))
                        .collect::<anyhow::Result<_>>()?,
                ),
                SerializedCheckedExport::Leaf { resource } => Self::Leaf(resource.try_into()?),
            })
        }
    }
}
pub use self::json::{
    SerializedCheckedComponent,
    SerializedCheckedExport,
};
