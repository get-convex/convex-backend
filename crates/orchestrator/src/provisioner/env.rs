//! Pure env composition: layer base + tier defaults + project overrides
//! into a single `BTreeMap<String, String>` that the Docker provisioner
//! turns into `-e KEY=VAL` flags.

use std::collections::BTreeMap;

pub fn compose_env<'a, I, J>(
    base: I,
    tier_defaults: &[(&str, &str)],
    overrides: J,
) -> BTreeMap<String, String>
where
    I: IntoIterator<Item = (&'a str, &'a str)>,
    J: IntoIterator<Item = (String, String)>,
{
    let mut out = BTreeMap::new();
    for (k, v) in base {
        out.insert(k.to_string(), v.to_string());
    }
    for (k, v) in tier_defaults {
        out.insert(k.to_string(), v.to_string());
    }
    for (k, v) in overrides {
        out.insert(k, v);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn map<const N: usize>(pairs: [(&str, &str); N]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn base_only() {
        let r = compose_env(
            [("A", "1"), ("B", "2")],
            &[],
            std::iter::empty::<(String, String)>(),
        );
        assert_eq!(r, map([("A", "1"), ("B", "2")]));
    }

    #[test]
    fn tier_overrides_base() {
        let r = compose_env(
            [("A", "1"), ("B", "2")],
            &[("A", "tier")],
            std::iter::empty::<(String, String)>(),
        );
        assert_eq!(r, map([("A", "tier"), ("B", "2")]));
    }

    #[test]
    fn project_override_wins_over_tier_and_base() {
        let r = compose_env(
            [("A", "base")],
            &[("A", "tier")],
            [("A".to_string(), "override".to_string())],
        );
        assert_eq!(r, map([("A", "override")]));
    }

    #[test]
    fn project_override_introduces_new_key() {
        let r = compose_env(
            [("A", "1")],
            &[],
            [("NEW_KNOB".to_string(), "42".to_string())],
        );
        assert_eq!(r, map([("A", "1"), ("NEW_KNOB", "42")]));
    }

    #[test]
    fn deterministic_ordering() {
        // BTreeMap guarantees lexicographic; this future-proofs against a
        // switch to HashMap that would make `docker run` invocations
        // non-deterministic across processes.
        let r = compose_env(
            [("Z", "1"), ("A", "2"), ("M", "3")],
            &[],
            std::iter::empty::<(String, String)>(),
        );
        let keys: Vec<_> = r.keys().collect();
        assert_eq!(keys, &["A", "M", "Z"]);
    }
}
