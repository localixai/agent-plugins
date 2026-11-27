//! JavaScript's `RegExp` for a pattern that comes from input (a hook's `matcher`, a rule's glob, a
//! relevance signal's `pattern`, a schema's `patternProperties`): ECMAScript's syntax and
//! semantics (lookaround, backreferences, named groups, `\p{…}`, the `dgimsuvy` flags), where the
//! `regex` crate the core's own fixed patterns use has neither lookaround nor backreferences.
//!
//! The engine is the `regress` crate, a backtracking matcher of ECMAScript's grammar. Text is
//! matched as JavaScript holds it, in UTF-16 code units: with `u` or `v` a surrogate pair is one
//! character, without them every unit is one (`/^.$/` does not match `😀`), and the pattern is
//! read the same way. So every index here (`index`, `last_index`) counts code units, as
//! `RegExpExecArray.index` and `lastIndex` do. A capture that splits a pair (only possible without
//! `u`) reads as U+FFFD, as every lone surrogate does in the core (docs/RUST.md).
//!
//! A pattern that backtracks catastrophically (`/(a+)+$/` on `aaaa…b`) runs as long as it takes,
//! as in JavaScriptCore, which sets no limit either: neither the spec nor the package states one.
//!
//! `scripts/rust/js-oracle/regexp.ts` records Bun's answers, which `tests/differential/js.rs`
//! holds this to.

mod escape_pattern;
mod legacy_case;
pub mod methods;
mod refusal;
mod source;
mod syntax_error;

use std::ops::Range;

use indexmap::IndexMap;
use regress::{Flags, Regex};

pub use syntax_error::SyntaxError;

/// The flags JavaScript knows, in the order its `flags` getter spells them.
const FLAG_ORDER: &str = "dgimsuvy";

/// The longest pattern JavaScriptCore compiles, in UTF-16 code units, whatever its flags and
/// before its syntax is read: one unit more is "regular expression too large" (measured in Bun,
/// and recorded by `scripts/rust/js-oracle/ignore.ts`, whose glob patterns reach it).
const MAX_PATTERN_LENGTH: usize = 1024 * 1024;

/// A compiled regular expression with the state JavaScript's keeps: `lastIndex`, which `exec`
/// and `test` read and write when the pattern is global (`g`) or sticky (`y`).
#[derive(Debug, Clone)]
pub struct RegExp {
    source: String,
    flags: String,
    regex: Regex,
    unicode: bool,
    canonical_text: bool,
    last_index: usize,
}

/// What `exec` answers for a match: `index`, the captures (the whole match first, `None` for a
/// group that did not take part) and `groups`, the named ones in the pattern's order (`None`
/// when the pattern names none, as JavaScript's `groups` is then `undefined`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegExpMatch {
    /// Where the match starts, in UTF-16 code units.
    pub index: usize,
    /// The match and each capturing group's text.
    pub captures: Vec<Option<String>>,
    /// The named groups and their text.
    pub groups: Option<IndexMap<String, Option<String>>>,
}

/// One match as code-unit ranges, what `replace` and `split` build from.
#[derive(Debug, Clone)]
struct Found {
    range: Range<usize>,
    captures: Vec<Option<Range<usize>>>,
    names: Vec<(String, Option<Range<usize>>)>,
}

/// The text of a code-unit range of `units`.
fn text_of(units: &[u16], range: Range<usize>) -> String {
    String::from_utf16_lossy(units.get(range).unwrap_or_default())
}

/// The flags as JavaScript accepts them: each of `dgimsuvy` at most once, and not `u` with `v`.
fn valid_flags(flags: &str) -> bool {
    let mut seen = String::new();
    for flag in flags.chars() {
        if !FLAG_ORDER.contains(flag) || seen.contains(flag) {
            return false;
        }
        seen.push(flag);
    }
    !(seen.contains('u') && seen.contains('v'))
}

