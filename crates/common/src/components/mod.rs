use std::str::FromStr;

use value::{
    DeveloperDocumentId,
    TableNamespace,
};

mod component_definition_path;
mod component_path;
mod function_paths;
mod module_paths;
mod reference;
mod resource;

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
        ExportPath,
        PublicFunctionPath,
    },
    module_paths::CanonicalizedComponentModulePath,
    reference::Reference,
    resource::{
        Resource,
        SerializedResource,
    },
};

// Globally unique system-assigned ID for a component.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ComponentId {
    Root,
    Child(DeveloperDocumentId),
}

impl ComponentId {
    pub fn is_root(&self) -> bool {
        matches!(self, ComponentId::Root)
    }

    /// This component should be passed in somehow -- it's not always going to
    /// be Root.
    #[allow(non_snake_case)]
    pub const fn TODO() -> Self {
        ComponentId::Root
    }

    /// Component for tests where we need a user component.
    /// Ideally we could switch this to some other component with no test
    /// breakage.
    #[cfg(any(test, feature = "testing"))]
    pub const fn test_user() -> Self {
        ComponentId::Root
    }

    pub fn serialize_to_string(&self) -> Option<String> {
        match self {
            ComponentId::Root => None,
            ComponentId::Child(id) => Some(id.to_string()),
        }
    }

    pub fn deserialize_from_string(s: Option<&str>) -> anyhow::Result<Self> {
        match s {
            None => Ok(ComponentId::Root),
            Some(s) => Ok(ComponentId::Child(DeveloperDocumentId::from_str(s)?)),
        }
    }
}

impl From<ComponentId> for TableNamespace {
    fn from(value: ComponentId) -> Self {
        match value {
            ComponentId::Root => TableNamespace::root_component(),
            ComponentId::Child(id) => TableNamespace::ByComponent(id),
        }
    }
}

impl From<TableNamespace> for ComponentId {
    fn from(value: TableNamespace) -> Self {
        match value {
            TableNamespace::Global => ComponentId::Root,
            TableNamespace::ByComponent(id) => ComponentId::Child(id),
        }
    }
}

#[cfg(any(test, feature = "testing"))]
mod proptests {
    use proptest::prelude::*;

    use super::{
        ComponentDefinitionId,
        ComponentId,
    };

    impl Arbitrary for ComponentId {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            Just(ComponentId::Root).boxed()
        }
    }

    impl Arbitrary for ComponentDefinitionId {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            Just(ComponentDefinitionId::Root).boxed()
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ComponentDefinitionId {
    Root,
    Child(DeveloperDocumentId),
}

impl ComponentDefinitionId {
    pub fn is_root(&self) -> bool {
        matches!(self, ComponentDefinitionId::Root)
    }
}
