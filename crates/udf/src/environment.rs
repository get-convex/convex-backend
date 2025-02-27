use std::{
    collections::BTreeMap,
    sync::LazyLock,
};

use common::{
    document::ParsedDocument,
    http::RequestDestination,
    runtime::Runtime,
    types::{
        EnvVarName,
        EnvVarValue,
    },
};
use database::Transaction;
use model::canonical_urls::{
    types::CanonicalUrl,
    CanonicalUrlsModel,
};

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

// Definitions used throughout the codebase:
// - `default_system_env_vars` means the .convex.cloud/.convex.site urls (or
//   otherwise statically configured urls),
// - `system_env_var_overrides` means the canonical urls,
// - `system_env_vars` means the merged `default_system_env_vars` and
//   `system_env_var_overrides`.
// - `user_environment_variables` means user-defined env vars in the dashboard.
// - `environment_variables` means the merged `system_env_vars` and
//   `user_environment_variables`.
// In most cases, function executions use `environment_variables`, although
// often they are computed at different times and merged later.
pub async fn system_env_vars<RT: Runtime>(
    tx: &mut Transaction<RT>,
    default_system_env_vars: BTreeMap<EnvVarName, EnvVarValue>,
) -> anyhow::Result<BTreeMap<EnvVarName, EnvVarValue>> {
    let system_env_var_overrides = system_env_var_overrides(tx).await?;
    let mut system_env_vars = default_system_env_vars;
    system_env_vars.extend(system_env_var_overrides);
    Ok(system_env_vars)
}

pub async fn system_env_var_overrides<RT: Runtime>(
    tx: &mut Transaction<RT>,
) -> anyhow::Result<BTreeMap<EnvVarName, EnvVarValue>> {
    let canonical_urls = CanonicalUrlsModel::new(tx).get_canonical_urls().await?;
    parse_system_env_var_overrides(canonical_urls)
}

pub fn parse_system_env_var_overrides(
    canonical_urls: BTreeMap<RequestDestination, ParsedDocument<CanonicalUrl>>,
) -> anyhow::Result<BTreeMap<EnvVarName, EnvVarValue>> {
    let mut system_env_var_overrides = BTreeMap::new();
    for (request_destination, canonical_url) in canonical_urls {
        let env_var_name = match request_destination {
            RequestDestination::ConvexCloud => CONVEX_ORIGIN.clone(),
            RequestDestination::ConvexSite => CONVEX_SITE.clone(),
        };
        system_env_var_overrides.insert(env_var_name, canonical_url.url.parse()?);
    }
    Ok(system_env_var_overrides)
}
