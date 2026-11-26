//! JavaScript's `Math` where Rust's `f64` answers differently.

/// `Math.min(first, second)`: `NaN` when either is `NaN` (Rust's `f64::min` takes the other), and
/// `-0` below `+0`.
#[must_use]
pub fn math_min(first: f64, second: f64) -> f64 {
    if first.is_nan() || second.is_nan() {
        return f64::NAN;
    }
    if first == second {
        return if first.is_sign_negative() {
            first
        } else {
            second
        };
    }
    first.min(second)
}

#[cfg(test)]
mod tests {
    use super::math_min;

    #[test]
    fn min_is_javascripts() {
        assert_eq!(math_min(1.0, 2.0).to_bits(), 1.0_f64.to_bits());
        assert!(math_min(f64::NAN, 2.0).is_nan());
        assert!(math_min(2.0, f64::NAN).is_nan());
        assert!(math_min(0.0, -0.0).is_sign_negative());
        assert!(math_min(-0.0, 0.0).is_sign_negative());
        assert_eq!(
            math_min(f64::NEG_INFINITY, 3.0).to_bits(),
            f64::NEG_INFINITY.to_bits()
        );
    }
}
