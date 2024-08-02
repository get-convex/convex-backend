use sync_types::CanonicalizedModulePath;

use super::ComponentId;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
#[cfg_attr(any(test, feature = "testing"), derive(proptest_derive::Arbitrary))]
pub struct CanonicalizedComponentModulePath {
    pub component: ComponentId,
    pub module_path: CanonicalizedModulePath,
}

impl CanonicalizedComponentModulePath {
    pub fn is_root(&self) -> bool {
        self.component.is_root()
    }
}
