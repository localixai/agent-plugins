//! The methods of `String.prototype` that take a `RegExp` and that the package's modules call:
//! `replace` with a replacement text (`$$`, `$&`, `` $` ``, `$'`, `$1`…`$99`, `$<name>`) or a
//! function, `split` and `matchAll`, each as ECMAScript's `RegExp.prototype[@@replace]`,
//! `[@@split]` and `[@@matchAll]` run them.

use std::ops::Range;

use super::{Found, RegExp, RegExpMatch, advance, text_of};

/// JavaScriptCore's `TypeError` for `matchAll` with a pattern that is not global.
pub const MATCH_ALL_NOT_GLOBAL: &str =
    "String.prototype.matchAll argument must not be a non-global regular expression";

/// The code units of `units` in `range`, empty for a range outside it.
fn units_in(units: &[u16], range: Range<usize>) -> &[u16] {
    units.get(range).unwrap_or_default()
}

/// The value of one or two ASCII digits.
fn digit(unit: u16) -> Option<usize> {
    (u16::from(b'0')..=u16::from(b'9'))
        .contains(&unit)
        .then(|| usize::from(unit - u16::from(b'0')))
}

/// ECMAScript's `GetSubstitution`: `template` with each `$` reference replaced from the match
/// `found` in `text`. A reference the match cannot fill (`$0`, `$7` with two groups, `$<` with
/// no named group) stays as written.
fn substitution(found: &Found, text: &[u16], template: &[u16]) -> Vec<u16> {
    let count = found.captures.len();
    let capture = |at: usize| -> &[u16] {
        found
            .captures
            .get(at)
            .cloned()
            .flatten()
            .map_or(&[][..], |range| units_in(text, range))
    };
    let dollar = u16::from(b'$');
    let mut out = Vec::with_capacity(template.len());
    let mut at = 0;
    while let Some(&unit) = template.get(at) {
        let next = template.get(at + 1).copied();
        if unit != dollar || next.is_none() {
            out.push(unit);
            at += 1;
            continue;
        }
        let next = next.unwrap_or_default();
        if next == dollar {
            out.push(dollar);
            at += 2;
        } else if next == u16::from(b'&') {
            out.extend_from_slice(units_in(text, found.range.clone()));
            at += 2;
        } else if next == u16::from(b'`') {
            out.extend_from_slice(units_in(text, 0..found.range.start));
            at += 2;
        } else if next == u16::from(b'\'') {
            out.extend_from_slice(units_in(text, found.range.end.min(text.len())..text.len()));
            at += 2;
        } else if let Some(first) = digit(next) {
            let two = template.get(at + 2).copied().and_then(digit);
            let (index, width) = match two {
                Some(second) if first * 10 + second <= count => (first * 10 + second, 2),
                _ => (first, 1),
            };
            if (1..=count).contains(&index) {
                out.extend_from_slice(capture(index - 1));
            } else {
                let written = template.get(at..at + 1 + width).unwrap_or_default();
                out.extend_from_slice(written);
            }
            at += 1 + width;
        } else if next == u16::from(b'<') {
            at += named_reference(found, text, template, at, &mut out);
        } else {
            out.push(dollar);
            at += 1;
        }
    }
    out
}

/// `$<name>` at `at`: the named group's text (empty when it did not take part or no group has
/// that name), or `$<` as written when there is no `>` or the pattern names no group. Answers
/// how many code units of the template it read.
fn named_reference(
    found: &Found,
    text: &[u16],
    template: &[u16],
    at: usize,
    out: &mut Vec<u16>,
) -> usize {
    let close = template
        .get(at + 2..)
        .and_then(|rest| rest.iter().position(|unit| *unit == u16::from(b'>')));
    let (Some(close), false) = (close, found.names.is_empty()) else {
        out.extend_from_slice(&[u16::from(b'$'), u16::from(b'<')]);
        return 2;
    };
    let name = String::from_utf16_lossy(units_in(template, at + 2..at + 2 + close));
    let range = found
        .names
        .iter()
        .find(|(each, _)| *each == name)
        .and_then(|(_, range)| range.clone());
    if let Some(range) = range {
        out.extend_from_slice(units_in(text, range));
    }
    close + 3
}

impl RegExp {
    /// `text.replace(regexp, replacement)`: the first match replaced, or every match when the
    /// pattern is global.
    pub fn replace(&mut self, text: &str, replacement: &str) -> String {
        let template: Vec<u16> = replacement.encode_utf16().collect();
        self.replace_each(text, |found, units| substitution(found, units, &template))
    }

    /// `text.replace(regexp, (match, ...groups) => …)`: each match replaced by what `replacement`
    /// answers for it, as `exec` answers it (`captures[0]` the match, then the groups).
    pub fn replace_with(
        &mut self,
        text: &str,
        mut replacement: impl FnMut(&RegExpMatch) -> String,
    ) -> String {
        self.replace_each(text, |found, units| {
            replacement(&found.to_match(units)).encode_utf16().collect()
        })
    }

