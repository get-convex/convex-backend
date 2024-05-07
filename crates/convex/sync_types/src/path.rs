use crate::identifier::MAX_IDENTIFIER_LEN;

pub fn check_valid_path_component(s: &str) -> anyhow::Result<()> {
    if s.len() > MAX_IDENTIFIER_LEN {
        anyhow::bail!(
            "Path component is too long ({} > maximum {}).",
            s.len(),
            MAX_IDENTIFIER_LEN
        );
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
    {
        anyhow::bail!(
            "Path component {s} can only contain alphanumeric characters, underscores, or periods."
        );
    }
    if !s.chars().any(|c| c.is_ascii_alphanumeric()) {
        anyhow::bail!("Path component {s} must have at least one alphanumeric character.");
    }
    Ok(())
}
