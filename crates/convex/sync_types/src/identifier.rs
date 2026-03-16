use std::{
    fmt::{
        self,
        Display,
    },
    ops::Deref,
    str::FromStr,
};

pub const MAX_IDENTIFIER_LEN: usize = 64;

pub const IDENTIFIER_REQUIREMENTS: &str =
    "Identifiers must start with a letter and can only contain letters, digits, and underscores.";

/// Check that a string can be used for a table name or field name in a
/// document.
///
/// We use a simplified ASCII version of Rust's syntax[^1] which also overlaps
/// with JavaScript's syntax[^2].
///
/// ```text
/// ident: start continue*
/// start: a-zA-z_
/// continue: a-zA-Z0-9_
/// ```
/// [^3] [^4]
/// To be conservative, let's also ban identifiers of entirely `_` too.
///
/// [^1]: <https://doc.rust-lang.org/reference/identifiers.html>
/// [^2]: <https://developer.mozilla.org/en-US/docs/Glossary/Identifier>
/// [^3]: <https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AXID_START%3A%5D&g=&i=>
/// [^4]: <https://util.unicode.org/UnicodeJsps/list-unicodeset.jsp?a=%5B%3AXID_CONTINUE%3A%5D&g=&i=>
pub fn check_valid_identifier(s: &str) -> anyhow::Result<()> {
    check_valid_identifier_inner(s, |e| anyhow::anyhow!(e.to_string()))
}

pub fn is_valid_identifier(s: &str) -> bool {
    check_valid_identifier_inner(s, |_| ()).is_ok()
}

#[derive(Clone, Debug, PartialEq, PartialOrd, Ord, Eq, Hash)]
pub struct Identifier(String);

impl Identifier {
    pub fn min() -> Self {
        Identifier(MIN_IDENTIFIER.to_string())
    }

    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }
}

impl FromStr for Identifier {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        check_valid_identifier(s)?;
        Ok(Identifier(s.to_string()))
    }
}

impl From<Identifier> for String {
    fn from(id: Identifier) -> String {
        id.0
    }
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Deref for Identifier {
    type Target = str;

    fn deref(&self) -> &str {
        &self.0
    }
}

fn check_valid_identifier_inner<E>(
    s: &str,
    error: impl FnOnce(fmt::Arguments<'_>) -> E,
) -> Result<(), E> {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() => (),
        Some('_') => (),
        Some(c) => {
            return Err(error(format_args!(
                "Invalid first character {c:?} in {s}: Identifiers must start with an alphabetic \
                 character or underscore"
            )))
        },
        None => return Err(error(format_args!("Identifier cannot be empty"))),
    };
    for c in chars {
        if !c.is_ascii_alphanumeric() && c != '_' {
            return Err(error(format_args!(
                "Identifier {s} has invalid character {c:?}: Identifiers can only contain \
                 alphanumeric characters or underscores"
            )));
        }
    }
    if s.len() > MAX_IDENTIFIER_LEN {
        return Err(error(format_args!(
            "Identifier is too long ({} > maximum {})",
            s.len(),
            MAX_IDENTIFIER_LEN
        )));
    }
    if s.chars().all(|c| c == '_') {
        return Err(error(format_args!(
            "Identifier {s} cannot have exclusively underscores"
        )));
    }
    Ok(())
}

pub const MIN_IDENTIFIER: &str = "A";

pub const MAX_FIELD_NAME_LENGTH: usize = 1024;

/// Check that a string can be used as field in a Convex object.
///
/// Field names cannot start with '$', must contain only non-control ASCII
/// characters, and must be at most 1024 characters long.
pub fn check_valid_field_name(s: &str) -> anyhow::Result<()> {
    if is_valid_field_name(s) {
        return Ok(());
    }
    check_valid_field_name_slow(s)
}

pub fn is_valid_field_name(s: &str) -> bool {
    if s.starts_with('$') {
        return false;
    }
    if s.len() > MAX_FIELD_NAME_LENGTH {
        return false;
    }
    // Ideally this should use slice::as_chunks, but MSRV is 1.85 and that method is
    // only in 1.88
    let mut chunks = s.as_bytes().chunks_exact(16);
    for chunk in &mut chunks {
        let chunk = <[u8; 16]>::try_from(chunk).unwrap();
        // this strange construction convinces LLVM to vectorize the check
        if chunk.map(|c| !c.is_ascii() || c.is_ascii_control()) != [false; 16] {
            return false;
        }
    }
    if chunks
        .remainder()
        .iter()
        .any(|c| !c.is_ascii() || c.is_ascii_control())
    {
        return false;
    }
    true
}

#[cold]
fn check_valid_field_name_slow(s: &str) -> anyhow::Result<()> {
    if s.starts_with('$') {
        anyhow::bail!("Field name {s} starts with '$', which is reserved.");
    }
    for c in s.chars() {
        if !c.is_ascii() || c.is_ascii_control() {
            anyhow::bail!(
                "Field name {s} has invalid character {c:?}: Field names can only contain \
                 non-control ASCII characters"
            );
        }
    }
    if s.len() > MAX_FIELD_NAME_LENGTH {
        anyhow::bail!(
            "Field name is too long ({} > maximum {})",
            s.len(),
            MAX_FIELD_NAME_LENGTH
        );
    }
    Ok(())
}

#[cfg(any(test, feature = "testing"))]
pub mod arbitrary_regexes {
    use proptest::prelude::*;

