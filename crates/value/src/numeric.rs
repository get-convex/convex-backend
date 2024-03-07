//! Helpers for working with floating point numbers.
use std::cmp::Ordering;

/// Is a floating point number an integer? Note that we do not consider negative
/// zero an integer, since we want there to be an injection between the subset
/// of `is_integral` floating point numbers and the integers.
#[allow(clippy::float_cmp)]
pub fn is_integral(n: f64) -> Option<i64> {
    // `n == n.trunc()` isn't optimized very well by the compiler. If this is
    // ever important, we just need to check that 1) the f64 is normal and 2)
    // the exponent is greater than (64 - the number of trailing zeros of the
    // mantissa) to ensure we shift the decimal point past all of the mantissa's
    // digits. Note also that subnormal numbers are never integers.
    // See https://stackoverflow.com/questions/26341494 for more color.
    let truncated = n.trunc();
    if !is_negative_zero(n) && !n.is_infinite() && truncated == n {
        Some(truncated as i64)
    } else {
        None
    }
}

/// Is a floating point number native zero?
pub fn is_negative_zero(n: f64) -> bool {
    matches!(n.total_cmp(&-0.0), Ordering::Equal)
}
