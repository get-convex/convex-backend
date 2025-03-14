use std::{
    collections::BTreeMap,
    hash::{
        Hash,
        Hasher,
    },
    str::FromStr,
    sync::LazyLock,
};

use anyhow::Context;
use fastrace::{
    collector::SpanContext,
    Span,
};
use fnv::FnvHasher;
use parking_lot::Mutex;
use rand::Rng;
use regex::Regex;
use serde::Deserialize;

use crate::knobs::REQUEST_TRACE_SAMPLE_CONFIG;

static SAMPLING_CONFIG_FROM_LOADER: LazyLock<Mutex<Option<SamplingConfig>>> =
    LazyLock::new(|| Mutex::new(None));

#[derive(Clone, Debug)]
pub struct EncodedSpan(pub Option<String>);

impl EncodedSpan {
    pub fn empty() -> Self {
        Self(None)
    }

    /// Encodes the current local parent `SpanContext`
    pub fn from_parent() -> Self {
        Self(SpanContext::current_local_parent().map(|ctx| ctx.encode_w3c_traceparent()))
    }
}

/// Given an instance name returns a span with the sample percentage specified
/// in `knobs.rs`
pub fn get_sampled_span<R: Rng>(
    instance_name: &str,
    name: &str,
    rng: &mut R,
    properties: BTreeMap<String, String>,
) -> Span {
    let sample_ratio = get_sampling_ratio(instance_name, name);
    let should_sample = rng.random_bool(sample_ratio);
    match should_sample {
        true => Span::root(name.to_owned(), SpanContext::random())
            .with_properties(|| properties)
            .with_property(|| ("dev.convex.instance_name", instance_name.to_owned())),
        false => Span::noop(),
    }
}

/// Psuedorandomly sample a span based on `key`, deterministically making the
/// same decision each time this function is called with the same `key`.
pub fn get_keyed_sampled_span<K: Hash + std::fmt::Debug>(
    key: K,
    instance_name: &str,
    name: &str,
    span_ctx: SpanContext,
    properties: BTreeMap<String, String>,
) -> Span {
    let mut hasher = FnvHasher::default();
    key.hash(&mut hasher);
    let hash = hasher.finish() as u32;
    let sample_ratio = get_sampling_ratio(instance_name, name);
    let threshold = ((u32::MAX as f64) * sample_ratio) as u32;
    if hash < threshold {
        tracing::info!("Sampling span for {key:?}: {name}");
        Span::root(name.to_owned(), span_ctx)
            .with_properties(|| properties)
            .with_property(|| ("dev.convex.instance_name", instance_name.to_owned()))
    } else {
        tracing::info!("Not sampling span for {key:?}: {name}");
        Span::noop()
    }
}

/// Sets the sampling configuration to be used by the `get_sampled_span`
/// function
pub fn set_sampling_config(config_str: &str) {
    match config_str.parse() {
        Ok(config) => {
            *SAMPLING_CONFIG_FROM_LOADER.lock() = Some(config);
            tracing::info!("Sampling config set to: {}", config_str.replace("\n", ""));
        },
        Err(e) => {
            tracing::error!("Failed to parse sampling config: {}", e);
        },
    }
}

fn get_sampling_ratio(instance_name: &str, name: &str) -> f64 {
    if SAMPLING_CONFIG_FROM_LOADER.lock().is_some() {
        SAMPLING_CONFIG_FROM_LOADER
            .lock()
            .as_ref()
            .unwrap()
            .sample_ratio(instance_name, name)
    } else {
        REQUEST_TRACE_SAMPLE_CONFIG.sample_ratio(instance_name, name)
    }
}

#[derive(Debug, Default)]
pub struct SamplingConfig {
    by_regex: Vec<(Option<String>, Regex, f64)>,
}

impl PartialEq for SamplingConfig {
    fn eq(&self, other: &Self) -> bool {
        if self.by_regex.len() != other.by_regex.len() {
            return false;
        }
        self.by_regex
            .iter()
            .zip(&other.by_regex)
            .all(|(a, b)| a.0 == b.0 && a.1.as_str() == b.1.as_str() && a.2 == b.2)
    }
}

