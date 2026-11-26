//! What JavaScriptCore refuses in a pattern the engine compiles: a quantifier on the assertion
//! `\b` or `\B` (ECMA-262 makes neither a `QuantifiableAssertion`), and with `u` or `v` a `\u`
//! that is neither four hexadecimal digits nor a braced code point (the engine reads a sign before
//! the digits, `\u{+41}`). JavaScriptCore names the first one it meets.

/// Whether `unit` is the ASCII character `character`.
fn is(unit: Option<&u32>, character: char) -> bool {
    unit.is_some_and(|unit| *unit == u32::from(character))
}

fn is_digit(unit: Option<&u32>, radix: u32) -> bool {
    unit.and_then(|unit| char::from_u32(*unit))
        .is_some_and(|character| character.is_digit(radix))
}

/// The length of the braced quantifier at `at` (`{n}`, `{n,}`, `{n,m}`, digits only), if one
/// starts there.
pub(super) fn braced_length(units: &[u32], at: usize) -> Option<usize> {
    if !is(units.get(at), '{') {
        return None;
    }
    let digits = |from: usize| {
        (from..)
            .take_while(|at| is_digit(units.get(*at), 10))
            .count()
    };
    let low = digits(at + 1);
    let mut end = at + 1 + low;
    if low == 0 {
        return None;
    }
    if is(units.get(end), ',') {
        end += 1 + digits(end + 1);
    }
    is(units.get(end), '}').then_some(end + 1 - at)
}

/// Whether a quantifier starts at `at`.
fn quantifier_at(units: &[u32], at: usize) -> bool {
    ['*', '+', '?'].iter().any(|each| is(units.get(at), *each))
        || braced_length(units, at).is_some()
}

/// The reason JavaScriptCore gives for the `\u` escape at `at` (its backslash) with `u`, if it
/// refuses it.
pub(super) fn unicode_escape(units: &[u32], at: usize) -> Option<&'static str> {
    if is(units.get(at + 2), '{') {
        let digits: String = (at + 3..)
            .map_while(|at| units.get(at).and_then(|unit| char::from_u32(*unit)))
            .take_while(char::is_ascii_hexdigit)
            .collect();
        let closed = is(units.get(at + 3 + digits.len()), '}');
        let in_range = u32::from_str_radix(&digits, 16).is_ok_and(|value| value <= 0x0010_FFFF);
        return (!in_range || !closed).then_some("invalid Unicode code point \\u{} escape");
    }
    let four = (at + 2..at + 6).all(|at| is_digit(units.get(at), 16));
    (!four).then_some("invalid Unicode \\u escape")
}

/// The reason JavaScriptCore refuses a pattern the engine compiled (`units` as it was given to
/// the engine), if it does: `unicode` for `u` or `v`, `sets` for `v`, whose classes nest.
pub(super) fn refusal(units: &[u32], unicode: bool, sets: bool) -> Option<&'static str> {
    let (mut at, mut depth) = (0, 0usize);
    while let Some(&unit) = units.get(at) {
        match char::from_u32(unit).unwrap_or_default() {
            '\\' => {
                let letter = units.get(at + 1).and_then(|unit| char::from_u32(*unit));
                let reason = match letter {
                    Some('b' | 'B') if depth == 0 && quantifier_at(units, at + 2) => {
                        Some("nothing to repeat")
                    }
                    Some('u') if unicode => unicode_escape(units, at),
                    _ => None,
                };
                if reason.is_some() {
                    return reason;
                }
                at += 1;
            }
            '[' => depth = if sets { depth + 1 } else { 1 },
            ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
        at += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{braced_length, refusal};

    fn units(source: &str) -> Vec<u32> {
        source.chars().map(u32::from).collect()
    }

    #[test]
    fn refuses_what_the_engine_would_run() {
        assert_eq!(
            refusal(&units("\\b*"), false, false),
            Some("nothing to repeat")
        );
        assert_eq!(refusal(&units("[\\b]*"), false, false), None);
        assert_eq!(refusal(&units("\\u+00e"), false, false), None);
        assert_eq!(
            refusal(&units("\\u+00e"), true, false),
            Some("invalid Unicode \\u escape")
        );
        assert!(refusal(&units("\\u{+41}"), true, false).is_some());
        assert!(refusal(&units("\\u{110000}"), true, false).is_some());
        assert_eq!(refusal(&units("\\u{41}\\u0041"), true, false), None);
        assert_eq!(braced_length(&units("{1,}"), 0), Some(4));
        assert_eq!(braced_length(&units("{+1}"), 0), None);
    }
}
