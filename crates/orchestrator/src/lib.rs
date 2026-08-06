//! `convex-orchestrator`: a self-hosted replacement for the hosted Convex
//! "BigBrain" provisioning service.
//!
//! See `docs/superpowers/specs/2026-05-02-convex-orchestrator-design.md`
//! for the full design.

pub mod acme;
pub mod auth;
pub mod config;
pub mod custom_domains;
pub mod host_capacity;
pub mod errors;
pub mod ids;
pub mod knob_registry;
pub mod provisioner;
pub mod proxy;
pub mod router;
pub mod routes;
pub mod secrets;
pub mod state;
pub mod storage;
pub mod stub_data;
pub mod time;
