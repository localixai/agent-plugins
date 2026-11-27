//! What the engine is given for a pattern where it would read the pattern's text otherwise than
//! JavaScript does. Without `u` or `v`:
//!
//! - an escape is given as `\uHHHH`, and an escaped surrogate as its unit: the engine would join
//!   a `😀` escape pair into one character, and it reads a sign before an escape's digits
//!   (`\x+1` is `x+1` in JavaScript);
//! - a `{` that starts no braced quantifier is given as `\{` (the engine reads `a{+1}` as one);
//! - case-insensitive parts (the `i` flag, a `(?i:…)` group) fold by `Canonicalize`
//!   (`legacy_case.rs`): each cased character becomes the class of the units it matches and each
//!   class its closure, and the pattern is compiled without `i`. A backreference that `i` applies
//!   to compares by `Canonicalize`, which no class spells: the text is then matched with each
//!   unit canonicalized (the captures are still the text's own); a pattern that also turns `i`
//!   on or off for a group keeps the engine's folding (docs/RUST.md), its escapes still spelled.
//!
//! With `u`, a `\W` inside a class that `i` applies to is spelled as its ranges: the engine would
//! fold the class after `\W`, which holds `ſ` and the Kelvin sign, so it would also meet `s` and
//! `k`.
//!
//! The engine has compiled the pattern as written first, so its text is valid here.

mod atoms;
mod scan;

use scan::{Folding, Scanner};

/// What the engine compiles: the pattern's units, and whether it folds case itself.
pub(super) struct EngineSource {
    pub(super) units: Vec<u32>,
    pub(super) icase: bool,
    /// Whether the text is matched canonicalized (`legacy_case::canonical`), as a backreference
    /// that `i` applies to compares what it repeats by `Canonicalize`.
    pub(super) canonical_text: bool,
}

/// The pattern's units as the engine first compiles it: code points with `u` or `v`, else UTF-16
/// units. With `u` or `v`, where the engine refuses what JavaScriptCore reads:
///
/// - an escaped character past ASCII is given as the character: JavaScriptCore reads `\é` as
///   `é`, where the spec refuses it (docs/RUST.md);
/// - a lead surrogate's `\uHHHH` before a `\u{…}` is given braced, as the engine would read the
///   braced escape as that surrogate's trail.
pub(super) fn written(source: &str, unicode: bool) -> Vec<u32> {
    if !unicode {
        return source.encode_utf16().map(u32::from).collect();
    }
    let units: Vec<u32> = source.chars().map(u32::from).collect();
    let mut out = Vec::with_capacity(units.len());
    let mut at = 0;
    while let Some(&unit) = units.get(at) {
        let escaped = units.get(at + 1).copied();
        if !is(Some(&unit), '\\') {
            out.push(unit);
            at += 1;
            continue;
        }
        if escaped.is_some_and(|escaped| escaped > 0x7F) {
            out.extend(escaped);
        } else if let Some(lead) = lead_before_braced(&units, at) {
            out.extend(format!("\\u{{{lead:X}}}").chars().map(u32::from));
            at += 4;
        } else {
            out.push(unit);
            out.extend(escaped);
        }
        at += 2;
    }
    out
}

/// The lead surrogate a `\uHHHH` at `at` stands for, when a `\u{` follows it.
fn lead_before_braced(units: &[u32], at: usize) -> Option<u32> {
    if !is(units.get(at + 1), 'u') || !is(units.get(at + 6), '\\') || !is(units.get(at + 7), 'u') {
        return None;
    }
    if !is(units.get(at + 8), '{') {
        return None;
    }
    let digits: String = units
        .get(at + 2..at + 6)?
        .iter()
        .map(|unit| char::from_u32(*unit))
        .collect::<Option<_>>()?;
    let value = u32::from_str_radix(&digits, 16).ok()?;
    (digits.chars().all(|digit| digit.is_ascii_hexdigit()) && (0xD800..=0xDBFF).contains(&value))
        .then_some(value)
}

/// The engine's spelling of the pattern `written`, `None` when it goes to the engine as written.
pub(super) fn engine_source(written: &[u32], flags: &str) -> Option<EngineSource> {
    if flags.contains('v') {
        return None;
    }
    let icase = flags.contains('i');
    if flags.contains('u') {
        return Scanner::new(written, icase, Folding::Pass)
            .unicode_word_classes()
            .map(|units| EngineSource {
                units,
                icase,
                canonical_text: false,
            });
    }
    let holds = |character: char| written.contains(&u32::from(character));
    if !(icase || holds('(') || holds('\\') || holds('{')) {
        return None;
    }
    let folded = Scanner::new(written, icase, Folding::Pass).legacy();
    let Some((units, canonical_text)) = folded else {
        let (units, _) = Scanner::new(written, icase, Folding::Engine).legacy()?;
        return Some(EngineSource {
            units,
            icase,
            canonical_text: false,
        });
    };
    Some(EngineSource {
        units,
        icase: false,
        canonical_text,
    })
}