impl SamplingConfig {
    fn sample_ratio(&self, instance_name: &str, name: &str) -> f64 {
        self.by_regex
            .iter()
            .find_map(|(rule_instance_name, name_regex, sample_ratio)| {
                if let Some(rule_instance_name) = rule_instance_name {
                    if rule_instance_name != instance_name {
                        return None;
                    }
                }
                if name_regex.is_match(name) {
                    Some(*sample_ratio)
                } else {
                    None
                }
            })
            .unwrap_or(0.0)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RouteOverride {
    route_regexp: String,
    fraction: f64,
}

// These are in priority order -- instance overrides take precedence over route
// overrides, which take precedence over the default fraction.
//
// When in doubt, write out a test case to verify the behavior.
// Technically the default fraction is redundant with `routeOverrides: [{
// "routeRegexp": ".*", "fraction": ... }]`, but it's pulled out for clarity.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SamplingConfigJson {
    instance_overrides: Option<BTreeMap<String, Vec<RouteOverride>>>,
    route_overrides: Option<Vec<RouteOverride>>,
    default_fraction: f64,
}

fn validate_fraction(value: f64, context: &str) -> anyhow::Result<f64> {
    if !(0.0..=1.0).contains(&value) {
        anyhow::bail!(
            "Invalid fraction {} in {}: clamping to [0.0, 1.0]",
            value,
            context
        );
    }
    Ok(value)
}

static DOT_STAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(".*").expect(".* is not a valid regex"));

impl TryFrom<SamplingConfigJson> for SamplingConfig {
    type Error = anyhow::Error;

    fn try_from(json: SamplingConfigJson) -> anyhow::Result<Self> {
        let mut by_regex = Vec::new();
        if let Some(instance_overrides) = json.instance_overrides {
            for (instance_name, route_overrides) in instance_overrides.iter() {
                for route_override in route_overrides {
                    by_regex.push((
                        Some(instance_name.to_owned()),
                        Regex::new(&route_override.route_regexp).context("Invalid route regexp")?,
                        validate_fraction(route_override.fraction, instance_name)?,
                    ));
                }
            }
        }
        if let Some(route_overrides) = json.route_overrides {
            for route_override in route_overrides {
                by_regex.push((
                    None,
                    Regex::new(&route_override.route_regexp).context("Invalid route regexp")?,
                    validate_fraction(route_override.fraction, &route_override.route_regexp)?,
                ));
            }
        }
        by_regex.push((
            None,
            DOT_STAR.clone(),
            validate_fraction(json.default_fraction, "default")?,
        ));
        Ok(SamplingConfig { by_regex })
    }
}

impl FromStr for SamplingConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        if s.starts_with('{') {
            let json: SamplingConfigJson =
                serde_json::from_str(s).context("Failed to parse sampling config as JSON")?;
            return SamplingConfig::try_from(json);
        }

        let mut by_regex = Vec::new();
        for token in s.split(',') {
            let parts: Vec<_> = token.split(':').map(|s| s.trim()).collect();
            anyhow::ensure!(parts.len() <= 2, "Too many parts {}", token);
            let (instance_name, token2) = if parts.len() == 2 {
                let instance_name = Some(parts[0].to_owned());
                (instance_name, parts[1])
            } else {
                (None, parts[0])
            };

            let parts: Vec<_> = token2.split('=').map(|s| s.trim()).collect();
            anyhow::ensure!(parts.len() <= 2, "Too many parts {}", token2);
            let (name_regex, rate) = if parts.len() == 2 {
                let regex = Regex::new(parts[0]).context("Failed to parse name regex")?;
                let rate: f64 = parts[1].parse().context("Failed to parse sampling rate")?;
                (regex, rate)
            } else {
                let rate: f64 = parts[0].parse().context("Failed to parse sampling rate")?;
                (DOT_STAR.clone(), rate)
            };
            by_regex.push((instance_name, name_regex, rate));
        }
        Ok(SamplingConfig { by_regex })
    }
}

/// Creates a root span from an encoded parent trace
pub fn initialize_root_from_parent(span_name: &str, encoded_parent: EncodedSpan) -> Span {
    if let Some(p) = encoded_parent.0 {
        if let Some(ctx) = SpanContext::decode_w3c_traceparent(p.as_str()) {
            return Span::root(span_name.to_string(), ctx);
        }
    }
    Span::noop()
}

#[cfg(test)]
mod tests {
    use crate::fastrace_helpers::SamplingConfig;

