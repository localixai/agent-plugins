//! Port of `src/format/env-number.ts`, switched.
//!
//! A numeric environment variable (spec §3.6.1 item 2, J3.17): plain digits, scientific notation
//! (`2e3` is 2000) and digit separators (`64_000` is 64000), unless the variable's row says plain
//! digits only. A sign, a blank or any other character makes the value unreadable (judgment). A
//! value that cannot be read is `None`, so the caller keeps its default.

use crate::js::{is_safe_integer, number};

/// The length of the run of `\d+(?:_\d+)*` at the start of `text`: digits, with single `_`
/// between digit runs. `0` when `text` does not start with a digit.
fn digits_length(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut end = 0;
    loop {
        let run = bytes
            .get(end..)
            .unwrap_or_default()
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if run == 0 {
            return if end == 0 { 0 } else { end - 1 };
        }
        end += run;
        if bytes.get(end) == Some(&b'_') {
            end += 1;
        } else {
            return end;
        }
    }
}

/// `^DIGITS(?:\.DIGITS)?(?:[eE][+-]?\d+)?$` with `DIGITS` as `\d+(?:_\d+)*`.
fn is_numeric(text: &str) -> bool {
    let mut rest = text;
    let whole = digits_length(rest);
    if whole == 0 {
        return false;
    }
    rest = rest.get(whole..).unwrap_or_default();
    if let Some(after_point) = rest.strip_prefix('.') {
        let fraction = digits_length(after_point);
        if fraction == 0 {
            return false;
        }
        rest = after_point.get(fraction..).unwrap_or_default();
    }
    if let Some(after_e) = rest.strip_prefix(['e', 'E']) {
        let digits = after_e.strip_prefix(['+', '-']).unwrap_or(after_e);
        return !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit());
    }
    rest.is_empty()
}

/// `^\d+$`.
fn is_plain(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}

/// The number a numeric variable holds, or `None` when it holds none.
#[must_use]
pub fn read_number_env(text: Option<&str>) -> Option<f64> {
    let text = text.filter(|text| is_numeric(text))?;
    let value = number(&text.replace('_', ""));
    value.is_finite().then_some(value)
}

/// A whole number, `0` included, read as `read_number_env` reads it.
#[must_use]
pub fn whole_env(text: Option<&str>) -> Option<f64> {
    read_number_env(text).filter(|value| is_safe_integer(*value))
}

/// A whole number above zero, read as `read_number_env` reads it.
#[must_use]
pub fn positive_whole_env(text: Option<&str>) -> Option<f64> {
    whole_env(text).filter(|value| *value > 0.0)
}

/// A whole number above zero in plain digits only, for a variable whose row says so.
#[must_use]
pub fn plain_positive_whole_env(text: Option<&str>) -> Option<f64> {
    let text = text.filter(|text| is_plain(text))?;
    let value = number(text);
    (is_safe_integer(value) && value > 0.0).then_some(value)
}

crate::registry::exports! {
    "format/env-number" {
        "readNumberEnv" => ok |text: Option<String>| read_number_env(text.as_deref());
        "wholeEnv" => ok |text: Option<String>| whole_env(text.as_deref());
        "positiveWholeEnv" => ok |text: Option<String>| positive_whole_env(text.as_deref());
        "plainPositiveWholeEnv" => ok |text: Option<String>| plain_positive_whole_env(text.as_deref());
    }
}
