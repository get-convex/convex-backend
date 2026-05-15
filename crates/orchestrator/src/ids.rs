//! ID, slug, and deployment-name generators.

use rand::{
    distr::Alphanumeric,
    Rng,
};

/// Generate a 16-character lowercase alphanumeric ID.
pub fn random_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(|c| (c as char).to_ascii_lowercase())
        .collect()
}

/// Slugify a name into a URL-safe slug. Strips non-alphanumeric chars,
/// lowercases, and squashes runs of `-`.
pub fn slugify(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut last_dash = true;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("untitled");
    }
    out
}

const ADJECTIVES: &[&str] = &[
    "happy", "fast", "bold", "calm", "brave", "merry", "kind", "swift",
    "quiet", "sunny", "lucky", "wise", "rapid", "noble", "eager", "fancy",
    "shiny", "spry", "gentle", "jolly",
];

const ANIMALS: &[&str] = &[
    "otter", "fox", "owl", "deer", "bear", "wolf", "lynx", "moose",
    "hawk", "puma", "tiger", "panda", "yak", "ibis", "elk", "boar",
    "crane", "shark", "whale", "raven",
];

/// Generate a `happy-otter-123` style deployment name.
pub fn random_deployment_name() -> String {
    let mut rng = rand::rng();
    let adj = ADJECTIVES[rng.random_range(0..ADJECTIVES.len())];
    let animal = ANIMALS[rng.random_range(0..ANIMALS.len())];
    let n: u16 = rng.random_range(100..1000);
    format!("{adj}-{animal}-{n}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("My Project!"), "my-project");
        assert_eq!(slugify("--leading"), "leading");
        assert_eq!(slugify("trailing--"), "trailing");
        assert_eq!(slugify(""), "untitled");
    }

    #[test]
    fn random_deployment_name_format() {
        let name = random_deployment_name();
        let parts: Vec<&str> = name.split('-').collect();
        assert_eq!(parts.len(), 3, "got {name}");
        assert!(parts[2].chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn random_id_length() {
        assert_eq!(random_id().len(), 16);
    }
}
