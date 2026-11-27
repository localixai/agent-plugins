//! Port of `src/hooks/env-file/powershell-reader.ts`, switched.
//!
//! hooks.md, "Persisted variables in PowerShell commands": PowerShell never runs `CLAUDE_ENV_FILE`;
//! "Claude Code reads the assignments out of it and copies them into the PowerShell command's
//! environment instead", and "only when every line is one of the following":
//!
//! - a blank line or a `#` comment
//! - one assignment at the start of the line, `export NAME=value`, `declare -x NAME=value` or
//!   `NAME=value`, with a value "Bash would use exactly as written", made of any mix of unquoted text
//!   that uses only letters, digits and `_ @ % + = : , . / -`, text in single quotes, and text in
//!   double quotes where any `$`, backtick or `"` inside is escaped with a backslash
//!
//! "If any line is of another kind ... PowerShell commands receive none of the variables, and
//! `claude --debug` logs `Session environment is not all plain assignments`." On Windows "PowerShell
//! commands also don't receive a variable whose value contains `/` or `\`", and "a sandboxed
//! PowerShell command receives none of the variables".
//!
//! Judgments (spec/judgments.md): a comment is a line whose first character that is not a space or
//! tab is `#`; a blank line holds only spaces and tabs; an assignment has no leading or trailing
//! space; in double quotes a backslash before any character other than `$`, a backtick, `"` or
//! another backslash stays a literal backslash, as in Bash; a name is a Bash identifier; a repeated
//! name takes its last value.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::{Pattern, regex_test};

/// The message `claude --debug` logs, as an issue's text.
pub const NOT_PLAIN_ASSIGNMENTS: &str = "Session environment is not all plain assignments";

static BLANK: Pattern = LazyLock::new(|| Regex::new(r"^[ \t]*$").ok());
static COMMENT: Pattern = LazyLock::new(|| Regex::new(r"^[ \t]*#").ok());
/// `.` is `[^\n\r  ]` (RS6).
static ASSIGNMENT: Pattern = LazyLock::new(|| {
    Regex::new(
        "^(?:(?:export|declare -x)[ \\t]+)?([A-Za-z_][A-Za-z0-9_]*)=([^\\n\\r\u{2028}\u{2029}]*)$",
    )
    .ok()
});

static UNQUOTED: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_@%+=:,./-]+").ok());
static SINGLE_QUOTED: Pattern = LazyLock::new(|| Regex::new(r"^'([^']*)'").ok());
static DOUBLE_QUOTED: Pattern =
    LazyLock::new(|| Regex::new("^\"((?:[^\"\\\\$`]|\\\\[^\\n\\r\u{2028}\u{2029}])*)\"").ok());

/// A value part reads off the front of the text: its value and what is left, or no match.
struct Part<'a> {
    value: String,
    rest: &'a str,
}

/// How a part's value is made of its match: the whole text, the quoted text, or that unescaped.
#[derive(Clone, Copy)]
enum PartValue {
    All,
    Inner,
    Unescaped,
}

/// In double quotes a backslash escapes `$`, a backtick, `"` and a backslash; before anything else it stays.
fn unescape_double(inner: &str) -> String {
    let mut out = String::new();
    let mut characters = inner.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\'
            && let Some(next) = characters.next_if(|next| matches!(next, '$' | '`' | '"' | '\\'))
        {
            out.push(next);
        } else {
            out.push(character);
        }
    }
    out
}

fn take<'a>(pattern: &Pattern, how: PartValue, text: &'a str) -> Option<Part<'a>> {
    let regex = pattern.as_ref()?;
    let captures = regex.captures(text)?;
    let whole = captures.get(0)?;
    let inner = captures.get(1).map_or("", |group| group.as_str());
    let value = match how {
        PartValue::All => whole.as_str().to_owned(),
        PartValue::Inner => inner.to_owned(),
        PartValue::Unescaped => unescape_double(inner),
    };
    Some(Part {
        value,
        rest: text.get(whole.end()..)?,
    })
}

/// The first part reader of `PARTS` that matches the front of the text.
fn first_part(text: &str) -> Option<Part<'_>> {
    take(&UNQUOTED, PartValue::All, text)
        .or_else(|| take(&SINGLE_QUOTED, PartValue::Inner, text))
        // The pattern lets `$`, a backtick and `"` through only after a backslash.
        .or_else(|| take(&DOUBLE_QUOTED, PartValue::Unescaped, text))
}

/// The value of an assignment's right-hand side, or `None` when a part of it is not plain.
fn plain_value(text: &str) -> Option<String> {
    let mut rest = text;
    let mut value = String::new();
    while !rest.is_empty() {
        let part = first_part(rest)?;
        value.push_str(&part.value);
        rest = part.rest;
    }
    Some(value)
}

/// What one line is.
enum Line {
    /// A blank or a comment (`undefined`).
    Skipped,
    /// A line of another kind (`null`).
    Other,
    /// An assignment's name and value.
    Assignment(String, String),
}

/// A line's assignment, `Skipped` for a blank or a comment, `Other` for a line of another kind.
fn read_line(line: &str) -> Line {
    if regex_test(&BLANK, line) || regex_test(&COMMENT, line) {
        return Line::Skipped;
    }
    let Some(captures) = ASSIGNMENT.as_ref().and_then(|regex| regex.captures(line)) else {
        return Line::Other;
    };
    let value = plain_value(captures.get(2).map_or("", |group| group.as_str()));
    match value {
        Some(value) => Line::Assignment(
            captures
                .get(1)
                .map_or_else(String::new, |group| group.as_str().to_owned()),
            value,
        ),
        None => Line::Other,
    }
}

/// What PowerShell commands receive.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PowershellVariables {
    /// What PowerShell commands receive: nothing when a line is not plain.
    pub variables: IndexMap<String, String>,
    /// The issues of the read.
    pub issues: Vec<PluginIssue>,
}

/// What the read takes besides the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct PowershellReadOptions {
    /// A Windows host drops a variable whose value holds `/` or `\`.
    pub windows: bool,
}

/// The variables a PowerShell command receives out of the text of every file the session's hooks wrote.
#[must_use]
pub fn read_powershell_variables(
    text: &str,
    options: PowershellReadOptions,
) -> PowershellVariables {
    let mut assigned: IndexMap<String, String> = IndexMap::new();
    for line in text.split('\n') {
        match read_line(line) {
            Line::Other => {
                return PowershellVariables {
                    variables: IndexMap::new(),
                    issues: vec![warning("CLAUDE_ENV_FILE", NOT_PLAIN_ASSIGNMENTS)],
                };
            }
            Line::Assignment(name, value) => {
                assigned.insert(name, value);
            }
            Line::Skipped => {}
        }
    }
    let received = assigned
        .into_iter()
        .filter(|(_, value)| !(options.windows && value.contains(['/', '\\'])))
        .collect();
    PowershellVariables {
        variables: received,
        issues: Vec::new(),
    }
}

crate::registry::exports! {
    "hooks/env-file/powershell-reader" {
        "NOT_PLAIN_ASSIGNMENTS" => ok || NOT_PLAIN_ASSIGNMENTS;
        "readPowershellVariables" => ok |text: String, options: PowershellReadOptions| read_powershell_variables(&text, options);
    }
}
