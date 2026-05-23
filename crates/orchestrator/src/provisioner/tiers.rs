//! Resource tiers for backend deployments.
//!
//! Each `Tier` declares a Docker `--memory` / `--cpus` budget and a
//! coordinated bundle of backend env-var defaults calculated from that
//! budget. `S16` is the default; its cache + concurrency knobs are the
//! formula baseline and remain bit-exact upstream defaults. Other resource
//! sizes get aggressive headroom so most knobs are intentionally much more
//! generous than the raw resource ratio. Two knobs are deliberately gentler:
//! `RUNTIME_WORKER_THREADS` follows the CPU cap without headroom so Tokio
//! doesn't oversubscribe the container, and `POSTGRES_MAX_CONNECTIONS` uses a
//! softer curve so large tiers don't ask Postgres to manage thousands of idle
//! backends before the workload needs them.
//!
//! Custom tiers use the string form `custom:<memory_mb>:<cpus>`, for example
//! `custom:12288:6.5`. They use explicit Docker resource caps and the same
//! generous dynamic knob formula as named tiers.

use std::borrow::Cow;

/// Default tier assigned to projects + deployments when nothing is specified.
pub const DEFAULT_TIER: &str = "S16";
const BASE_MEMORY_MB: u32 = 4096;
const BASE_CPUS: f32 = 2.0;
const MIN_KNOB_SCALE: f64 = 0.25;
const KNOB_HEADROOM_MULTIPLIER: f64 = 3.0;
const LEGACY_MAX_KNOB_MEMORY_MB: u32 = 131_072;
const LEGACY_MAX_KNOB_CPUS: f32 = 64.0;

const BASE_KNOB_DEFAULTS: &[(&str, u128)] = &[
    ("RUNTIME_WORKER_THREADS", 4),
    ("UDF_CACHE_MAX_SIZE", 104_857_600),
    ("FUNRUN_INDEX_CACHE_SIZE", 50_000_000),
    ("FUNRUN_MODULE_CACHE_SIZE", 250_000_000),
    ("FUNRUN_CODE_CACHE_SIZE", 500_000_000),
    ("FUNRUN_MAX_ISOLATE_WORKERS", 128),
    ("HTTP_SERVER_TCP_BACKLOG", 256),
    ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", 1024),
    ("APPLICATION_MAX_CONCURRENT_QUERIES", 16),
    ("APPLICATION_MAX_CONCURRENT_MUTATIONS", 16),
    ("APPLICATION_MAX_CONCURRENT_V8_ACTIONS", 64),
    ("APPLICATION_MAX_CONCURRENT_NODE_ACTIONS", 64),
    ("MAX_CONCURRENT_ACTION_OPS", 8),
    ("COMMITTER_QUEUE_SIZE", 128),
    ("MAX_BYTES_WRITTEN_PER_SECOND", 4_194_304),
    ("POSTGRES_MAX_CONNECTIONS", 128),
];

#[derive(Debug, Clone)]
pub struct Tier {
    pub name: Cow<'static, str>,
    pub memory_mb: u32,
    pub cpus: f32,
    pub knob_defaults: Vec<(&'static str, String)>,
    /// When `true`, provisioner skips `--memory`/`--cpus` docker flags and
    /// runs without an orchestrator-imposed resource cap.
    pub unbounded: bool,
    /// True for `custom:<memory_mb>:<cpus>` values.
    pub custom: bool,
}

#[derive(Debug, Clone, Copy)]
struct TierPreset {
    name: &'static str,
    memory_mb: u32,
    cpus: f32,
    unbounded: bool,
}

impl TierPreset {
    fn resolve(&self) -> Tier {
        let (knob_memory_mb, knob_cpus) = if self.unbounded {
            (LEGACY_MAX_KNOB_MEMORY_MB, LEGACY_MAX_KNOB_CPUS)
        } else {
            (self.memory_mb, self.cpus)
        };
        Tier {
            name: Cow::Borrowed(self.name),
            memory_mb: self.memory_mb,
            cpus: self.cpus,
            knob_defaults: knob_defaults_for_resources(knob_memory_mb, knob_cpus, self.unbounded),
            unbounded: self.unbounded,
            custom: false,
        }
    }
}

/// Tier ladder, in increasing resource order.
const TIER_PRESETS: &[TierPreset] = &[
    TierPreset {
        name: "S4",
        memory_mb: 1024,
        cpus: 0.5,
        unbounded: false,
    },
    TierPreset {
        name: "S8",
        memory_mb: 2048,
        cpus: 1.0,
        unbounded: false,
    },
    TierPreset {
        name: "S16",
        memory_mb: 4096,
        cpus: 2.0,
        unbounded: false,
    },
    TierPreset {
        name: "S32",
        memory_mb: 8192,
        cpus: 4.0,
        unbounded: false,
    },
    TierPreset {
        name: "S64",
        memory_mb: 16384,
        cpus: 8.0,
        unbounded: false,
    },
    TierPreset {
        name: "S128",
        memory_mb: 32768,
        cpus: 16.0,
        unbounded: false,
    },
    TierPreset {
        name: "S256",
        memory_mb: 65536,
        cpus: 32.0,
        unbounded: false,
    },
    TierPreset {
        name: "max",
        // memory_mb and cpus are unused when unbounded=true; docker run
        // skips --memory/--cpus. Use 0 as a sentinel so arithmetic on
        // bounded tiers stays correct.
        memory_mb: 0,
        cpus: 0.0,
        unbounded: true,
    },
];

pub fn lookup(name: &str) -> Option<Tier> {
    TIER_PRESETS
        .iter()
        .find(|t| t.name == name)
        .map(TierPreset::resolve)
}

pub fn resolve(name: &str) -> Option<Tier> {
    lookup(name).or_else(|| parse_custom(name))
}

pub fn all_tier_names() -> impl Iterator<Item = &'static str> {
    TIER_PRESETS.iter().map(|t| t.name)
}

