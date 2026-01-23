use std::sync::LazyLock;

use tuple_struct::tuple_struct_string;

tuple_struct_string!(RegionName);

impl Default for RegionName {
    fn default() -> RegionName {
        default_region().clone()
    }
}

#[cfg_attr(any(test, feature = "testing"), allow(dead_code))]
static DEFAULT_REGION: LazyLock<RegionName> = LazyLock::new(|| "aws-us-east-1".into());
#[cfg_attr(not(any(test, feature = "testing")), allow(dead_code))]
static TEST_REGION: LazyLock<RegionName> = LazyLock::new(|| "local".into());

/// Returns the default region for the current environment.
pub fn default_region() -> &'static RegionName {
    #[cfg(any(test, feature = "testing"))]
    {
        &TEST_REGION
    }
    #[cfg(not(any(test, feature = "testing")))]
    {
        &DEFAULT_REGION
    }
}
