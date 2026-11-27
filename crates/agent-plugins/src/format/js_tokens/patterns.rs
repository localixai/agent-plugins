//! The sticky patterns of `src/format/js-tokens.ts` (`WORD`, `NUMBER`, `SPACE`, `HEX2`,
//! `UNICODE`) as matchers over UTF-16 units, and the character tests they share with the scanner.

use crate::js::identifier::{is_id_continue, is_id_start};
use crate::js::utf16::{is_blank_unit, slice};

pub(super) fn is(char: Option<u16>, ascii: char) -> bool {
    char.is_some_and(|unit| u32::from(unit) == u32::from(ascii))
}

fn is_digit(char: Option<u16>) -> bool {
    char.is_some_and(|unit| (0x30..=0x39).contains(&unit))
}

fn is_hex(char: Option<u16>) -> bool {
    char.and_then(|unit| char::from_u32(u32::from(unit)))
        .is_some_and(|character| character.is_ascii_hexdigit())
}

/// The code point at `at` (a surrogate pair is one) and its length in units.
fn code_point_at(text: &[u16], at: usize) -> Option<(char, usize)> {
    let units = slice(text, at, at + 2);
    let first = char::decode_utf16(units.iter().copied()).next()?.ok()?;
    Some((first, first.len_utf16()))
}

/// `WORD` at `at`, `/[\p{ID_Start}_$][\p{ID_Continue}$‌‍]*/uy`: its length, or `None`.
pub(super) fn word_at(text: &[u16], at: usize) -> Option<usize> {
    let (first, length) = code_point_at(text, at)?;
    if !(is_id_start(first) || first == '_' || first == '$') {
        return None;
    }
    let mut end = at + length;
    while let Some((next, length)) = code_point_at(text, end) {
        if !(is_id_continue(next) || matches!(next, '$' | '\u{200C}' | '\u{200D}')) {
            break;
        }
        end += length;
    }
    Some(end - at)
}

/// The end of a run of units from `at` that `wanted` accepts.
fn run_of(text: &[u16], at: usize, wanted: impl Fn(Option<u16>) -> bool) -> usize {
    let mut end = at;
    while end < text.len() && wanted(text.get(end).copied()) {
        end += 1;
    }
    end
}

/// One of `0x…`, `0b…`, `0o…`: the prefix's letter (either case), then digits and `_`, then `n`.
fn prefixed_at(
    text: &[u16],
    at: usize,
    letter: char,
    digit: impl Fn(Option<u16>) -> bool,
) -> Option<usize> {
    let prefix = text.get(at + 1).copied();
    if !is(text.get(at).copied(), '0')
        || !(is(prefix, letter) || is(prefix, letter.to_ascii_uppercase()))
    {
        return None;
    }
    let end = run_of(text, at + 2, |unit| digit(unit) || is(unit, '_'));
    (end > at + 2).then(|| end + usize::from(is(text.get(end).copied(), 'n')) - at)
}

/// `NUMBER` at `at`: `0[xX][\da-fA-F_]+n?|0[bB][01_]+n?|0[oO][0-7_]+n?|(?:\d[\d_]*\.?[\d_]*|\.\d[\d_]*)(?:[eE][+-]?\d+)?n?`
/// with `y`, its alternatives tried in order: its length, or `None`.
pub(super) fn number_at(text: &[u16], at: usize) -> Option<usize> {
    let unit = |at: usize| text.get(at).copied();
    let digit_or_separator = |unit: Option<u16>| is_digit(unit) || is(unit, '_');
    let binary = |unit: Option<u16>| is(unit, '0') || is(unit, '1');
    let octal = |unit: Option<u16>| unit.is_some_and(|unit| (0x30..=0x37).contains(&unit));
    if let Some(length) = prefixed_at(text, at, 'x', is_hex)
        .or_else(|| prefixed_at(text, at, 'b', binary))
        .or_else(|| prefixed_at(text, at, 'o', octal))
    {
        return Some(length);
    }
    let mut end = if is_digit(unit(at)) {
        let integer = run_of(text, at + 1, digit_or_separator);
        let point = integer + usize::from(is(unit(integer), '.'));
        run_of(text, point, digit_or_separator)
    } else if is(unit(at), '.') && is_digit(unit(at + 1)) {
        run_of(text, at + 2, digit_or_separator)
    } else {
        return None;
    };
    if is(unit(end), 'e') || is(unit(end), 'E') {
        let sign = end + 1 + usize::from(is(unit(end + 1), '+') || is(unit(end + 1), '-'));
        let digits = run_of(text, sign, is_digit);
        if digits > sign {
            end = digits;
        }
    }
    Some(end + usize::from(is(unit(end), 'n')) - at)
}

/// `SPACE` at `at`, `/\s+/y`: its length, `0` for none.
pub(super) fn space_at(text: &[u16], at: usize) -> usize {
    run_of(text, at, is_blank_unit) - at
}

/// `HEX2` at `at`, `/[\da-fA-F]{2}/y`: its length.
pub(super) fn hex2_at(text: &[u16], at: usize) -> Option<usize> {
    (is_hex(text.get(at).copied()) && is_hex(text.get(at + 1).copied())).then_some(2)
}

/// `UNICODE` at `at`, `/\{([\da-fA-F]+)\}|[\da-fA-F]{4}/y`: its length and its digits.
pub(super) fn unicode_at(text: &[u16], at: usize) -> Option<(usize, &[u16])> {
    if is(text.get(at).copied(), '{') {
        let end = run_of(text, at + 1, is_hex);
        if end > at + 1 && is(text.get(end).copied(), '}') {
            return Some((end + 1 - at, slice(text, at + 1, end)));
        }
    }
    (run_of(text, at, is_hex) >= at + 4).then(|| (4, slice(text, at, at + 4)))
}

/// `Number.parseInt(digits, 16)` of hexadecimal digits, as far as it can tell past U+10FFFF.
pub(super) fn hex_value(digits: &[u16]) -> u64 {
    digits.iter().fold(0_u64, |value, unit| {
        let digit = char::from_u32(u32::from(*unit))
            .and_then(|character| character.to_digit(16))
            .unwrap_or(0);
        value.saturating_mul(16).saturating_add(u64::from(digit))
    })
}
