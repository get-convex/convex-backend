//! Common utilities across multiple files.

use std::{
    ops::Deref,
    sync::LazyLock,
};

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
/// Useful for strings (team names, device names) remain unique.
pub fn increment_name(name: &str, amt: u64) -> String {
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
}

/// Ensures that we are always running Convex services in UTC.
pub fn ensure_utc() -> anyhow::Result<()> {
    if let Ok(val) = std::env::var("TZ")
        && val != "UTC"
    {
        anyhow::bail!("TZ is set, but Convex requires UTC. Unset TZ to continue.")
    }
    std::env::set_var("TZ", "UTC");

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
        assert_eq!(increment_name(test, 1), expected);
    }

    assert_eq!(increment_name("Foo", 50), "Foo (50)");
    assert_eq!(increment_name("Foo (20)", 50), "Foo (70)");
}
