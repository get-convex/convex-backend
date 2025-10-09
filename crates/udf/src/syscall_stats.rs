use std::time::Duration;

use pb::outcome::SyscallStats as SyscallStatsProto;
#[cfg(any(test, feature = "testing"))]
use proptest::prelude::*;
use serde_json::{
    json,
    Value as JsonValue,
};
use value::heap_size::HeapSize;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SyscallStats {
    pub invocations: u32,
    pub errors: u32,
    pub total_duration: Duration,
}

#[cfg(any(test, feature = "testing"))]
impl Arbitrary for SyscallStats {
    type Parameters = ();

    type Strategy = impl Strategy<Value = SyscallStats>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        use proptest::prelude::*;
        (
            any::<u32>(),
            any::<u32>(),
            0..=i64::MAX as u64,
            any::<u32>(),
        )
            .prop_map(|(invocations, errors, secs, nanos)| {
                let total_duration = Duration::new(secs, nanos);
                Self {
                    invocations,
                    errors,
                    total_duration,
                }
            })
    }
}

impl HeapSize for SyscallStats {
    fn heap_size(&self) -> usize {
        0
    }
}

impl SyscallStats {
    pub fn merge(&mut self, other: &Self) {
        self.invocations += other.invocations;
        self.errors += other.errors;
        self.total_duration += other.total_duration;
    }
}

impl TryFrom<SyscallStats> for SyscallStatsProto {
    type Error = anyhow::Error;

    fn try_from(
        SyscallStats {
            invocations,
            errors,
            total_duration,
        }: SyscallStats,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            invocations,
            errors,
            total_duration: Some(total_duration.try_into()?),
        })
    }
}

impl TryFrom<SyscallStatsProto> for SyscallStats {
    type Error = anyhow::Error;

    fn try_from(
        SyscallStatsProto {
            invocations,
            errors,
            total_duration,
        }: SyscallStatsProto,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            invocations,
            errors,
            total_duration: total_duration
                .ok_or_else(|| anyhow::anyhow!("Missing total duration"))?
                .try_into()?,
        })
    }
}

impl From<SyscallStats> for JsonValue {
    fn from(value: SyscallStats) -> Self {
        json!({
            "invocations": value.invocations,
            "errors": value.errors,
            "totalDuration": value.total_duration.as_secs_f64(),
        })
    }
}

#[cfg(test)]
mod tests {

    use cmd_util::env::env_config;
    use proptest::prelude::*;
    use value::testing::assert_roundtrips;

    use super::{
        SyscallStats,
        SyscallStatsProto,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { cases: 256 * env_config("CONVEX_PROPTEST_MULTIPLIER", 1), failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_syscall_stats_roundtrips(left in any::<SyscallStats>()) {
            assert_roundtrips::<SyscallStats, SyscallStatsProto>(left);
        }
    }
}
