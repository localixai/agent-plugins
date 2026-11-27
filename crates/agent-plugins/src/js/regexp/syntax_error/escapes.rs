//! The scans that tell which reason JavaScriptCore gives for an escape with `u`: the first `\\u`
//! it refuses, the first escape it may not hold, and a group name that holds a backslash.

use super::super::refusal::unicode_escape;
use super::scan::{group_header, structural};

/// What JavaScriptCore names for the first `\\u` escape it refuses.
pub(super) fn first_unicode_escape(source: &[char]) -> Option<&'static str> {
    let units: Vec<u32> = source
        .iter()
        .map(|character| u32::from(*character))
        .collect();
    let mut at = 0;
    while let Some(character) = source.get(at) {
        if *character == '\\'
            && source.get(at + 1) == Some(&'u')
            && let Some(reason) = unicode_escape(&units, at)
        {
            return Some(reason);
        }
        at += if *character == '\\' { 2 } else { 1 };
    }
    None
}

/// What JavaScriptCore names for the first escape a pattern with `u` may not hold.
pub(super) fn escape_reason(source: &[char]) -> &'static str {
    let groups = groups(source);
    let mut reference_reason = None;
    let (mut at, mut in_class) = (0, false);
    while let Some(character) = source.get(at) {
        if *character != '\\' {
            in_class = match character {
                '[' => true,
                ']' => false,
                _ => in_class,
            };
            at += 1;
            continue;
        }
        let after = source.get(at + 2);
        match source.get(at + 1) {
            Some('c') if after.is_none() => return "invalid escaped character for Unicode pattern",
            Some('c') if !after.is_some_and(char::is_ascii_alphabetic) => {
                return "invalid \\c escape for Unicode pattern";
            }
            Some('0') if after.is_some_and(char::is_ascii_digit) => {
                return "invalid octal escape for Unicode pattern";
            }
            Some('x')
                if !source
                    .get(at + 2..at + 4)
                    .is_some_and(|digits| digits.iter().all(char::is_ascii_hexdigit)) =>
            {
                return "invalid escaped character for Unicode pattern";
            }
            Some('1'..='9') if in_class => return "invalid octal escape for Unicode pattern",
            // A number past the groups is known only once the whole pattern is read.
            Some('1'..='9') if reference(source, at + 1).is_none_or(|number| number > groups) => {
                reference_reason =
                    reference_reason.or(Some("invalid backreference for Unicode pattern"));
            }
            Some('1'..='9') => {}
            Some(letter) if !is_unicode_escape(*letter) => {
                return "invalid escaped character for Unicode pattern";
            }
            _ => {}
        }
        at += 2;
    }
    reference_reason
        .map(|reason| structural(source, true).unwrap_or(reason))
        .unwrap_or("invalid escaped character for Unicode pattern")
}

/// Whether a group's name or a `\\k<…>` holds a backslash: JavaScriptCore reads a name's escape
/// as `\\u`.
pub(super) fn escaped_name(source: &[char]) -> bool {
    (0..source.len()).any(|at| {
        let opens = source.get(at..at + 3) == Some(&['(', '?', '<'])
            || source.get(at..at + 3) == Some(&['\\', 'k', '<']);
        let name = source.get(at + 3..).unwrap_or_default();
        let part = |c: &&char| c.is_alphanumeric() || matches!(c, '_' | '$' | '\\');
        opens && name.iter().take_while(part).any(|c| *c == '\\')
    })
}

/// How many capturing groups `source` opens, outside classes and escapes.
fn groups(source: &[char]) -> usize {
    let (mut count, mut at, mut in_class) = (0, 0, false);
    while let Some(character) = source.get(at) {
        match (in_class, character) {
            (_, '\\') => at += 1,
            (false, '[') => in_class = true,
            (true, ']') => in_class = false,
            (false, '(') if source.get(at + 1) != Some(&'?') || group_header(source, at) > 4 => {
                count += 1;
            }
            _ => {}
        }
        at += 1;
    }
    count
}

/// The number of the backreference whose first digit is at `at`.
fn reference(source: &[char], at: usize) -> Option<usize> {
    let digits: String = source
        .get(at..)?
        .iter()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

/// A character a pattern with `u` may escape.
fn is_unicode_escape(letter: char) -> bool {
    "^$\\.*+?()[]{}|/-bBdDsSwWfnrtv0cxupPk".contains(letter)
}