/// The ranges `\W` stands for inside a class that `i` applies to, with `u`: the complement of
/// `[0-9A-Z_a-z]` and the two characters that fold into it, `ſ` and the Kelvin sign.
const UNICODE_NOT_WORD: &str = r"\x00-\x2F\x3A-\x40\x5B-\x5E\x60\x7B-žƀ-℩Å-\u{10FFFF}";

/// Whether a modifier group `(?ims-ims:` starts at `at` of `units` (after `(?`), and the state of
/// `i` inside it given the state outside; with the group's header length and its spelling
/// without `i`.
fn modifiers(units: &[u32], at: usize, outside: bool) -> Option<(bool, usize, Vec<u32>)> {
    let mut length = 0;
    let (mut add, mut remove) = (Vec::new(), Vec::new());
    let mut removing = false;
    loop {
        let unit = *units.get(at + length)?;
        length += 1;
        match char::from_u32(unit)? {
            ':' => break,
            '-' if !removing => removing = true,
            flag @ ('i' | 'm' | 's') if removing => remove.push(flag),
            flag @ ('i' | 'm' | 's') => add.push(flag),
            _ => return None,
        }
    }
    let inside = if add.contains(&'i') {
        true
    } else if remove.contains(&'i') {
        false
    } else {
        outside
    };
    let mut spelled: String = add.iter().filter(|flag| **flag != 'i').collect();
    let removed: String = remove.iter().filter(|flag| **flag != 'i').collect();
    if !removed.is_empty() {
        spelled.push('-');
        spelled.push_str(&removed);
    }
    spelled.push(':');
    Some((inside, length, spelled.chars().map(u32::from).collect()))
}

/// Whether `unit` is the ASCII character `character`.
fn is(unit: Option<&u32>, character: char) -> bool {
    unit.is_some_and(|unit| *unit == u32::from(character))
}

/// A unit as an escape a class may hold: `\uHHHH`, or the unit itself for a surrogate, which the
/// engine reads as one unit only when it is not escaped.
fn class_unit(unit: u32, out: &mut Vec<u32>) {
    if (0xD800..=0xDFFF).contains(&unit) {
        out.push(unit);
    } else {
        out.extend(format!("\\u{unit:04X}").chars().map(u32::from));
    }
}

/// Whether a name of a `(?<name>` group starts at `at` (after `(?<`): not a lookbehind.
fn is_group_name(units: &[u32], at: usize) -> bool {
    !(is(units.get(at), '=') || is(units.get(at), '!'))
}

#[cfg(test)]
mod tests {
    use super::atoms::is_letter;
    use super::{engine_source, written};

    fn spelled(source: &str, flags: &str) -> Option<String> {
        let unicode = flags.contains('u') || flags.contains('v');
        engine_source(&written(source, unicode), flags).map(|engine| {
            engine
                .units
                .iter()
                .map(|unit| char::from_u32(*unit).unwrap_or('?'))
                .collect()
        })
    }

    #[test]
    fn spells_cased_units_as_classes_without_u() {
        assert_eq!(spelled("s", "i").unwrap(), "[\\u0053\\u0073]");
        assert_eq!(spelled("ſ\\.", "i").unwrap(), "ſ\\u002E");
        assert_eq!(spelled("(?i:a)b", "").unwrap(), "(?:[\\u0041\\u0061])b");
        assert_eq!(spelled("(?<Ab>x)\\k<Ab>", "").unwrap(), r"(?<Ab>x)\k<Ab>");
        assert!(
            engine_source(&written("(a)\\1", false), "i")
                .is_some_and(|engine| engine.canonical_text)
        );
        assert_eq!(spelled("(a)\\1(?-i:b)", "i").unwrap(), "(a)\\1(?-i:b)");
        assert_eq!(spelled("\\x+1{+1}", "").unwrap(), "\\u0078+1\\{+1}");
        assert!(spelled("abc", "").is_none());
        assert!(is_letter(u32::from('Q')));
    }

    #[test]
    fn spells_word_classes_by_their_ranges_with_u() {
        let word = spelled("[\\W]", "iu").unwrap();
        assert!(word.starts_with(r"[\x00-\x2F"));
        assert!(spelled("[\\W]", "u").is_none());
        let units = |text: &str| text.chars().map(u32::from).collect::<Vec<_>>();
        assert_eq!(written("\\é\\\\", true), units("é\\\\"));
        assert_eq!(written("\\uD83D\\u{E9}", true), units("\\u{D83D}\\u{E9}"));
    }
}