fn parse_custom(name: &str) -> Option<Tier> {
    let rest = name.strip_prefix("custom:")?;
    let (memory_mb, cpus) = rest.split_once(':')?;
    if cpus.contains(':') {
        return None;
    }
    let memory_mb = memory_mb.parse::<u32>().ok()?;
    let cpus = cpus.parse::<f32>().ok()?;
    if memory_mb == 0 || !cpus.is_finite() || cpus <= 0.0 {
        return None;
    }
    Some(Tier {
        name: Cow::Owned(name.to_string()),
        memory_mb,
        cpus,
        knob_defaults: knob_defaults_for_resources(memory_mb, cpus, false),
        unbounded: false,
        custom: true,
    })
}

fn knob_defaults_for_resources(
    memory_mb: u32,
    cpus: f32,
    unbounded: bool,
) -> Vec<(&'static str, String)> {
    let scale = resource_knob_scale(memory_mb, cpus);
    BASE_KNOB_DEFAULTS
        .iter()
        .map(|(key, value)| {
            let scaled = match *key {
                "RUNTIME_WORKER_THREADS" => runtime_worker_threads_for_resources(cpus, unbounded),
                "POSTGRES_MAX_CONNECTIONS" => {
                    ((*value as f64) * postgres_connection_scale(memory_mb, cpus)).ceil() as u128
                },
                _ => ((*value as f64) * scale).ceil() as u128,
            };
            (*key, scaled)
        })
        .map(|(key, value)| (key, value.to_string()))
        .collect()
}

fn resource_knob_scale(memory_mb: u32, cpus: f32) -> f64 {
    let raw = raw_resource_scale(memory_mb, cpus);
    if raw > 1.0 {
        (raw * KNOB_HEADROOM_MULTIPLIER).ceil()
    } else if raw < 1.0 {
        (raw * KNOB_HEADROOM_MULTIPLIER).min(1.0)
    } else {
        raw
    }
}

fn raw_resource_scale(memory_mb: u32, cpus: f32) -> f64 {
    (memory_mb as f64 / BASE_MEMORY_MB as f64)
        .max(cpus as f64 / BASE_CPUS as f64)
        .max(MIN_KNOB_SCALE)
}

fn postgres_connection_scale(memory_mb: u32, cpus: f32) -> f64 {
    let raw = raw_resource_scale(memory_mb, cpus);
    if raw > 1.0 {
        (raw.sqrt() * KNOB_HEADROOM_MULTIPLIER).ceil()
    } else if raw < 1.0 {
        (raw * KNOB_HEADROOM_MULTIPLIER).min(1.0)
    } else {
        raw
    }
}