    /// `RegExp.prototype[@@replace]`: the matches found first (all of them when global, from
    /// `lastIndex` 0), then each replaced by `replacement`'s units.
    fn replace_each(
        &mut self,
        text: &str,
        mut replacement: impl FnMut(&Found, &[u16]) -> Vec<u16>,
    ) -> String {
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut matches = Vec::new();
        if self.global() {
            self.last_index = 0;
            while let Some(found) = self.builtin_exec(&units) {
                if found.range.is_empty() {
                    self.last_index = advance(&units, self.last_index, self.unicode);
                }
                matches.push(found);
            }
        } else if let Some(found) = self.builtin_exec(&units) {
            matches.push(found);
        }
        let mut out = Vec::with_capacity(units.len());
        let mut next = 0;
        for found in &matches {
            let start = found.range.start.min(units.len());
            if start >= next {
                out.extend_from_slice(units_in(&units, next..start));
                out.extend(replacement(found, &units));
                next = found.range.end;
            }
        }
        out.extend_from_slice(units_in(&units, next.min(units.len())..units.len()));
        String::from_utf16_lossy(&out)
    }

    /// `text.split(regexp)`: the pieces between matches, each capture of a match between them.
    /// The leftmost match from a position is the sticky match the specification tries at each
    /// position before it, so the search goes straight to it.
    #[must_use]
    pub fn split(&self, text: &str) -> Vec<Option<String>> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let size = units.len();
        if size == 0 {
            return match self.find_at(&units, 0, true) {
                Some(_) => Vec::new(),
                None => vec![Some(String::new())],
            };
        }
        let mut pieces = Vec::new();
        let (mut piece_start, mut at) = (0, 0);
        while at < size {
            let Some(found) = self.find_at(&units, at, false) else {
                break;
            };
            if found.range.start >= size {
                break;
            }
            at = found.range.start;
            let end = found.range.end.min(size);
            if end == piece_start {
                at = advance(&units, at, self.unicode);
                continue;
            }
            pieces.push(Some(text_of(&units, piece_start..at)));
            pieces.extend(
                found
                    .captures
                    .iter()
                    .map(|range| range.clone().map(|range| text_of(&units, range))),
            );
            piece_start = end;
            at = end;
        }
        pieces.push(Some(text_of(&units, piece_start..size)));
        pieces
    }

    /// `[...text.matchAll(regexp)]`: every match from `lastIndex` on, which only a global
    /// pattern may be asked for (JavaScriptCore's `TypeError` otherwise).
    pub fn match_all(&self, text: &str) -> Result<Vec<RegExpMatch>, &'static str> {
        if !self.global() {
            return Err(MATCH_ALL_NOT_GLOBAL);
        }
        let units: Vec<u16> = text.encode_utf16().collect();
        let mut matcher = self.clone();
        let mut matches = Vec::new();
        while let Some(found) = matcher.builtin_exec(&units) {
            if found.range.is_empty() {
                matcher.last_index = advance(&units, matcher.last_index, self.unicode);
            }
            matches.push(found.to_match(&units));
        }
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::super::RegExp;

    #[test]
    fn replaces_with_references() {
        let mut regexp = RegExp::new("(?<word>b)(c)?", "g").unwrap();
        let replaced = regexp.replace("abcab", "[$1|$2|$<word>|$&|$`|$'|$$|$0|$3|$<x>|$<]");
        assert_eq!(
            replaced,
            "a[b|c|b|bc|a|ab|$|$0|$3||$<]a[b||b|b|abca||$|$0|$3||$<]"
        );
    }

    #[test]
    fn replaces_with_what_a_function_answers() {
        let mut first = RegExp::new("(a)(x)?", "").unwrap();
        let replaced = first.replace_with("babab", |found| {
            format!("<{}{:?}>", found.index, found.captures)
        });
        assert_eq!(replaced, r#"b<1[Some("a"), Some("a"), None]>bab"#);
        let mut every = RegExp::new("a*", "g").unwrap();
        assert_eq!(
            every.replace_with("baa", |found| format!("[{}]", found.index)),
            "[0]b[1][3]"
        );
        assert_eq!(every.last_index(), 0);
    }

    #[test]
    fn splits_around_matches_and_their_captures() {
        let regexp = RegExp::new(r"\s+", "").unwrap();
        let pieces: Vec<_> = regexp.split(" a  b ").into_iter().flatten().collect();
        assert_eq!(pieces, ["", "a", "b", ""]);
        let regexp = RegExp::new("(-)|x", "").unwrap();
        assert_eq!(
            regexp.split("a-bxc"),
            [
                Some("a".into()),
                Some("-".into()),
                Some("b".into()),
                None,
                Some("c".into())
            ]
        );
        assert_eq!(RegExp::new("", "").unwrap().split("ab").len(), 2);
    }

    #[test]
    fn matches_all_of_a_global_pattern_only() {
        let regexp = RegExp::new("a*", "g").unwrap();
        let indexes: Vec<_> = regexp
            .match_all("baa")
            .unwrap()
            .iter()
            .map(|m| m.index)
            .collect();
        assert_eq!(indexes, [0, 1, 3]);
        assert!(RegExp::new("a", "").unwrap().match_all("a").is_err());
    }
}
