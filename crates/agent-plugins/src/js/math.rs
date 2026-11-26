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

/// `Math.round(x)`: the nearest integer, a half toward `+Infinity` (Rust's `f64::round` takes it
/// away from zero), `-0` for a value in `[-0.5, 0)`, and a value with no fraction as it is.
#[must_use]
pub fn math_round(value: f64) -> f64 {
    if !value.is_finite() || value.abs() >= 4_503_599_627_370_496.0 {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    };
    if rounded == 0.0 && value.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::{math_min, math_round};

    #[test]
    fn round_is_javascripts() {
        assert_eq!(math_round(2.5).to_bits(), 3.0_f64.to_bits());
        assert_eq!(math_round(-2.5).to_bits(), (-2.0_f64).to_bits());
        assert_eq!(math_round(2.4).to_bits(), 2.0_f64.to_bits());
        assert_eq!(
            math_round(0.499_999_999_999_999_94).to_bits(),
            0.0_f64.to_bits()
        );
        assert!(math_round(-0.3).is_sign_negative());
        assert!(math_round(f64::NAN).is_nan());
        assert_eq!(math_round(f64::INFINITY), f64::INFINITY);
    }

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
