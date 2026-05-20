//! Hardcoded resource tiers for backend deployments.
//!
//! Each `Tier` declares a Docker `--memory` / `--cpus` budget and a
//! coordinated bundle of backend env-var defaults that scale with that
//! budget. `S16` is the default; its cache + concurrency knobs are
//! bit-exact upstream defaults so existing deployments don't regress on
//! upgrade. The lone exception is `RUNTIME_WORKER_THREADS`, which we
//! pin to `4` (upstream's `0` means "use all host cores", which varies
//! across heterogeneous hosts — pinning a fixed worker count keeps
//! behavior predictable from one host to another).

/// Default tier assigned to projects + deployments when nothing is specified.
pub const DEFAULT_TIER: &str = "S16";

#[derive(Debug, Clone, Copy)]
pub struct Tier {
    pub name: &'static str,
    pub memory_mb: u32,
    pub cpus: f32,
    pub knob_defaults: &'static [(&'static str, &'static str)],
}

/// Tier ladder, in increasing resource order.
pub const TIERS: &[Tier] = &[
    Tier {
        name: "S4",
        memory_mb: 1024,
        cpus: 0.5,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "1"),
            ("UDF_CACHE_MAX_SIZE", "26214400"), // ¼ × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "12500000"), // ¼ × 50,000,000
            ("FUNRUN_MODULE_CACHE_SIZE", "62500000"), // ¼ × 250,000,000
            ("FUNRUN_CODE_CACHE_SIZE", "125000000"), // ¼ × 500,000,000
            ("FUNRUN_MAX_ISOLATE_WORKERS", "32"), // ¼ × 128
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "256"),
        ],
    },
    Tier {
        name: "S8",
        memory_mb: 2048,
        cpus: 1.0,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "2"),
            ("UDF_CACHE_MAX_SIZE", "52428800"), // ½ × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "25000000"), // ½ × 50,000,000
            ("FUNRUN_MODULE_CACHE_SIZE", "125000000"), // ½ × 250,000,000
            ("FUNRUN_CODE_CACHE_SIZE", "250000000"), // ½ × 500,000,000
            ("FUNRUN_MAX_ISOLATE_WORKERS", "64"), // ½ × 128
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "512"),
        ],
    },
    Tier {
        name: "S16",
        memory_mb: 4096,
        cpus: 2.0,
        // S16 = upstream defaults verbatim. Do not change these without
        // breaking the "no regression on upgrade" guarantee from the spec.
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "4"), // upstream is 0 (= host cores); pin for predictability
            ("UDF_CACHE_MAX_SIZE", "104857600"),
            ("FUNRUN_INDEX_CACHE_SIZE", "50000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "250000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "500000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "128"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "1024"),
        ],
    },
    Tier {
        name: "S32",
        memory_mb: 8192,
        cpus: 4.0,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "8"),
            ("UDF_CACHE_MAX_SIZE", "209715200"), // 2 × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "100000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "500000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "1000000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "256"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "2048"),
        ],
    },
];

pub fn lookup(name: &str) -> Option<&'static Tier> {
    TIERS.iter().find(|t| t.name == name)
}

pub fn all_tier_names() -> impl Iterator<Item = &'static str> {
    TIERS.iter().map(|t| t.name)
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn s16_matches_upstream_defaults() {
        let s16 = lookup("S16").unwrap();
        // These four are the spec's "bit-exact upstream defaults" guarantee.
        // If upstream changes them, update both knobs.rs and this test.
        let pairs: std::collections::HashMap<_, _> = s16.knob_defaults.iter().copied().collect();
        assert_eq!(pairs["UDF_CACHE_MAX_SIZE"], "104857600");
        assert_eq!(pairs["FUNRUN_INDEX_CACHE_SIZE"], "50000000");
        assert_eq!(pairs["FUNRUN_MODULE_CACHE_SIZE"], "250000000");
        assert_eq!(pairs["FUNRUN_CODE_CACHE_SIZE"], "500000000");
        assert_eq!(pairs["FUNRUN_MAX_ISOLATE_WORKERS"], "128");
        assert_eq!(pairs["HTTP_SERVER_MAX_CONCURRENT_REQUESTS"], "1024");
        // RUNTIME_WORKER_THREADS is intentionally pinned to "4" — upstream
        // default is "0" (= number of host cores). See module doc.
        assert_eq!(pairs["RUNTIME_WORKER_THREADS"], "4");
    }

    #[test]
    fn memory_strictly_increases_through_ladder() {
        let mems: Vec<_> = TIERS.iter().map(|t| t.memory_mb).collect();
        for w in mems.windows(2) {
            assert!(w[0] < w[1], "tier memory must be strictly increasing");
        }
    }
}
