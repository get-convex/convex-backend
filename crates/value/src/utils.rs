use std::{
    fmt,
    fmt::Display,
};

fn display_composite<K: Display, V: Display, I: Iterator<Item = (Option<K>, V)>>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: I,
) -> fmt::Result {
    let mut first = true;
    write!(f, "{}", enclosing[0])?;
    for (key, value) in items {
        if !first {
            write!(f, ", ")?;
        }
        if let Some(key) = key {
            write!(f, "{key}: ")?;
        }
        write!(f, "{value}")?;
        first = false;
    }
    write!(f, "{}", enclosing[1])
}

/// Format an iterator of `items` with a comma separator and enclosed by
/// `enclosing[0]` and `enclosing[1]`.
pub fn display_sequence<V: Display>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: impl Iterator<Item = V>,
) -> fmt::Result {
    // Since we're passing in `None` for the key type, we need to pass in something
    // for the first type parameter to help type inference out.
    display_composite::<usize, V, _>(f, enclosing, items.map(|v| (None, v)))
}

/// Format an iterator of key-value pairs with a comma separator and enclosed by
/// `enclosing[0]` and `enclosing[1]`.
pub fn display_map<K: Display, V: Display>(
    f: &mut fmt::Formatter,
    enclosing: [&str; 2],
    items: impl Iterator<Item = (K, V)>,
) -> fmt::Result {
    display_composite(f, enclosing, items.map(|(k, v)| (Some(k), v)))
}