impl RegExp {
    /// `new RegExp(source, flags)`: a `SyntaxError` with JavaScriptCore's message for flags it
    /// refuses or a pattern that does not compile.
    pub fn new(source: &str, flags: &str) -> Result<Self, SyntaxError> {
        if !valid_flags(flags) {
            return Err(SyntaxError::flags());
        }
        if super::code_unit_length(source) > MAX_PATTERN_LENGTH {
            return Err(SyntaxError::refused("regular expression too large"));
        }
        let has = |flag: char| flags.contains(flag);
        let unicode = has('u') || has('v');
        let engine_flags = Flags {
            icase: has('i'),
            multiline: has('m'),
            dot_all: has('s'),
            no_opt: false,
            unicode: has('u'),
            unicode_sets: has('v'),
        };
        let written = source::written(source, unicode);
        let mut regex = Regex::from_unicode(written.iter().copied(), engine_flags)
            .map_err(|error| SyntaxError::pattern(source, &error, unicode))?;
        if let Some(reason) = refusal::refusal(&written, unicode, has('v')) {
            return Err(SyntaxError::refused(reason));
        }
        let mut canonical_text = false;
        // The pattern compiled as written, so its respelling does too; were it not to, the
        // pattern as written keeps its answers.
        if let Some(engine) = source::engine_source(&written, flags) {
            let respelled_flags = Flags {
                icase: engine.icase,
                ..engine_flags
            };
            if let Ok(respelled) =
                Regex::from_unicode(engine.units.iter().copied(), respelled_flags)
            {
                regex = respelled;
                canonical_text = engine.canonical_text;
            }
        }
        Ok(Self {
            source: escape_pattern::escape_pattern(source),
            flags: FLAG_ORDER.chars().filter(|flag| has(*flag)).collect(),
            regex,
            unicode,
            canonical_text,
            last_index: 0,
        })
    }

    /// The pattern as `RegExp.prototype.source` prints it (`escape_pattern.rs`).
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The flags in JavaScript's order (`RegExp.prototype.flags`).
    #[must_use]
    pub fn flags(&self) -> &str {
        &self.flags
    }

    /// Whether the pattern is global (`g`).
    fn global(&self) -> bool {
        self.flags.contains('g')
    }

    /// Whether the pattern is sticky (`y`).
    fn sticky(&self) -> bool {
        self.flags.contains('y')
    }

    /// `lastIndex`, in UTF-16 code units.
    #[must_use]
    pub const fn last_index(&self) -> usize {
        self.last_index
    }

    /// Sets `lastIndex`.
    pub const fn set_last_index(&mut self, index: usize) {
        self.last_index = index;
    }

    /// `regexp.test(text)`.
    pub fn test(&mut self, text: &str) -> bool {
        self.exec(text).is_some()
    }

    /// `regexp.test(text)` of a pattern several readers share: the same answer, with `lastIndex`
    /// written only to a copy when the pattern is global or sticky, so the pattern stays as it was.
    #[must_use]
    pub fn test_shared(&self, text: &str) -> bool {
        if self.global() || self.sticky() {
            return self.clone().test(text);
        }
        let units: Vec<u16> = text.encode_utf16().collect();
        self.find_at(&units, 0, false).is_some()
    }

    /// `regexp.exec(text)`.
    pub fn exec(&mut self, text: &str) -> Option<RegExpMatch> {
        let units: Vec<u16> = text.encode_utf16().collect();
        let found = self.builtin_exec(&units)?;
        Some(found.to_match(&units))
    }

    /// The first match starting at `start` or after it (only at `start` when `sticky`).
    fn find_at(&self, units: &[u16], start: usize, sticky: bool) -> Option<Found> {
        let canonical: Vec<u16>;
        let units = if self.canonical_text {
            canonical = units
                .iter()
                .map(|unit| legacy_case::canonical(*unit))
                .collect();
            &canonical
        } else {
            units
        };
        let found = if self.unicode {
            self.regex.find_from_utf16(units, start).next()
        } else {
            self.regex.find_from_ucs2(units, start).next()
        }?;
        if sticky && found.range.start != start {
            return None;
        }
        let names = found
            .named_groups()
            .map(|(name, range)| (name.to_owned(), range))
            .collect();
        Some(Found {
            range: found.range,
            captures: found.captures,
            names,
        })
    }

    /// ECMAScript's `RegExpBuiltinExec`: `lastIndex` is read and written only when global or
    /// sticky; past the end, or with a sticky pattern that does not match there, it goes back to 0.
    fn builtin_exec(&mut self, units: &[u16]) -> Option<Found> {
        let stateful = self.global() || self.sticky();
        let mut start = if stateful { self.last_index } else { 0 };
        // With `u` the text is code points: a `lastIndex` inside a pair reads from its start.
        if self.unicode && start > 0 && inside_pair(units, start) {
            start -= 1;
        }
        if start > units.len() {
            if stateful {
                self.last_index = 0;
            }
            return None;
        }
        let Some(found) = self.find_at(units, start, self.sticky()) else {
            if stateful {
                self.last_index = 0;
            }
            return None;
        };
        if stateful {
            self.last_index = found.range.end;
        }
        Some(found)
    }
}

