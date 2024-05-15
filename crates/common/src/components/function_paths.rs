use serde::{
    Deserialize,
    Serialize,
};
use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};
use value::heap_size::HeapSize;

use super::{
    component_definition_path::ComponentDefinitionPath,
    ComponentId,
};

pub struct ComponentDefinitionFunctionPath {
    pub component: ComponentDefinitionPath,
    pub udf_path: UdfPath,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct ComponentFunctionPath {
    pub component: ComponentId,
    pub udf_path: UdfPath,
}

impl ComponentFunctionPath {
    pub fn as_root_udf_path(&self) -> anyhow::Result<&UdfPath> {
        anyhow::ensure!(self.component.is_root());
        Ok(&self.udf_path)
    }

    pub fn into_root_udf_path(self) -> anyhow::Result<UdfPath> {
        anyhow::ensure!(self.component.is_root());
        Ok(self.udf_path)
    }

    pub fn canonicalize(self) -> CanonicalizedComponentFunctionPath {
        CanonicalizedComponentFunctionPath {
            component: self.component,
            udf_path: self.udf_path.canonicalize(),
        }
    }

    pub fn debug_str(&self) -> String {
        if !self.component.is_root() {
            tracing::warn!("ComponentFunctionPath::debug_str called on non-root path");
        }
        format!("{:?}", self.udf_path)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SerializedComponentFunctionPath {
    pub component: Option<String>,
    pub udf_path: String,
}

impl TryFrom<SerializedComponentFunctionPath> for ComponentFunctionPath {
    type Error = anyhow::Error;

    fn try_from(p: SerializedComponentFunctionPath) -> anyhow::Result<Self> {
        Ok(Self {
            component: match p.component {
                Some(component) => ComponentId::Child(component.parse()?),
                None => ComponentId::Root,
            },
            udf_path: p.udf_path.parse()?,
        })
    }
}

impl TryFrom<ComponentFunctionPath> for SerializedComponentFunctionPath {
    type Error = anyhow::Error;

    fn try_from(p: ComponentFunctionPath) -> anyhow::Result<Self> {
        Ok(Self {
            component: match p.component {
                ComponentId::Root => None,
                ComponentId::Child(component) => Some(component.to_string()),
            },
            udf_path: p.udf_path.to_string(),
        })
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CanonicalizedComponentFunctionPath {
    pub component: ComponentId,
    pub udf_path: CanonicalizedUdfPath,
}

impl CanonicalizedComponentFunctionPath {
    pub fn as_root_udf_path(&self) -> anyhow::Result<&CanonicalizedUdfPath> {
        anyhow::ensure!(self.component.is_root());
        Ok(&self.udf_path)
    }

    pub fn into_root_udf_path(self) -> anyhow::Result<CanonicalizedUdfPath> {
        anyhow::ensure!(self.component.is_root());
        Ok(self.udf_path)
    }

    pub fn debug_str(&self) -> String {
        if !self.component.is_root() {
            tracing::warn!("ComponentFunctionPath::debug_str called on non-root path");
        }
        format!("{:?}", self.udf_path)
    }
}

impl From<CanonicalizedComponentFunctionPath> for ComponentFunctionPath {
    fn from(p: CanonicalizedComponentFunctionPath) -> Self {
        Self {
            component: p.component,
            udf_path: p.udf_path.into(),
        }
    }
}

impl HeapSize for CanonicalizedComponentFunctionPath {
    fn heap_size(&self) -> usize {
        self.udf_path.heap_size()
    }
}
