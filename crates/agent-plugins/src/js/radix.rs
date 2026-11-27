//! A number's text in exponent form and in a power-of-two radix, as JavaScript writes them
//! (`toExponential()`, `toString(8)`, `toString(16)`): the YAML writer's `1e+3`, `0o17` and `0x1f`.

use super::shortest_digits;

/// `Number.prototype.toExponential()` with no digits given: the shortest digits that read back as
/// the number, one before the point, then `e`, the exponent's sign and the exponent (`1.5e+3`).
/// For a finite number.
#[must_use]
pub fn to_exponential(value: f64) -> String {
    if value == 0.0 {
        return "0e+0".to_owned();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let (digits, n) = shortest_digits(value.abs());
    let exponent = n - 1;
    let (first, rest) = digits.split_at(1.min(digits.len()));
    let point = if rest.is_empty() {
        String::new()
    } else {
        format!(".{rest}")
    };
    let exponent_sign = if exponent < 0 { '-' } else { '+' };
    format!("{sign}{first}{point}e{exponent_sign}{}", exponent.abs())
}

/// `Number.prototype.toString(radix)` of a non-negative integer for radix 8 or 16: its exact
/// digits, lowercase (the bits of the double grouped by three or four).
#[must_use]
pub fn to_radix_string(value: f64, radix: u32) -> String {
    let bits_per_digit = if radix == 8 { 3 } else { 4 };
    let bits = integer_bits(value);
    let padding = (bits_per_digit - bits.len() % bits_per_digit) % bits_per_digit;
    let padded: Vec<u32> = std::iter::repeat_n(0, padding).chain(bits).collect();
    let digits: String = padded
        .chunks(bits_per_digit)
        .map(|group| group.iter().fold(0, |sum, bit| sum * 2 + bit))
        .filter_map(|digit| char::from_digit(digit, radix))
        .collect();
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// The binary digits of a non-negative integral double, most significant first.
fn integer_bits(value: f64) -> Vec<u32> {
    if !(value.is_finite() && value >= 1.0) {
        return vec![0];
    }
    let raw = value.to_bits();
    let exponent = i64::try_from((raw >> 52) & 0x7FF).unwrap_or(0) - 1075;
    let mantissa = (raw & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let mut bits: Vec<u32> = (0..53)
        .rev()
        .map(|shift| u32::from((mantissa >> shift) & 1 == 1))
        .collect();
    match usize::try_from(exponent) {
        Ok(zeros) => bits.extend(std::iter::repeat_n(0, zeros)),
        Err(_) => bits.truncate(
            bits.len()
                .saturating_sub(usize::try_from(-exponent).unwrap_or(0)),
        ),
    }
    bits
}

#[cfg(test)]
mod tests {
    use super::{to_exponential, to_radix_string};

    /// What Bun answers: `bun -e "for (const v of [0, 1, 1000, 1500, 0.000123, 1e21,
    /// 123456789012]) console.log(v.toExponential()); for (const v of [0, 8, 255,
    /// 9007199254740993, 1e21, 2**60+2**10]) console.log(v.toString(8), v.toString(16))"`.
    #[test]
    fn writes_as_javascript_does() {
        let exponential = [
            (0.0, "0e+0"),
            (1.0, "1e+0"),
            (1000.0, "1e+3"),
            (1500.0, "1.5e+3"),
            (0.000_123, "1.23e-4"),
            (1e21, "1e+21"),
            (123_456_789_012.0, "1.23456789012e+11"),
        ];
        for (value, text) in exponential {
            assert_eq!(to_exponential(value), text);
        }
        let radix = [
            (0.0, "0", "0"),
            (8.0, "10", "8"),
            (255.0, "377", "ff"),
            (
                9_007_199_254_740_992.0,
                "400000000000000000",
                "20000000000000",
            ),
            (1e21, "154327115334273650000000", "3635c9adc5dea00000"),
            (
                1_152_921_504_606_848_000.0,
                "100000000000000002000",
                "1000000000000400",
            ),
        ];
        for (value, octal, hex) in radix {
            assert_eq!(to_radix_string(value, 8), octal);
            assert_eq!(to_radix_string(value, 16), hex);
        }
    }
}