    use super::Identifier;

    pub const IDENTIFIER_REGEX: &str = "[a-zA-Z_][a-zA-Z][a-zA-Z0-9_]{0,62}";
    pub const USER_IDENTIFIER_REGEX: &str = "[a-zA-Z][a-zA-Z0-9_]{0,63}";
    pub const SYSTEM_IDENTIFIER_REGEX: &str = "_[a-zA-Z][a-zA-Z0-9_]{0,62}";
    // ' ' through ~ is all non-control ASCII. First character cannot be `$` or
    // `_`. These can be longer, but keep them shorter for the sake of tests.
    pub const USER_FIELD_NAME_REGEX: &str = "([ -#%-^`-~][ -~]{0,63})?";
    // Technically this can be broader, but system fields are usually valid
    // identifiers
    pub const SYSTEM_FIELD_NAME_REGEX: &str = "_[a-zA-Z][a-zA-Z0-9_]{0,62}";

    impl Arbitrary for Identifier {
        type Parameters = ();
        type Strategy = BoxedStrategy<Self>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            prop_oneof![USER_IDENTIFIER_REGEX, SYSTEM_IDENTIFIER_REGEX]
                .prop_map(Identifier)
                .boxed()
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{
        arbitrary_regexes::IDENTIFIER_REGEX,
        check_valid_field_name,
        check_valid_identifier,
        is_valid_field_name,
        MIN_IDENTIFIER,
    };

    proptest! {
        #![proptest_config(
            ProptestConfig { failure_persistence: None, ..ProptestConfig::default() }
        )]

        #[test]
        fn test_min_identifier(ident in IDENTIFIER_REGEX) {
            assert!(MIN_IDENTIFIER <= &ident[..]);
        }

        #[test]
        fn test_field_name_fast_path(s in any::<String>()) {
            assert_eq!(is_valid_field_name(&s), check_valid_field_name(&s).is_ok());
        }
    }

    #[test]
    fn test_control_char_in_identifier_body() {
        let s = "abc\u{0010}def";
        let err = check_valid_identifier(s).unwrap_err().to_string();
        assert!(err.contains("'\\u{10}'"), "got: {err}");
    }

    #[test]
    fn test_control_char_as_identifier_start() {
        let s = "\u{0010}abc";
        let err = check_valid_identifier(s).unwrap_err().to_string();
        assert!(err.contains("'\\u{10}'"), "got: {err}");
    }

    #[test]
    fn test_control_char_in_field_name() {
        let s = "field\u{0010}name";
        let err = check_valid_field_name(s).unwrap_err().to_string();
        assert!(err.contains("'\\u{10}'"), "got: {err}");
    }

    #[test]
    fn test_newline_in_identifier() {
        let s = "abc\ndef";
        let err = check_valid_identifier(s).unwrap_err().to_string();
        assert!(err.contains("'\\n'"), "got: {err}");
    }

    #[test]
    fn test_regular_char_in_identifier_body() {
        let s = "abc@def";
        let err = check_valid_identifier(s).unwrap_err().to_string();
        assert!(err.contains("'@'"), "got: {err}");
    }

    #[test]
    fn test_regular_char_as_identifier_start() {
        let s = "9abc";
        let err = check_valid_identifier(s).unwrap_err().to_string();
        assert!(err.contains("'9'"), "got: {err}");
    }

    #[test]
    fn test_emoji_in_field_name() {
        let s = "field😀name";
        let err = check_valid_field_name(s).unwrap_err().to_string();
        assert!(err.contains("'😀'"), "got: {err}");
    }
}
