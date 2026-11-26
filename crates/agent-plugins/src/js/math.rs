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

/// `Math.round(value)`: the nearest integer, a tie toward `+Infinity` (`-2.5` is `-2`), `-0` for a
/// negative that rounds to zero, and `NaN`, the infinities and `±0` as they are. (`f64::round`
/// breaks a tie away from zero, and `floor(value + 0.5)` is wrong for `0.49999999999999994`.)
#[must_use]
pub fn math_round(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 || value.abs() >= 4_503_599_627_370_496.0 {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    };
    if rounded == 0.0 && value < 0.0 {
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
        assert_eq!(math_round(-0.2).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(math_round(0.49999999999999994).to_bits(), 0.0_f64.to_bits());
        assert_eq!(math_round(-0.5).to_bits(), (-0.0_f64).to_bits());
        assert_eq!(math_round(1.5).to_bits(), 2.0_f64.to_bits());
        assert!(math_round(f64::NAN).is_nan());
        assert_eq!(math_round(f64::INFINITY).to_bits(), f64::INFINITY.to_bits());
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
