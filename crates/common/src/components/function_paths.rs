use sync_types::{
    CanonicalizedUdfPath,
    UdfPath,
};
use value::heap_size::HeapSize;

use super::{
    component_definition_path::ComponentDefinitionPath,
    CanonicalizedComponentModulePath,
    ComponentId,
};

pub struct ComponentDefinitionFunctionPath {
    pub component: ComponentDefinitionPath,
    pub udf_path: UdfPath,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    any(test, feature = "testing"),
    // Only define `Debug` in test builds so we don't accidentally
    // print these paths out when migrating to component-aware paths.
    derive(Debug, proptest_derive::Arbitrary)
)]
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
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(
    any(test, feature = "testing"),
    derive(Debug, proptest_derive::Arbitrary)
)]
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

    pub fn module(&self) -> CanonicalizedComponentModulePath {
        CanonicalizedComponentModulePath {
            component: self.component.clone(),
            module_path: self.udf_path.module().clone(),
        }
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
