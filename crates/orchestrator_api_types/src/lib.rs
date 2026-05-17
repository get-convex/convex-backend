//! Request/response types for the `convex-orchestrator` HTTP surface.
//!
//! The orchestrator is the self-hosted replacement for "BigBrain", the
//! hosted Convex provisioning service. The wire formats here match what the
//! existing dashboard, CLI, and `crates/big_brain_client` already
//! deserialize.
//!
//! Types fall into three groups:
//! - `deployment` — load-bearing types re-exported from
//!   `big_brain_private_api_types` for byte-compatibility with the backend
//!   and CLI (`crates/big_brain_client`, `npm-packages/convex/src/cli`).
//! - `dashboard` — types served under `/api/dashboard/*` to back the cloud
//!   dashboard's existing typed clients.
//! - `management` — types served under `/v1/*`, the public Convex
//!   Management API.

pub mod dashboard;
pub mod deployment;
pub mod management;
pub mod stubs;
