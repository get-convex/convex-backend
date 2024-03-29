//! Code for interacting with our metrics logging
#![feature(lazy_cell)]
#![feature(iter_intersperse)]
#![feature(try_blocks)]
#![feature(min_specialization)]
#![feature(type_alias_impl_trait)]
#![feature(impl_trait_in_assoc_type)]

use std::{
    env,
    sync::{
        LazyLock,
        Once,
    },
};

use semver::Version;

mod macros;
mod metrics;
mod progress;
mod reporting;
mod tags;
mod timer;

pub use crate::{
    macros::*,
    metrics::*,
    progress::ProgressCounter,
    reporting::{
        get_desc,
        log_counter,
        log_counter_with_tags,
        log_distribution,
        log_distribution_with_tags,
        log_gauge,
        log_gauge_with_tags,
    },
    tags::*,
    timer::{
        CancelableTimer,
        StatusTimer,
        Timer,
    },
};

static SERVER_VERSION: LazyLock<Option<Version>> = LazyLock::new(|| {
    // Use the version baked in at compile time.
    // In dev/test, use a fallback runtime-set version
    let compile_time = option_env!("CONVEX_RELEASE_VERSION");
    let runtime = env::var("CONVEX_RELEASE_VERSION_DEV").ok();
    compile_time.or(runtime.as_deref()).and_then(|s| {
        if s == "dev" {
            return None;
        }
        let v = Version::parse(s).unwrap_or_else(|_| panic!("Invalid version: {}", s));
        Some(v)
    })
});

fn initialize_server_version() -> String {
    let version_str = SERVER_VERSION
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "0.0.0".to_owned());
    INIT_VERSION_GAUGE.call_once(|| {
        CONVEX_BINARY_VERSIONS_TOTAL
            .with_label_values(&[&SERVICE_NAME, &version_str])
            .set(1.0);
    });
    version_str
}
pub static SERVER_VERSION_STR: LazyLock<String> = LazyLock::new(initialize_server_version);

pub const COMPILED_REVISION: &str = env!("VERGEN_GIT_SHA");

register_convex_gauge!(
    CONVEX_BINARY_VERSIONS_TOTAL,
    "Gauge representing the existence of a certain process at a certain version, as indicated in \
     the labels",
    &["binary", "version"]
);
static INIT_VERSION_GAUGE: Once = Once::new();
