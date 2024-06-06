use std::{
    collections::BTreeMap,
    str::FromStr,
};

use anyhow::Context;
use minitrace::{
    collector::SpanContext,
    Span,
};
use rand::Rng;
use regex::Regex;

use crate::knobs::REQUEST_TRACE_SAMPLE_CONFIG;

#[derive(Clone)]
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
    name: &str,
    rng: &mut R,
    properties: BTreeMap<String, String>,
) -> Span {
    let sample_ratio = REQUEST_TRACE_SAMPLE_CONFIG.sample_ratio(name);
    let should_sample = rng.gen_bool(sample_ratio);
    match should_sample {
        true => Span::root(name.to_owned(), SpanContext::random()).with_properties(|| properties),
        false => Span::noop(),
    }
}

#[derive(Debug)]
pub struct SamplingConfig {
    global: f64,
    by_name_regex: BTreeMap<String, (Regex, f64)>,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        // No sampling by default
        Self {
            global: 0.0,
            by_name_regex: BTreeMap::new(),
        }
    }
}

impl SamplingConfig {
    fn sample_ratio(&self, name: &str) -> f64 {
        self.by_name_regex
            .values()
            .find_map(|(name_regex, sample_ratio)| {
                if name_regex.is_match(name) {
                    Some(*sample_ratio)
                } else {
                    None
                }
            })
            .unwrap_or(self.global)
    }
}

impl FromStr for SamplingConfig {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let mut global = None;
        let mut by_name_regex = BTreeMap::new();
        for token in s.split(',') {
            let parts: Vec<_> = token.split('=').map(|s| s.trim()).collect();
            anyhow::ensure!(parts.len() <= 2, "Too many parts {}", token);
            if parts.len() == 2 {
                let name = parts[0];
                let name_regex = Regex::new(name).context("Failed to parse name regex")?;
                let rate: f64 = parts[1].parse().context("Failed to parse sampling rate")?;
                let old_value = by_name_regex.insert(name.to_owned(), (name_regex, rate));
                anyhow::ensure!(old_value.is_none(), "{} set more than once", name);
            } else {
                let rate: f64 = parts[0].parse().context("Failed to parse sampling rate")?;
                anyhow::ensure!(global.is_none(), "Global sampling rate set more than once");
                global = Some(rate)
            }
        }
        Ok(SamplingConfig {
            global: global.unwrap_or(0.0),
            by_name_regex,
        })
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
    use crate::minitrace_helpers::SamplingConfig;

    #[test]
    fn test_parse_sampling_config() -> anyhow::Result<()> {
        let config: SamplingConfig = "1".parse()?;
        assert_eq!(config.global, 1.0);
        assert_eq!(config.by_name_regex.len(), 0);
        assert_eq!(config.sample_ratio("a"), 1.0);

        let config: SamplingConfig = "a=0.5,b=0.15".parse()?;
        assert_eq!(config.global, 0.0);
        assert_eq!(config.by_name_regex.len(), 2);
        assert_eq!(config.sample_ratio("a"), 0.5);
        assert_eq!(config.sample_ratio("b"), 0.15);
        assert_eq!(config.sample_ratio("c"), 0.0);

        let config: SamplingConfig = "a=0.5,b=0.15,0.01".parse()?;
        assert_eq!(config.global, 0.01);
        assert_eq!(config.by_name_regex.len(), 2);
        assert_eq!(config.by_name_regex.len(), 2);
        assert_eq!(config.sample_ratio("a"), 0.5);
        assert_eq!(config.sample_ratio("b"), 0.15);
        assert_eq!(config.sample_ratio("c"), 0.01);

        let config: SamplingConfig = "/f/.*=0.5".parse()?;
        assert_eq!(config.by_name_regex.len(), 1);
        assert_eq!(config.sample_ratio("/f/a"), 0.5);
        assert_eq!(config.sample_ratio("/f/b"), 0.5);
        assert_eq!(config.sample_ratio("c"), 0.0);

        // Invalid configs.
        let err = "100,200".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Global sampling rate set more than once"));

        let err = "a=a".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("Failed to parse sampling rate"));

        let err = "a=0.1,a=0.2".parse::<SamplingConfig>().unwrap_err();
        assert!(format!("{}", err).contains("a set more than once"));

        Ok(())
    }
}
