//! JavaScript's `slice`, of a string and of an array: its indexes, and a string's counted in
//! UTF-16 code units.

/// JavaScript's `text.slice(start, end)` with indexes from 0 up, counted in UTF-16 code units
/// (`None` is to the end; an index past the end is the end). A cut inside a character above
/// U+FFFF leaves half of it, a lone surrogate, which reads as U+FFFD here (docs/RUST.md, "Where
/// the crate answers differently").
#[must_use]
pub fn slice_code_units(text: &str, start: usize, end: Option<usize>) -> String {
    let units: Vec<u16> = text.encode_utf16().collect();
    let end = end.unwrap_or(units.len()).min(units.len());
    String::from_utf16_lossy(units.get(start.min(end)..end).unwrap_or_default())
}

/// An index argument of JavaScript's `slice` (of an array or a string) as the position it names in
/// a sequence of `length` items: its integer part (NaN is 0), counted from the end when negative,
/// and kept within `0..=length`.
#[must_use]
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "the value is clamped to 0..=length, a whole number a usize holds, before the cast"
)]
pub fn slice_index(value: f64, length: usize) -> usize {
    let length_number = length as f64;
    let integer = if value.is_nan() { 0.0 } else { value.trunc() };
    let position = if integer < 0.0 {
        (length_number + integer).max(0.0)
    } else {
        integer.min(length_number)
    };
    position as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'const t = "a\u{1F600}b"; console.log(JSON.stringify([t.slice(1), t.slice(2),
    //   t.slice(0, 2), t.slice(3, 1), t.slice(9)]))' prints ["😀b","\ude00b","a\ud83d","",""].
    #[test]
    fn slices_count_code_units() {
        let text = "a\u{1F600}b";
        assert_eq!(slice_code_units(text, 1, None), "\u{1F600}b");
        assert_eq!(slice_code_units(text, 2, None), "\u{FFFD}b");
        assert_eq!(slice_code_units(text, 0, Some(2)), "a\u{FFFD}");
        assert_eq!(slice_code_units(text, 3, Some(1)), "");
        assert_eq!(slice_code_units(text, 9, None), "");
    }

    // bun -e 'const a = [0, 1, 2, 3, 4]; console.log(JSON.stringify([a.slice(1.7), a.slice(-2),
    //   a.slice(-9), a.slice(NaN), a.slice(1e300), a.slice(0, 2.9), a.slice(Infinity)]))'
    // prints [[1,2,3,4],[3,4],[0,1,2,3,4],[0,1,2,3,4],[],[0,1],[]].
    #[test]
    fn slice_indexes_clamp_as_javascript_does() {
        assert_eq!(slice_index(1.7, 5), 1);
        assert_eq!(slice_index(-2.0, 5), 3);
        assert_eq!(slice_index(-9.0, 5), 0);
        assert_eq!(slice_index(f64::NAN, 5), 0);
        assert_eq!(slice_index(1e300, 5), 5);
        assert_eq!(slice_index(2.9, 5), 2);
        assert_eq!(slice_index(f64::INFINITY, 5), 5);
        assert_eq!(slice_index(f64::NEG_INFINITY, 5), 0);
    }
}
