//! Common utilities across multiple files.

use std::{
    ops::Deref,
    sync::LazyLock,
};

use rand;
use regex::Regex;
pub use value::utils::{
    display_map,
    display_sequence,
};

#[derive(Clone)]
pub struct ReadOnly<T>(T);

impl<T> ReadOnly<T> {
    pub fn new(inner: T) -> Self {
        Self(inner)
    }
}

impl<T> Deref for ReadOnly<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.0
    }
}

/// Captures a string ending in ` (number)`
static NAME_NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(.*) \((\d+)\)$").unwrap());

/// "Increment" a string.
/// E.g. `Foo` becomes `Foo (1)`, and `Foo (1)` becomes `Foo (2)`.
/// If amt is None, uses 8 random hex characters instead.
/// Useful for strings (team names, device names) remain unique.
pub fn increment_name(name: &str, amt: Option<u64>) -> String {
    match amt {
        Some(amt) => {
            if let Some(number) = NAME_NUMBER_RE
                .captures(name)
                .and_then(|c| c.get(2))
                .and_then(|m| m.as_str().parse::<u64>().ok())
            {
                NAME_NUMBER_RE
                    .replace(name, |caps: &regex::Captures| {
                        format!("{} ({})", &caps[1], number + amt)
                    })
                    .into()
            } else {
                format!("{name} ({amt})")
            }
        },
        None => {
            let hex_suffix: String = (0..8)
                .map(|_| format!("{:x}", rand::random::<u8>() % 16))
                .collect();
            format!("{name} ({hex_suffix})")
        },
    }
}

/// Ensures that we are always running Convex services in UTC.
pub fn ensure_utc() -> anyhow::Result<()> {
    if let Ok(val) = std::env::var("TZ")
        && val != "UTC"
    {
        anyhow::bail!("TZ is set, but Convex requires UTC. Unset TZ to continue.")
    }
    unsafe { std::env::set_var("TZ", "UTC") };

    Ok(())
}

#[test]
fn test_increment_name() {
    let cases = [
        ("Foo", "Foo (1)"),
        ("Foo (1)", "Foo (2)"),
        ("Foo's (1) Bar (1001)", "Foo's (1) Bar (1002)"),
        ("Foo (1", "Foo (1 (1)"),
        ("Foo (a)", "Foo (a) (1)"),
    ];
    for (test, expected) in cases {
        assert_eq!(increment_name(test, Some(1)), expected);
    }

    assert_eq!(increment_name("Foo", Some(50)), "Foo (50)");
    assert_eq!(increment_name("Foo (20)", Some(50)), "Foo (70)");

    // Test None case - should generate 8 hex characters
    let result = increment_name("Test", None);
    assert!(result.starts_with("Test ("));
    assert!(result.ends_with(")"));
    let hex_part = &result[6..result.len() - 1]; // Extract hex part between "Test (" and ")"
    assert_eq!(hex_part.len(), 8);
    assert!(hex_part.chars().all(|c| c.is_ascii_hexdigit()));
}
