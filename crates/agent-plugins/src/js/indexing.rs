//! JavaScript's numbers where the package indexes or counts with one: `array[index]` for a number
//! that may be no index, and an array's `length` as a number.

/// `items[index]` for a number `index`: the element when it is a whole number inside the array
/// (`-0` is `0`), else `None`, which is `undefined` (`items[1.5]`, `items[-1]` and `items[NaN]`
/// name no element).
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the index is a whole number in 0..length, which a usize and a double both hold"
)]
pub fn element_at<T>(items: &[T], index: f64) -> Option<&T> {
    let inside = index.fract() == 0.0 && index >= 0.0 && index < items.len() as f64;
    if inside {
        items.get(index as usize)
    } else {
        None
    }
}

/// An array's length as the number `length` is.
#[must_use]
pub fn length_number(length: usize) -> f64 {
    f64::from(u32::try_from(length).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod tests {
    use super::{element_at, length_number};

    #[test]
    fn only_a_whole_number_inside_the_array_is_an_index() {
        let items = ["a", "b"];
        assert_eq!(element_at(&items, 0.0), Some(&"a"));
        assert_eq!(element_at(&items, -0.0), Some(&"a"));
        assert_eq!(element_at(&items, 1.0), Some(&"b"));
        assert_eq!(element_at(&items, 2.0), None);
        assert_eq!(element_at(&items, 1.5), None);
        assert_eq!(element_at(&items, -1.0), None);
        assert_eq!(element_at(&items, f64::NAN), None);
        assert_eq!(element_at(&items, f64::INFINITY), None);
    }

    #[test]
    fn a_length_is_a_number() {
        assert!((length_number(3) - 3.0).abs() < f64::EPSILON);
    }
}
