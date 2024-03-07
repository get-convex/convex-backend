use errors::ErrorMetadata;

mod expression;
mod query;
pub use expression::JsonExpression;

#[cfg(test)]
mod tests;

pub fn invalid_json() -> ErrorMetadata {
    ErrorMetadata::bad_request("InvalidJson", "Invalid JSON")
}
