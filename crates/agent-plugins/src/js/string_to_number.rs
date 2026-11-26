//! `Number(text)`: ECMAScript's StringToNumber (ECMA-262 §7.1.4.1.1), as the npm package reads a
//! number from a text. Rust's `str::parse::<f64>` reads another grammar (`inf`, `nan`, any case),
//! so the text is matched to JavaScript's first and only then read.

use super::trim;

/// The number a text spells for JavaScript's `Number(text)`, `NaN` when it spells none.
///
/// The blanks around it are `trim`'s (StrWhiteSpaceChar is WhiteSpace and LineTerminator, the
/// same set); a text of blanks only is `0`. Then either `0x`, `0o` or `0b` (any case) and at
/// least one digit of that base, with no sign, or a StrDecimalLiteral: an optional `+` or `-`,
/// then `Infinity`, or digits with an optional `.` and fraction (a side may be empty, not both)
/// and an optional exponent `e`/`E`, sign, digits. No `_` separator, no hexadecimal fraction.
/// Either form reads as the nearest double, ties to even.
#[must_use]
pub fn number(text: &str) -> f64 {
    let text = trim(text);
    if text.is_empty() {
        return 0.0;
    }
    let prefix = text.get(..2).map(str::to_ascii_lowercase);
    let width = match prefix.as_deref() {
        Some("0x") => Some(4),
        Some("0o") => Some(3),
        Some("0b") => Some(1),
        _ => None,
    };
    match (width, text.get(2..)) {
        (Some(width), Some(digits)) => binary_integer(digits, width),
        _ => decimal(text),
    }
    .unwrap_or(f64::NAN)
}

/// A StrDecimalLiteral's value, or `None` when the text is not one.
fn decimal(text: &str) -> Option<f64> {
    let unsigned = text.strip_prefix(['+', '-']).unwrap_or(text);
    if unsigned == "Infinity" {
        return Some(if text.starts_with('-') {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        });
    }
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, Some(exponent)),
        None => (unsigned, None),
    };
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let digits = |part: &str| part.bytes().all(|byte| byte.is_ascii_digit());
    let exponent_digits =
        exponent.map(|exponent| exponent.strip_prefix(['+', '-']).unwrap_or(exponent));
    let fits = (!whole.is_empty() || !fraction.is_empty())
        && digits(whole)
        && digits(fraction)
        && exponent_digits.is_none_or(|exponent| !exponent.is_empty() && digits(exponent));
    // Of JavaScript's shape, the text is one Rust reads, correctly rounded as JavaScript reads it.
    fits.then(|| text.parse().ok()).flatten()
}

/// The value of the digits of a base 2, 8 or 16 (`width` bits a digit), or `None` when there is
/// no digit or one is not of the base. Rounded to the nearest double, ties to even.
fn binary_integer(digits: &str, width: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }
    let mut bits: Vec<bool> = Vec::new();
    for character in digits.chars() {
        let digit = character.to_digit(1 << width)?;
        bits.extend((0..width).rev().map(|shift| (digit >> shift) & 1 == 1));
    }
    let first = bits.iter().position(|bit| *bit).unwrap_or(bits.len());
    let bits = bits.get(first..).unwrap_or_default();
    let value = |bits: &[bool]| {
        bits.iter()
            .fold(0_u64, |value, bit| (value << 1) | u64::from(*bit))
    };
    let Some((kept, dropped)) = bits.split_at_checked(53) else {
        return Some(value(bits) as f64);
    };
    let mut mantissa = value(kept);
    let half = dropped.first().copied().unwrap_or(false);
    let beyond_half = dropped.get(1..).unwrap_or_default().contains(&true);
    if half && (beyond_half || mantissa & 1 == 1) {
        mantissa += 1;
    }
    let scale = u64::try_from(dropped.len())
        .ok()
        .filter(|exponent| *exponent <= 1023)
        .map_or(f64::INFINITY, |exponent| {
            f64::from_bits((exponent + 1023) << 52)
        });
    Some(mantissa as f64 * scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_read_as_javascript_reads_them() {
        let cases = [
            ("", 0.0),
            (" \u{3000}12\u{FEFF}", 12.0),
            ("0x1F", 31.0),
            ("0B101", 5.0),
            ("-1.5e3", -1500.0),
            (".5", 0.5),
            ("5.", 5.0),
            ("-Infinity", f64::NEG_INFINITY),
            ("0x20000000000003", 9_007_199_254_740_996.0),
        ];
        for (text, value) in cases {
            assert_eq!(number(text).to_bits(), value.to_bits(), "{text:?}");
        }
        for text in [
            "1_000", "-0x1", "0x", ".", "1e", "infinity", "nan", "\u{85}1",
        ] {
            assert!(number(text).is_nan(), "{text:?}");
        }
        assert!(number("-0").is_sign_negative());
    }
}
