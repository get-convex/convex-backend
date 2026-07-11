//! In-memory usage metric storage for deployment usage-limit enforcement.
//!
//! Live usage is recorded from the usage-event stream as events are emitted,
//! so enforcement reads are realtime. The stores are rehydrated from
//! historical usage rollups by `AppMetricSeeder` on deployment load.

mod meter;
mod recorder;
mod stores;
mod worker;

pub use self::{
    meter::{
        ExceededUsageLimit,
        SeedRow,
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
