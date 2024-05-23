use std::sync::LazyLock;

use cmd_util::env::env_config;
use value::{
    InternalId,
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
    },
    module_paths::CanonicalizedComponentModulePath,
    reference::Reference,
    resource::{
        Resource,
        SerializedResource,
    },
};

pub static COMPONENTS_ENABLED: LazyLock<bool> =
    LazyLock::new(|| env_config("COMPONENTS_ENABLED", false));

pub fn require_components_enabled() -> anyhow::Result<()> {
    if !*COMPONENTS_ENABLED {
        anyhow::bail!("Components are not enabled, set COMPONENTS_ENABLED=true to enable them.");
    }
    Ok(())
}

// Globally unique system-assigned ID for a component.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ComponentId {
    Root,
    Child(InternalId),
}

impl ComponentId {
    pub fn is_root(&self) -> bool {
        matches!(self, ComponentId::Root)
    }
}

impl From<ComponentId> for TableNamespace {
    fn from(value: ComponentId) -> Self {
        if *COMPONENTS_ENABLED {
            match value {
                ComponentId::Root => TableNamespace::RootComponent,
                ComponentId::Child(id) => TableNamespace::ByComponent(id),
            }
        } else {
            match value {
                ComponentId::Root => TableNamespace::Global,
                ComponentId::Child(_id) => TableNamespace::Global,
            }
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
    Child(InternalId),
}

impl ComponentDefinitionId {
    pub fn is_root(&self) -> bool {
        matches!(self, ComponentDefinitionId::Root)
    }
}

impl From<ComponentDefinitionId> for TableNamespace {
    fn from(value: ComponentDefinitionId) -> Self {
        if *COMPONENTS_ENABLED {
            match value {
                ComponentDefinitionId::Root => TableNamespace::RootComponentDefinition,
                ComponentDefinitionId::Child(id) => TableNamespace::ByComponentDefinition(id),
            }
        } else {
            match value {
                ComponentDefinitionId::Root => TableNamespace::Global,
                ComponentDefinitionId::Child(_id) => TableNamespace::Global,
            }
        }
    }
}
