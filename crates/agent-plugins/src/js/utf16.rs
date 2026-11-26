//! A JavaScript string as its UTF-16 code units, for a port that walks a text by index as the
//! TypeScript does: `text[i]` is one code unit, `indexOf`, `startsWith(x, i)` and `slice` count in
//! them, and a cut between the two halves of a character above U+FFFF keeps each half where it
//! was. A text made back from units reads a lone half as U+FFFD (docs/RUST.md, "Where the crate
//! answers differently").

use super::is_whitespace;

/// The code units of `text`.
#[must_use]
pub fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

/// The text of `units`, a lone surrogate read as U+FFFD.
#[must_use]
pub fn text_of(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// `text[at]`: the code unit at `at`, or `None` past the end.
#[must_use]
pub fn unit_at(units: &[u16], at: usize) -> Option<u16> {
    units.get(at).copied()
}

/// `text.slice(start, end)` with both indexes from 0 up: the units between them, kept within the
/// text; none when `start` is past `end`.
#[must_use]
pub fn slice(units: &[u16], start: usize, end: usize) -> &[u16] {
    let end = end.min(units.len());
    units.get(start.min(end)..end).unwrap_or_default()
}

/// `text.startsWith(prefix, at)`.
#[must_use]
pub fn starts_with_at(units: &[u16], at: usize, prefix: &str) -> bool {
    (at..)
        .zip(prefix.encode_utf16())
        .all(|(at, unit)| unit_at(units, at) == Some(unit))
}

/// `text.indexOf(needle, from)`: where `needle` next starts at or after `from`, or `None` (`-1`).
#[must_use]
pub fn index_of(units: &[u16], needle: &str, from: usize) -> Option<usize> {
    let wanted = needle.encode_utf16().count();
    let last = units.len().checked_sub(wanted)?;
    (from.min(units.len())..=last).find(|&at| starts_with_at(units, at, needle))
}

/// Whether `unit` is the one code unit of `text` and `text` has no other: `char === text` for a
/// `char` read as `text[i]`.
#[must_use]
pub fn is_unit(unit: Option<u16>, text: &str) -> bool {
    let mut spelled = text.encode_utf16();
    unit.is_some() && spelled.next() == unit && spelled.next().is_none()
}

/// `/\s/.test(char)` for a `char` read as `text[i]`: JavaScript's whitespace, none past the end
/// or for a half of a character above U+FFFF.
#[must_use]
pub fn is_blank_unit(unit: Option<u16>) -> bool {
    unit.and_then(|unit| char::from_u32(u32::from(unit)))
        .is_some_and(is_whitespace)
}

/// `text.replaceAll(from, to)` of a text as units.
#[must_use]
pub fn replace_all(units: &[u16], from: &str, to: &str) -> Vec<u16> {
    let (from_units, to_units): (Vec<u16>, Vec<u16>) =
        (from.encode_utf16().collect(), to.encode_utf16().collect());
    let mut out = Vec::with_capacity(units.len());
    let mut at = 0;
    while at < units.len() {
        if !from_units.is_empty() && units.get(at..at + from_units.len()) == Some(&from_units) {
            out.extend_from_slice(&to_units);
            at += from_units.len();
        } else {
            out.extend(unit_at(units, at));
            at += 1;
        }
    }
    out
}

/// `String.fromCodePoint(code)` as units, for a `code` up to U+10FFFF: one unit up to U+FFFF (a
/// surrogate alone too), else the two halves.
#[must_use]
pub fn from_code_point(code: u32) -> Vec<u16> {
    match char::from_u32(code) {
        Some(character) => character.encode_utf16(&mut [0; 2]).to_vec(),
        None => u16::try_from(code)
            .map(|unit| vec![unit])
            .unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_a_text_as_javascript_indexes_it() {
        let text = units("a😀b\u{a0}");
        assert_eq!(text.len(), 5);
        assert_eq!(index_of(&text, "b", 0), Some(3));
        assert_eq!(index_of(&text, "b", 4), None);
        assert_eq!(index_of(&text, "", 9), Some(5));
        assert!(starts_with_at(&text, 3, "b\u{a0}"));
        assert!(!starts_with_at(&text, 4, "\u{a0}x"));
        assert_eq!(text_of(slice(&text, 1, 3)), "😀");
        assert_eq!(text_of(slice(&text, 1, 2)), "\u{fffd}");
        assert_eq!(slice(&text, 4, 2), &[] as &[u16]);
        assert!(is_blank_unit(unit_at(&text, 4)));
        assert!(!is_blank_unit(unit_at(&text, 1)));
        assert!(!is_blank_unit(None));
        assert!(is_unit(unit_at(&text, 0), "a"));
        assert!(!is_unit(unit_at(&text, 0), "ab"));
        assert!(!is_unit(None, ""));
        assert_eq!(from_code_point(0x1f600), units("😀"));
        assert_eq!(from_code_point(0xd800), [0xd800]);
        assert_eq!(from_code_point(0x41), [0x41]);
        assert_eq!(
            text_of(&replace_all(&units("a\\`b\\`"), "\\`", "`")),
            "a`b`"
        );
    }
}
