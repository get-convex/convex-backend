use value::InternalDocumentId;

mod component_definition_path;
mod component_path;
mod function_paths;

pub use self::{
    component_definition_path::ComponentDefinitionPath,
    component_path::{
        ComponentName,
        ComponentPath,
    },
    function_paths::{
        CanonicalizedComponentFunctionPath,
        ComponentDefinitionFunctionPath,
        ComponentFunctionPath,
    },
};

// Globally unique system-assigned ID for a component.
pub enum ComponentId {
    Root,
    Child(InternalDocumentId),
}