fn runtime_worker_threads_for_resources(cpus: f32, unbounded: bool) -> u128 {
    if unbounded {
        // Match bare-metal behavior: let Tokio choose from the actual host.
        return 0;
    }
    (cpus as f64).ceil().clamp(1.0, 64.0) as u128
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(tier: &Tier) -> std::collections::HashMap<&'static str, &str> {
        tier.knob_defaults
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect()
    }

    #[test]
    fn default_tier_resolves() {
        assert!(lookup(DEFAULT_TIER).is_some());
    }

    #[test]
    fn all_advertised_tiers_resolve() {
        for name in all_tier_names() {
            assert!(lookup(name).is_some(), "tier {name} missing from TIERS");
        }
    }

    #[test]
    fn lookup_unknown_is_none() {
        assert!(lookup("XL").is_none());
        assert!(lookup("").is_none());
    }

    #[test]
    fn custom_tier_resolves_resource_limits() {
        let custom = resolve("custom:12288:6.5").unwrap();
        assert_eq!(custom.name, "custom:12288:6.5");
        assert_eq!(custom.memory_mb, 12288);
        assert_eq!(custom.cpus, 6.5);
        assert!(!custom.unbounded);

        let defaults = pairs(&custom);
        assert_eq!(defaults["POSTGRES_MAX_CONNECTIONS"], "768");
        assert_eq!(defaults["RUNTIME_WORKER_THREADS"], "7");
    }

    #[test]
    fn custom_tier_calculates_generous_knobs_from_memory_or_cpu() {
        let cpu_heavy = resolve("custom:1024:6.5").unwrap();
        let cpu_defaults = pairs(&cpu_heavy);
        assert_eq!(cpu_defaults["POSTGRES_MAX_CONNECTIONS"], "768");
        assert_eq!(cpu_defaults["RUNTIME_WORKER_THREADS"], "7");

        let mid_ladder = resolve("custom:9000:4.1").unwrap();
        let mid_ladder_defaults = pairs(&mid_ladder);
        assert_eq!(mid_ladder_defaults["POSTGRES_MAX_CONNECTIONS"], "640");
        assert_eq!(mid_ladder_defaults["RUNTIME_WORKER_THREADS"], "5");

        let above_ladder = resolve("custom:65536:48").unwrap();
        let above_ladder_defaults = pairs(&above_ladder);
        assert_eq!(above_ladder_defaults["POSTGRES_MAX_CONNECTIONS"], "1920");
        assert_eq!(above_ladder_defaults["RUNTIME_WORKER_THREADS"], "48");
    }

    #[test]
    fn custom_tier_rejects_invalid_resources() {
        assert!(resolve("custom:0:1").is_none());
        assert!(resolve("custom:1024:0").is_none());
        assert!(resolve("custom:1024:NaN").is_none());
        assert!(resolve("custom:1024:not-a-cpu").is_none());
    }

    #[test]
    fn s16_matches_upstream_defaults() {
        let s16 = lookup("S16").unwrap();
        // These four are the spec's "bit-exact upstream defaults" guarantee.
        // If upstream changes them, update both knobs.rs and this test.
        let defaults = pairs(&s16);
        assert_eq!(defaults["UDF_CACHE_MAX_SIZE"], "104857600");
        assert_eq!(defaults["FUNRUN_INDEX_CACHE_SIZE"], "50000000");
        assert_eq!(defaults["FUNRUN_MODULE_CACHE_SIZE"], "250000000");
        assert_eq!(defaults["FUNRUN_CODE_CACHE_SIZE"], "500000000");
        assert_eq!(defaults["FUNRUN_MAX_ISOLATE_WORKERS"], "128");
        assert_eq!(defaults["HTTP_SERVER_TCP_BACKLOG"], "256");
        assert_eq!(defaults["HTTP_SERVER_MAX_CONCURRENT_REQUESTS"], "1024");
        assert_eq!(defaults["APPLICATION_MAX_CONCURRENT_QUERIES"], "16");
        assert_eq!(defaults["APPLICATION_MAX_CONCURRENT_MUTATIONS"], "16");
        assert_eq!(defaults["APPLICATION_MAX_CONCURRENT_V8_ACTIONS"], "64");
        assert_eq!(defaults["APPLICATION_MAX_CONCURRENT_NODE_ACTIONS"], "64");
        assert_eq!(defaults["MAX_CONCURRENT_ACTION_OPS"], "8");
        assert_eq!(defaults["COMMITTER_QUEUE_SIZE"], "128");
        assert_eq!(defaults["MAX_BYTES_WRITTEN_PER_SECOND"], "4194304");
        // RUNTIME_WORKER_THREADS follows the explicit CPU cap without the
        // generous headroom multiplier used for throughput queues.
        assert_eq!(defaults["RUNTIME_WORKER_THREADS"], "2");
        // POSTGRES_MAX_CONNECTIONS — upstream default for Postgres clients
        // (matches `crates/common/src/knobs.rs:934`).
        assert_eq!(defaults["POSTGRES_MAX_CONNECTIONS"], "128");
    }

    #[test]
    fn memory_strictly_increases_through_ladder() {
        // Unbounded tiers (e.g. `max`) don't have a real memory_mb value;
        // exclude them from the monotonicity check.
        let mems: Vec<_> = TIER_PRESETS
            .iter()
            .filter(|t| !t.unbounded)
            .map(|t| t.memory_mb)
            .collect();
        for w in mems.windows(2) {
            assert!(w[0] < w[1], "tier memory must be strictly increasing");
        }
    }

    #[test]
    fn unbounded_tier_recognized() {
        let max = lookup("max").unwrap();
        assert!(max.unbounded, "`max` tier must be unbounded");
        let s16 = lookup("S16").unwrap();
        assert!(!s16.unbounded, "S16 must not be unbounded");
    }

    #[test]
    fn postgres_max_connections_include_aggressive_headroom_per_tier() {
        let expected: &[(&str, &str)] = &[
            ("S4", "96"),
            ("S8", "128"),
            ("S16", "128"),
            ("S32", "640"),
            ("S64", "768"),
            ("S128", "1152"),
            ("S256", "1536"),
            ("max", "2176"),
        ];
        for (tier_name, want) in expected {
            let tier = lookup(tier_name).expect("tier present");
            let got = tier
                .knob_defaults
                .iter()
                .find(|(k, _)| *k == "POSTGRES_MAX_CONNECTIONS")
                .map(|(_, v)| v.as_str())
                .unwrap_or("MISSING");
            assert_eq!(
                got, *want,
                "tier {tier_name} POSTGRES_MAX_CONNECTIONS = {got:?}, want {want:?}",
            );
        }
    }

    #[test]
    fn throughput_knobs_scale_with_tier_headroom() {
        let s128 = lookup("S128").unwrap();
        let s128_defaults = pairs(&s128);
        assert_eq!(s128_defaults["HTTP_SERVER_TCP_BACKLOG"], "6144");
        assert_eq!(s128_defaults["APPLICATION_MAX_CONCURRENT_QUERIES"], "384");
        assert_eq!(s128_defaults["APPLICATION_MAX_CONCURRENT_MUTATIONS"], "384");
        assert_eq!(s128_defaults["APPLICATION_MAX_CONCURRENT_V8_ACTIONS"], "1536");
        assert_eq!(s128_defaults["APPLICATION_MAX_CONCURRENT_NODE_ACTIONS"], "1536");
        assert_eq!(s128_defaults["MAX_CONCURRENT_ACTION_OPS"], "192");
        assert_eq!(s128_defaults["COMMITTER_QUEUE_SIZE"], "3072");
        assert_eq!(s128_defaults["MAX_BYTES_WRITTEN_PER_SECOND"], "100663296");

        let s256 = lookup("S256").unwrap();
        let s256_defaults = pairs(&s256);
        assert_eq!(s256_defaults["HTTP_SERVER_TCP_BACKLOG"], "12288");
        assert_eq!(s256_defaults["APPLICATION_MAX_CONCURRENT_QUERIES"], "768");
        assert_eq!(s256_defaults["APPLICATION_MAX_CONCURRENT_MUTATIONS"], "768");
        assert_eq!(s256_defaults["APPLICATION_MAX_CONCURRENT_V8_ACTIONS"], "3072");
        assert_eq!(s256_defaults["APPLICATION_MAX_CONCURRENT_NODE_ACTIONS"], "3072");
        assert_eq!(s256_defaults["MAX_CONCURRENT_ACTION_OPS"], "384");
        assert_eq!(s256_defaults["COMMITTER_QUEUE_SIZE"], "6144");
        assert_eq!(s256_defaults["MAX_BYTES_WRITTEN_PER_SECOND"], "201326592");
    }

    #[test]
    fn runtime_worker_threads_do_not_use_throughput_headroom() {
        let expected: &[(&str, &str)] = &[
            ("S4", "1"),
            ("S8", "1"),
            ("S16", "2"),
            ("S32", "4"),
            ("S64", "8"),
            ("S128", "16"),
            ("S256", "32"),
            ("max", "0"),
        ];
        for (tier_name, want) in expected {
            let tier = lookup(tier_name).expect("tier present");
            let got = tier
                .knob_defaults
                .iter()
                .find(|(k, _)| *k == "RUNTIME_WORKER_THREADS")
                .map(|(_, v)| v.as_str())
                .unwrap_or("MISSING");
            assert_eq!(
                got, *want,
                "tier {tier_name} RUNTIME_WORKER_THREADS = {got:?}, want {want:?}",
            );
        }
    }
}
