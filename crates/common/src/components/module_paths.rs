use sync_types::CanonicalizedModulePath;

use super::ComponentDefinitionId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CanonicalizedComponentModulePath {
    pub component: ComponentDefinitionId,
    pub module_path: CanonicalizedModulePath,
}

impl CanonicalizedComponentModulePath {
    pub fn is_root(&self) -> bool {
        self.component.is_root()
    }

    pub fn as_root_module_path(&self) -> anyhow::Result<&CanonicalizedModulePath> {
        anyhow::ensure!(self.component.is_root());
        Ok(&self.module_path)
    }

    pub fn into_root_module_path(self) -> anyhow::Result<CanonicalizedModulePath> {
        anyhow::ensure!(self.component.is_root());
        Ok(self.module_path)
    }
}
