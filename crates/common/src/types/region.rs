use std::sync::{
    LazyLock,
    OnceLock,
};

use tuple_struct::tuple_struct_string;

tuple_struct_string!(RegionName);

impl Default for RegionName {
    fn default() -> RegionName {
        default_region().clone()
    }
}

static DEFAULT_REGION_NAME: LazyLock<RegionName> = LazyLock::new(|| "aws-us-east-1".into());
pub static TEST_REGION_NAME: LazyLock<RegionName> = LazyLock::new(|| "local".into());

// We are only able to determine what the default region should be at runtime,
// so we set the region in local_dev_bootstrap or setup_db, and then use the
// default_region function everywhere
static RUNTIME_DEFAULT_REGION: OnceLock<RegionName> = OnceLock::new();

pub fn set_test_region_as_default() -> anyhow::Result<()> {
    RUNTIME_DEFAULT_REGION
        .set(TEST_REGION_NAME.clone())
        .or_else(|region| {
            // Allow calling this multiple times if it's already set to the test region name
            if region == *TEST_REGION_NAME {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Default region already set to non-test region"
                ))
            }
        })
}

/// Returns the default region for the current environment.
pub fn default_region() -> &'static RegionName {
    RUNTIME_DEFAULT_REGION.get_or_init(|| DEFAULT_REGION_NAME.clone())
}
