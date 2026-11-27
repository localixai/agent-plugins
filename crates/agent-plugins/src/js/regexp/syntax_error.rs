//! The `SyntaxError` `new RegExp` throws, worded as JavaScriptCore words it: `Invalid flags
//! supplied to RegExp constructor.` for flags, `Invalid regular expression: <reason>` for a
//! pattern. The engine names fewer reasons than JavaScriptCore, so where one of its reasons
//! stands for several, the pattern's text tells which. No module of the package shows the reason:
//! a pattern that does not compile matches nothing, or is reported in the package's own words.

mod escapes;
mod scan;

use std::fmt;

use escapes::{escape_reason, escaped_name, first_unicode_escape};
use scan::{atom_reason, braces_out_of_order, depth, structural};

/// What `new RegExp(source, flags)` threw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxError {
    message: String,
}

impl SyntaxError {
    /// Flags JavaScript does not accept.
    pub(super) fn flags() -> Self {
        Self {
            message: "Invalid flags supplied to RegExp constructor.".to_owned(),
        }
    }

    /// A pattern the engine refused, with its reason as JavaScriptCore gives it.
    pub(super) fn pattern(source: &str, error: &regress::Error, unicode: bool) -> Self {
        let units: Vec<char> = source.chars().collect();
        let reason = reason(&units, &error.text, unicode);
        Self {
            message: format!("Invalid regular expression: {reason}"),
        }
    }

    /// A pattern the engine accepts and JavaScriptCore refuses (`source::refusal`).
    pub(super) fn refused(reason: &str) -> Self {
        Self {
            message: format!("Invalid regular expression: {reason}"),
        }
    }
}

impl fmt::Display for SyntaxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

/// JavaScriptCore's reason for the engine's error text.
fn reason(source: &[char], text: &str, unicode: bool) -> String {
    // JavaScriptCore refuses a `\u` escape as it reads it, where the engine may name an error past it.
    if let Some(words) = first_unicode_escape(source).filter(|_| unicode) {
        return words.to_owned();
    }
    let words = match text {
        "Unbalanced parenthesis" if depth(source) > 0 => "missing )",
        "Unbalanced parenthesis" => "unmatched parentheses",
        "Unbalanced bracket" | "Unbalanced class set bracket" => {
            "missing terminating ] for character class"
        }
        "Range values reversed, start char code is greater than end char code." => {
            "range out of order in character class"
        }
        "Invalid character range" | "Invalid class set range" => {
            "invalid range in character class for Unicode pattern"
        }
        "Invalid quantifier" if braces_out_of_order(source) => {
            "numbers out of order in {} quantifier"
        }
        "Invalid quantifier" => "incomplete {} quantifier for Unicode pattern",
        "Invalid braced quantifier" | "Quantifier not allowed here" | "Invalid atom character" => {
            atom_reason(source, unicode)
        }
        "Incomplete escape" | "Unterminated escape" => "\\ at end of pattern",
        "Duplicate capture group name" => "duplicate group specifier name",
        "Invalid token at named capture group identifier"
        | "Invalid named backreference syntax"
            if escaped_name(source) =>
        {
            "invalid Unicode \\u escape"
        }
        "Invalid token at named capture group identifier" if contains(source, &['(', '?', '<']) => {
            "invalid group specifier name"
        }
        "Invalid token at named capture group identifier" => "unrecognized character after (?",
        "Invalid named backreference syntax" if unicode => {
            "invalid escaped character for Unicode pattern"
        }
        "Invalid named backreference syntax" => "invalid \\k<> named backreference",
        "Invalid group modifier" => modifier_reason(source),
        "Invalid property name"
        | "Invalid property escape"
        | "Invalid character at property escape start" => "invalid property expression",
        "Invalid unicode escape" => {
            first_unicode_escape(source).unwrap_or("invalid Unicode \\u escape")
        }
        "Invalid character escape" => escape_reason(source),
        // JavaScriptCore checks the names a pattern refers to once it has read the whole of it.
        other if other.starts_with("Backreference to invalid named capture group") => {
            structural(source, unicode).unwrap_or("invalid \\k<> named backreference")
        }
        other => other,
    };
    words.to_owned()
}

fn contains(source: &[char], needle: &[char]) -> bool {
    source.windows(needle.len()).any(|window| window == needle)
}

/// What JavaScriptCore names for the first modifier group `(?ims-ims:` it refuses: a character
/// that is no flag before the `:`, else flags it cannot apply (repeated, on and off, none).
fn modifier_reason(source: &[char]) -> &'static str {
    let start = source
        .windows(3)
        .position(|window| matches!(window, ['(', '?', 'i' | 'm' | 's' | '-']));
    let Some(start) = start else {
        return "unrecognized character after (?";
    };
    let flags = source
        .get(start + 2..)
        .unwrap_or_default()
        .iter()
        .take_while(|character| matches!(character, 'i' | 'm' | 's' | '-'))
        .count();
    match source.get(start + 2 + flags) {
        Some(':') => "invalid regular expression modifier",
        None if source.get(start + 2) == Some(&'-') && flags == 1 => {
            "invalid regular expression modifier"
        }
        None => "missing )",
        Some(_) => "unrecognized character after (?",
    }
}
