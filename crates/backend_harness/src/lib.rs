#![feature(exit_status_error)]
#![feature(try_blocks)]
#![feature(try_blocks_heterogeneous)]

/// Harness for spawning/testing a backend for a given package
mod metrics;
mod provision;

pub use provision::{
    get_cli_version,
    get_configured_deployment_name,
    with_provision,
    BackendProvisioner,
    ProvisionHostCredentials,
    ProvisionRequest,
};
