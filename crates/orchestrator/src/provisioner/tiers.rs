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
//!
//! `max` is an unbounded tier: `docker run` skips `--memory`/`--cpus`
//! flags entirely, and the host-capacity check treats it as consuming the
//! whole host so subsequent deployments hit 409. Intended for single-tenant
//! hosts where one deployment should own all resources.

/// Default tier assigned to projects + deployments when nothing is specified.
pub const DEFAULT_TIER: &str = "S16";

#[derive(Debug, Clone, Copy)]
pub struct Tier {
    pub name: &'static str,
    pub memory_mb: u32,
    pub cpus: f32,
    pub knob_defaults: &'static [(&'static str, &'static str)],
    /// When `true`, provisioner skips `--memory`/`--cpus` docker flags and
    /// the host-capacity check treats this tier as consuming the entire host.
    pub unbounded: bool,
}

/// Tier ladder, in increasing resource order.
pub const TIERS: &[Tier] = &[
    Tier {
        name: "S4",
        memory_mb: 1024,
        cpus: 0.5,
        unbounded: false,
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
        unbounded: false,
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
        unbounded: false,
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
        unbounded: false,
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
    Tier {
        name: "S64",
        memory_mb: 16384,
        cpus: 8.0,
        unbounded: false,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "16"),
            ("UDF_CACHE_MAX_SIZE", "419430400"), // 4 × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "200000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "1000000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "2000000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "512"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "4096"),
        ],
    },
    Tier {
        name: "S128",
        memory_mb: 32768,
        cpus: 16.0,
        unbounded: false,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "32"),
            ("UDF_CACHE_MAX_SIZE", "838860800"), // 8 × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "400000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "2000000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "4000000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "1024"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "8192"),
        ],
    },
    Tier {
        name: "S256",
        memory_mb: 65536,
        cpus: 32.0,
        unbounded: false,
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "64"),
            ("UDF_CACHE_MAX_SIZE", "1677721600"), // 16 × 104,857,600
            ("FUNRUN_INDEX_CACHE_SIZE", "800000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "4000000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "8000000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "2048"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "16384"),
        ],
    },
    Tier {
        name: "max",
        // memory_mb and cpus are unused when unbounded=true; docker run
        // skips --memory/--cpus. Use 0 as a sentinel so arithmetic on
        // bounded tiers stays correct.
        memory_mb: 0,
        cpus: 0.0,
        unbounded: true,
        // max reuses S256's knob defaults so cache sizes don't blow up.
        knob_defaults: &[
            ("RUNTIME_WORKER_THREADS", "64"),
            ("UDF_CACHE_MAX_SIZE", "1677721600"),
            ("FUNRUN_INDEX_CACHE_SIZE", "800000000"),
            ("FUNRUN_MODULE_CACHE_SIZE", "4000000000"),
            ("FUNRUN_CODE_CACHE_SIZE", "8000000000"),
            ("FUNRUN_MAX_ISOLATE_WORKERS", "2048"),
            ("HTTP_SERVER_MAX_CONCURRENT_REQUESTS", "16384"),
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
        // Unbounded tiers (e.g. `max`) don't have a real memory_mb value;
        // exclude them from the monotonicity check.
        let mems: Vec<_> = TIERS
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
}
