//! In-memory usage metric storage for deployment usage-limit enforcement.
//!
//! Live usage is recorded from the usage-event stream as events are emitted,
//! so enforcement reads are realtime. The stores are rehydrated from
//! historical usage rollups by `AppMetricSeeder` on deployment load.
//!
//! The module splits into the store model and its calendar-window math
//! ([`stores`]), the meter that evaluates limits against it ([`meter`]), the
//! recorder that feeds the meter from the usage-event stream ([`recorder`]),
//! and the background worker that acts on evaluations ([`worker`]).

mod meter;
mod recorder;
mod stores;
mod worker;

pub use self::{
    meter::{
        ExceededUsageLimit,
        UsageLimitEvaluation,
        UsageMeter,
    },
    recorder::{
        usage_deltas,
        UsageLimitRecorder,
    },
    stores::{
        UsageMetricResolution,
        UsageMetricStores,
    },
    worker::UsageLimitWorker,
};
