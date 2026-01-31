//! Shared utilities for custom log attributes in UDFs and actions.
//!
//! This module provides validation and merging logic for custom log attributes
//! that can be set via `ctx.setLogAttributes()` in user functions.

use serde_json::Value as JsonValue;

/// Maximum number of custom log attributes allowed
pub const MAX_ATTRIBUTES: usize = 10;

/// Maximum serialized size of custom log attributes in bytes
pub const MAX_SIZE_BYTES: usize = 1024;

/// Maximum length for attribute keys
pub const MAX_KEY_LENGTH: usize = 64;

/// Check if a single attribute is valid, returning an error message if not.
///
/// Validation rules:
/// - Key must be <= 64 characters
/// - Key must contain only alphanumeric characters, underscores, and dots
/// - Value must be string, number, or boolean
pub fn validate_log_attribute(key: &str, value: &JsonValue) -> Result<(), String> {
    // Key validation: alphanumeric + underscores + dots, max 64 chars
    if key.len() > MAX_KEY_LENGTH {
        return Err(format!(
            "setLogAttributes: key '{}' exceeds max length of {MAX_KEY_LENGTH} chars",
            &key[..32]
        ));
    }
    if !key
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
    {
        return Err(format!(
            "setLogAttributes: key '{key}' must contain only alphanumeric characters, \
             underscores, and dots"
        ));
    }

    // Values must be string, number, or boolean
    match value {
        JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Bool(_) => Ok(()),
        _ => Err(format!(
            "setLogAttributes: value for key '{}' must be string, number, or boolean (got {})",
            key,
            match value {
                JsonValue::Null => "null",
                JsonValue::Array(_) => "array",
                JsonValue::Object(_) => "object",
                _ => "unknown",
            }
        )),
    }
}

/// Merge new attributes into existing ones leniently.
///
/// This function:
/// - Skips invalid attributes (with warnings)
/// - Stops adding if max attribute count is reached
/// - Reverts additions that would exceed max size
///
/// Returns a list of warning messages for any skipped attributes.
pub fn merge_custom_log_attributes_lenient(
    existing: &mut serde_json::Map<String, JsonValue>,
    attrs: serde_json::Map<String, JsonValue>,
) -> Vec<String> {
    let mut warnings = Vec::new();

    // Add new attributes, filtering out invalid ones
    for (key, value) in attrs {
        // Validate the attribute
        if let Err(msg) = validate_log_attribute(&key, &value) {
            warnings.push(msg);
            continue;
        }

        // Check if adding this attribute would exceed the key limit
        // (only count if it's a new key, not an update)
        if !existing.contains_key(&key) && existing.len() >= MAX_ATTRIBUTES {
            warnings.push(format!(
                "setLogAttributes: skipping key '{key}' - maximum of {MAX_ATTRIBUTES} attributes \
                 reached"
            ));
            continue;
        }

        // Tentatively add/update the attribute
        let old_value = existing.insert(key.clone(), value.clone());

        // Check if we've exceeded the size limit
        if let Ok(serialized) = serde_json::to_string(&existing)
            && serialized.len() > MAX_SIZE_BYTES
        {
            // Revert this addition - it made us too large
            if let Some(old) = old_value {
                existing.insert(key.clone(), old);
            } else {
                existing.remove(&key);
            }
            warnings.push(format!(
                "setLogAttributes: skipping key '{key}' - would exceed max size of \
                 {MAX_SIZE_BYTES} bytes"
            ));
            // Stop adding more attributes
            break;
        }
    }

    warnings
}

