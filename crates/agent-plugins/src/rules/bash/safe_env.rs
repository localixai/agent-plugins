//! Port of `src/rules/bash/safe-env.ts`, switched.
//!
//! "Certain known-safe environment variables" (permissions.md, "Bash > Wrappers"): the docs name
//! `NODE_ENV`, `LANG` and `NO_COLOR`; the rest of the set, and the values each accepts, are what
//! Claude Code 2.1.322 let through, read-only or past `Bash(echo hi)` (spec/judgments.md). A value
//! a variable or substitution writes is never safe. The same set serves the read-only set and the
//! allow rules, which a host extends with `MatchContext.safeEnvVars` (any value).

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{Pattern, code_unit_length, regex_test, slice_code_units};

/// A word of the command as written and as the shell reads it: `ShellWord` of
/// `src/rules/bash/words.ts` (its `Word` and `WordValue` of `unquote.ts`), declared here until that
/// module is ported.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellWord {
    /// The word as written.
    pub text: String,
    /// Where it starts in the command.
    pub start: f64,
    /// Where it ends in the command.
    pub end: f64,
    /// A redirection operator (`>`, `2>&`, `<<`), whose target is the next word.
    pub operator: bool,
    /// The word with its quotes, escapes and line continuations removed.
    pub value: String,
    /// A `$` or backtick outside single quotes: a variable or substitution may change the value.
    pub expands: bool,
    /// A `*`, `?` or `[` outside quotes: the shell may expand the word to file names.
    pub globs: bool,
}

type ValueCheck = fn(&str) -> bool;

const fn any(_value: &str) -> bool {
    true
}

static LOCALE_VALUE: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:|C|POSIX|(?:C|[a-z]{2,3}_[A-Z]{2})\.(?:UTF-8|utf-?8)(?:@[a-z]+)?)$").ok()
});

/// `C`, `POSIX`, empty, or a UTF-8 locale: `LANG=en_US.ISO-8859-1` and `LANG=en_US` were asked.
fn locale(value: &str) -> bool {
    regex_test(&LOCALE_VALUE, value)
}

/// "COLUMNS has integer attribute": a non-integer was asked (`/^\d+$/`, ASCII digits).
fn integer(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// `TZ=America/New_York` passed, `TZ=../../etc` was asked.
fn zone(value: &str) -> bool {
    !value.split('/').any(|segment| segment == "..")
}

const SAFE: &[(&str, ValueCheck)] = &[
    ("NODE_ENV", any),
    ("NO_COLOR", any),
    ("LANG", locale),
    ("LC_ALL", locale),
    ("LC_CTYPE", locale),
    ("LC_TIME", any),
    ("LANGUAGE", any),
    ("TERM", any),
    ("TZ", zone),
    ("COLUMNS", integer),
    ("LINES", integer),
    ("FORCE_COLOR", any),
    ("CLICOLOR", any),
    ("CI", any),
    ("RUST_LOG", any),
    ("RUST_BACKTRACE", any),
    ("PYTHONUNBUFFERED", any),
    ("PYTHONDONTWRITEBYTECODE", any),
    ("GOOS", any),
    ("GOARCH", any),
    ("GREP_COLOR", any),
    ("LS_COLORS", any),
    ("BLOCK_SIZE", any),
    ("TIME_STYLE", any),
];

/// `/^([A-Za-z_][A-Za-z0-9_]*)=/`: the name before the first `=`, when it is one.
fn assignment(text: &str) -> Option<&str> {
    let (name, _) = text.split_once('=')?;
    let mut characters = name.chars();
    let first = characters.next()?;
    let named = (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| character.is_ascii_alphanumeric() || character == '_');
    named.then_some(name)
}

/// The variable a word assigns (`LANG=C`), or `undefined` when it is not an assignment.
#[must_use]
pub fn assigned_variable(word: &ShellWord) -> Option<&str> {
    assignment(&word.text)
}

/// Whether an assignment word is a safe one: a known variable with a value it accepts, or a variable
/// the host names (`hostSafe`, any value). A value the shell expands is never safe.
#[must_use]
pub fn is_safe_assignment(word: &ShellWord, host_safe: &HashSet<String>) -> bool {
    let Some(variable) = assigned_variable(word) else {
        return false;
    };
    if word.expands {
        return false;
    }
    if host_safe.contains(variable) {
        return true;
    }
    let check = SAFE
        .iter()
        .find(|(name, _)| *name == variable)
        .map(|(_, check)| *check);
    check.is_some_and(|check| {
        check(&slice_code_units(
            &word.value,
            code_unit_length(variable) + 1,
            None,
        ))
    })
}

crate::registry::exports! {
    "rules/bash/safe-env" {
        "assignedVariable" => ok |word: ShellWord| assigned_variable(&word).map(str::to_owned);
        "isSafeAssignment" => ok |word: ShellWord, host_safe: Option<crate::registry::Set<String>>|
            is_safe_assignment(&word, &host_safe.map(|set| set.members.into_iter().collect()).unwrap_or_default());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_locale_pattern_compiles() {
        assert!(LOCALE_VALUE.is_some());
        assert!(locale("en_US.UTF-8"));
        assert!(locale(""));
        assert!(!locale("en_US"));
    }
}
