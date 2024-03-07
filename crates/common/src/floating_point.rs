/// Lowest [`i64`] that can be exactly represented by a [`f64`].
pub const MIN_EXACT_F64_INT: i64 = -(1 << f64::MANTISSA_DIGITS);
/// Highest [`i64`] that can be exactly represented by a [`f64`].
pub const MAX_EXACT_F64_INT: i64 = 1 << f64::MANTISSA_DIGITS;

/// Assert that two float values are close together.
pub fn assert_approx_equal(left: f64, right: f64) {
    let diff = (left - right).abs();
    let add = left.abs() + right.abs();
    if diff > 0.0005 * add {
        panic!("assertion failed: `(left ~= right) left: {left} right: {right}");
    }
}