impl Found {
    /// The match as `exec` answers it.
    fn to_match(&self, units: &[u16]) -> RegExpMatch {
        let mut captures = vec![Some(text_of(units, self.range.clone()))];
        captures.extend(
            self.captures
                .iter()
                .map(|range| range.clone().map(|range| text_of(units, range))),
        );
        let groups = (!self.names.is_empty()).then(|| {
            self.names
                .iter()
                .map(|(name, range)| (name.clone(), range.clone().map(|r| text_of(units, r))))
                .collect()
        });
        RegExpMatch {
            index: self.range.start,
            captures,
            groups,
        }
    }
}

/// Whether `index` is the low half of a surrogate pair.
fn inside_pair(units: &[u16], index: usize) -> bool {
    matches!(units.get(index), Some(0xDC00..=0xDFFF))
        && index
            .checked_sub(1)
            .is_some_and(|before| matches!(units.get(before), Some(0xD800..=0xDBFF)))
}

/// ECMAScript's `AdvanceStringIndex`: one code unit on, or a whole surrogate pair in `unicode`
/// mode.
fn advance(units: &[u16], index: usize, unicode: bool) -> usize {
    if !unicode {
        return index + 1;
    }
    let pair = matches!(units.get(index), Some(0xD800..=0xDBFF))
        && matches!(units.get(index + 1), Some(0xDC00..=0xDFFF));
    index + if pair { 2 } else { 1 }
}

#[cfg(test)]
mod tests {
    use super::RegExp;

    #[test]
    fn matches_with_lookaround_and_backreferences() {
        let mut regexp = RegExp::new(r"(?<=\$)(\d+)(?!\d)\.\1", "").unwrap();
        let found = regexp.exec("cost $12.12").unwrap();
        assert_eq!(found.index, 6);
        assert_eq!(
            found.captures,
            vec![Some("12.12".into()), Some("12".into())]
        );
    }

    #[test]
    fn counts_utf16_code_units_and_reads_pairs_by_the_u_flag() {
        assert!(!RegExp::new("^.$", "").unwrap().test("😀"));
        assert!(RegExp::new("^.$", "u").unwrap().test("😀"));
        assert_eq!(RegExp::new("b", "").unwrap().exec("😀b").unwrap().index, 2);
    }

    #[test]
    fn keeps_last_index_for_global_and_sticky_patterns() {
        let mut global = RegExp::new("a", "g").unwrap();
        assert!(global.test("aba"));
        assert_eq!(global.last_index(), 1);
        assert!(global.test("aba"));
        assert_eq!(global.last_index(), 3);
        assert!(!global.test("aba"));
        assert_eq!(global.last_index(), 0);
        let mut sticky = RegExp::new("b", "y").unwrap();
        assert!(!sticky.test("ab"));
        sticky.set_last_index(1);
        assert!(sticky.test("ab"));
    }

    #[test]
    fn names_groups_in_the_pattern_s_order() {
        let found = RegExp::new("(?<b>x)(?<a>y)?", "")
            .unwrap()
            .exec("x")
            .unwrap();
        let groups = found.groups.unwrap();
        assert_eq!(groups.keys().collect::<Vec<_>>(), ["b", "a"]);
        assert_eq!(groups.get("a"), Some(&None));
    }

    #[test]
    fn refuses_what_javascript_core_refuses() {
        let error = RegExp::new("(", "").unwrap_err();
        assert_eq!(error.to_string(), "Invalid regular expression: missing )");
        let error = RegExp::new("x", "gg").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid flags supplied to RegExp constructor."
        );
    }

    #[test]
    fn refuses_a_pattern_longer_than_javascript_core_compiles() {
        let longest = "a".repeat(1024 * 1024);
        assert!(RegExp::new(&longest, "").is_ok());
        let too_large = "Invalid regular expression: regular expression too large";
        let error = RegExp::new(&format!("{longest}("), "").unwrap_err();
        assert_eq!(error.to_string(), too_large);
        let pairs = "😀".repeat(512 * 1024);
        assert_eq!(
            RegExp::new(&format!("{pairs}a"), "u")
                .unwrap_err()
                .to_string(),
            too_large
        );
        let error = RegExp::new(&format!("{longest}a"), "gq").unwrap_err();
        assert_eq!(
            error.to_string(),
            "Invalid flags supplied to RegExp constructor."
        );
    }

    #[test]
    fn a_shared_test_leaves_the_pattern_as_it_was() {
        let global = RegExp::new("a", "g").unwrap();
        assert!(global.test_shared("ba"));
        assert!(global.test_shared("ba"));
        assert_eq!(global.last_index(), 0);
        assert!(!RegExp::new("^a", "").unwrap().test_shared("ba"));
    }
}
