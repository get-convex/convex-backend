#[cfg(test)]
mod tests;
pub mod tracker;

/// What is the maximum length of a single text term? We will silently drop
/// terms that exceed this length.
///
/// TODO: Or should we truncate these to a prefix?
pub const MAX_TEXT_TERM_LENGTH: usize = 32;