    #[test]
    fn test_parse_sampling_config() -> anyhow::Result<()> {
        let config: SamplingConfig = "1".parse()?;
        assert_eq!(config.by_regex.len(), 1);
        assert_eq!(config.sample_ratio("carnitas", "a"), 1.0);

        let config: SamplingConfig = "a=0.5,b=0.15".parse()?;
        assert_eq!(config.by_regex.len(), 2);
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.15);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.0);

        let config: SamplingConfig = "a=0.5,b=0.15,0.01".parse()?;
        assert_eq!(config.by_regex.len(), 3);
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.15);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.01);

        let config: SamplingConfig = "/f/.*=0.5".parse()?;
        assert_eq!(config.by_regex.len(), 1);
        assert_eq!(config.sample_ratio("carnitas", "/f/a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "/f/b"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.0);

        // Instance overrides.
        let config: SamplingConfig = "alpastor:a=0.5,b=0.15,carnitas:0.01,1.0".parse()?;
        assert_eq!(config.by_regex.len(), 4);
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.01);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.15);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.01);
        assert_eq!(config.sample_ratio("alpastor", "a"), 0.5);
        assert_eq!(config.sample_ratio("alpastor", "b"), 0.15);
        assert_eq!(config.sample_ratio("alpastor", "c"), 1.0);
        assert_eq!(config.sample_ratio("chorizo", "a"), 1.0);
        assert_eq!(config.sample_ratio("chorizo", "b"), 0.15);
        assert_eq!(config.sample_ratio("chorizo", "c"), 1.0);

        // Invalid configs.
        let err = "a=a".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Failed to parse sampling rate"));

        let err = "a:a:a=1.0".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Too many parts"));

        let err = "a:a=a=1.0".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Too many parts"));

        Ok(())
    }

    #[test]
    fn test_parse_sampling_config_json() -> anyhow::Result<()> {
        let config: SamplingConfig = r#"{ "defaultFraction": 1.0 }"#.parse()?;
        assert_eq!(config.by_regex.len(), 1);
        assert_eq!(config.sample_ratio("carnitas", "a"), 1.0);

        let config: SamplingConfig = r#"{
            "routeOverrides": [
                { "routeRegexp": "a", "fraction": 0.5 },
                { "routeRegexp": "b", "fraction": 0.15 }
            ],
            "defaultFraction": 0.0
        }"#
        .parse()?;
        assert_eq!(config.by_regex.len(), 3);
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.15);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.0);

        let config: SamplingConfig = r#"{
            "routeOverrides": [
                { "routeRegexp": "a", "fraction": 0.5 },
                { "routeRegexp": "b", "fraction": 0.15 }
            ],
            "defaultFraction": 0.01
        }"#
        .parse()?;
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.15);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.01);

        let config: SamplingConfig = r#"{
            "routeOverrides": [
                { "routeRegexp": "/f/.*", "fraction": 0.5 }
            ],
            "defaultFraction": 0.0
        }"#
        .parse()?;
        assert_eq!(config.sample_ratio("carnitas", "/f/a"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "/f/b"), 0.5);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.0);

        // Instance overrides.
        let config: SamplingConfig = r#"{
            "instanceOverrides": {
                "alpastor": [
                    { "routeRegexp": "a", "fraction": 0.5 },
                    { "routeRegexp": "c", "fraction": 0.5 }
                ],
                "carnitas": [ { "routeRegexp": ".*", "fraction": 0.01 } ]
            },
            "routeOverrides": [
                { "routeRegexp": "b", "fraction": 0.15 }
            ],
            "defaultFraction": 1.0
        }"#
        .parse()?;
        assert_eq!(config.sample_ratio("carnitas", "a"), 0.01);
        assert_eq!(config.sample_ratio("carnitas", "b"), 0.01);
        assert_eq!(config.sample_ratio("carnitas", "c"), 0.01);
        assert_eq!(config.sample_ratio("alpastor", "a"), 0.5);
        assert_eq!(config.sample_ratio("alpastor", "b"), 0.15);
        assert_eq!(config.sample_ratio("alpastor", "c"), 0.5);
        assert_eq!(config.sample_ratio("chorizo", "a"), 1.0);
        assert_eq!(config.sample_ratio("chorizo", "b"), 0.15);
        assert_eq!(config.sample_ratio("chorizo", "c"), 1.0);

        // Invalid configs.
        let err = "{ defaultFraction: 1.0 }"
            .parse::<SamplingConfig>()
            .unwrap_err();
        assert!(format!("{}", err).contains("Failed to parse sampling config as JSON"));

        let err = r#"{ "defaultFraction": 4.0 }"#.parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Invalid fraction 4 in default"));

        let err = r#"{
            "defaultFraction": 1.0,
            "routeOverrides": [{ "routeRegexp": "(", "fraction": 0.5 }]
        }"#
        .parse::<SamplingConfig>()
        .unwrap_err();
        assert!(format!("{}", err).contains("Invalid route regexp"));

        Ok(())
    }
}
