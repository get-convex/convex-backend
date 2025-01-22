use std::sync::LazyLock;

use common::types::EnvVarName;

pub static CONVEX_ORIGIN: LazyLock<EnvVarName> = LazyLock::new(|| {
    "CONVEX_CLOUD_URL"
        .parse()
        .expect("CONVEX_CLOUD_URL should be a valid EnvVarName")
});

pub static CONVEX_SITE: LazyLock<EnvVarName> = LazyLock::new(|| {
    "CONVEX_SITE_URL"
        .parse()
        .expect("CONVEX_SITE_URL should be a valid EnvVarName")
});