/// Validate custom log attributes strictly (used for testing).
///
/// Unlike `merge_custom_log_attributes_lenient`, this returns an error
/// if any validation fails rather than skipping invalid attributes.
#[cfg(test)]
pub fn validate_log_attributes(attrs: &serde_json::Map<String, JsonValue>) -> anyhow::Result<()> {
    use errors::ErrorMetadata;

    // Max 10 attribute keys
    if attrs.len() > MAX_ATTRIBUTES {
        anyhow::bail!(ErrorMetadata::bad_request(
            "TooManyLogAttributes",
            format!(
                "Too many log attributes: {} (max {MAX_ATTRIBUTES})",
                attrs.len()
            )
        ));
    }

    // Max 1KB total serialized size
    let size = serde_json::to_string(attrs)
        .map_err(|e| anyhow::anyhow!("Failed to serialize log attributes: {e}"))?
        .len();
    if size > MAX_SIZE_BYTES {
        anyhow::bail!(ErrorMetadata::bad_request(
            "LogAttributesTooLarge",
            format!("Log attributes too large: {size} bytes (max {MAX_SIZE_BYTES})")
        ));
    }

    for (key, value) in attrs {
        // Key validation: alphanumeric + underscores + dots, max 64 chars
        if key.len() > MAX_KEY_LENGTH {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidLogAttributeKey",
                format!(
                    "Log attribute key too long: {} (max {MAX_KEY_LENGTH} chars)",
                    key.len()
                )
            ));
        }
        if !key
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '.')
        {
            anyhow::bail!(ErrorMetadata::bad_request(
                "InvalidLogAttributeKey",
                format!(
                    "Log attribute key must be alphanumeric with underscores or dots: {}",
                    key
                )
            ));
        }

        // Values must be string, number, or boolean
        match value {
            JsonValue::String(_) | JsonValue::Number(_) | JsonValue::Bool(_) => {},
            _ => {
                anyhow::bail!(ErrorMetadata::bad_request(
                    "InvalidLogAttributeValue",
                    format!(
                        "Log attribute value must be string, number, or boolean for key: {}",
                        key
                    )
                ));
            },
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_validate_log_attribute_valid() {
        assert!(validate_log_attribute("valid_key", &json!("string")).is_ok());
        assert!(validate_log_attribute("key123", &json!(42)).is_ok());
        assert!(validate_log_attribute("key", &json!(true)).is_ok());
    }

    #[test]
    fn test_validate_log_attribute_dot_separated_keys() {
        // OTel-style dot-separated keys should be valid
        assert!(validate_log_attribute("http.method", &json!("POST")).is_ok());
        assert!(validate_log_attribute("user.id", &json!("123")).is_ok());
        assert!(validate_log_attribute("service.name", &json!("my_service")).is_ok());
    }

    #[test]
    fn test_validate_log_attribute_invalid_key_chars() {
        let result = validate_log_attribute("invalid-key", &json!("value"));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("alphanumeric characters, underscores, and dots"));
    }

    #[test]
    fn test_validate_log_attribute_key_too_long() {
        let long_key = "a".repeat(65);
        let result = validate_log_attribute(&long_key, &json!("value"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceeds max length"));
    }

    #[test]
    fn test_validate_log_attribute_invalid_value_type() {
        assert!(validate_log_attribute("key", &json!(null)).is_err());
        assert!(validate_log_attribute("key", &json!([1, 2, 3])).is_err());
        assert!(validate_log_attribute("key", &json!({"nested": "object"})).is_err());
    }

    #[test]
    fn test_merge_skips_invalid_attributes() {
        let mut existing = serde_json::Map::new();
        let mut attrs = serde_json::Map::new();
        attrs.insert("valid_key".to_string(), json!("value"));
        attrs.insert("invalid-key".to_string(), json!("value"));

        let warnings = merge_custom_log_attributes_lenient(&mut existing, attrs);

        assert_eq!(existing.len(), 1);
        assert!(existing.contains_key("valid_key"));
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn test_merge_respects_max_attributes() {
        let mut existing = serde_json::Map::new();
        for i in 0..MAX_ATTRIBUTES {
            existing.insert(format!("key{i}"), json!("value"));
        }

        let mut attrs = serde_json::Map::new();
        attrs.insert("new_key".to_string(), json!("value"));

        let warnings = merge_custom_log_attributes_lenient(&mut existing, attrs);

        assert_eq!(existing.len(), MAX_ATTRIBUTES);
        assert!(!existing.contains_key("new_key"));
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("maximum of"));
    }

    #[test]
    fn test_merge_allows_update_at_max() {
        let mut existing = serde_json::Map::new();
        for i in 0..MAX_ATTRIBUTES {
            existing.insert(format!("key{i}"), json!("original"));
        }

        let mut attrs = serde_json::Map::new();
        attrs.insert("key0".to_string(), json!("updated"));

        let warnings = merge_custom_log_attributes_lenient(&mut existing, attrs);

        assert_eq!(existing.len(), MAX_ATTRIBUTES);
        assert_eq!(existing.get("key0"), Some(&json!("updated")));
        assert!(warnings.is_empty());
    }

    #[test]
    fn test_validate_log_attributes_too_many() {
        let mut attrs = serde_json::Map::new();
        for i in 0..=MAX_ATTRIBUTES {
            attrs.insert(format!("key{i}"), json!("value"));
        }

        let result = validate_log_attributes(&attrs);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_log_attributes_valid() {
        let mut attrs = serde_json::Map::new();
        attrs.insert("key1".to_string(), json!("value1"));
        attrs.insert("key2".to_string(), json!(42));
        attrs.insert("http.method".to_string(), json!("GET"));

        assert!(validate_log_attributes(&attrs).is_ok());
    }
}
