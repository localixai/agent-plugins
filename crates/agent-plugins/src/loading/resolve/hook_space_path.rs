//! Ported from `src/loading/resolve/hook-space-path.ts`; only the core uses it now, so it has no wrapper.
//!
//! errors.md, "Hook needs quotes around a path with a space" (spec §17.18.8): an `asyncRewake` hook
//! in shell form whose `command` has an interpreter followed by an unquoted path placeholder that
//! resolves to a path with a space does not run, and the conversation view gets
//! `<Event> hook needs quotes around a path with a space (<script>)`.
//!
//! Judgment (spec/judgments.md): the docs name `python3` as an example interpreter; the kit takes the
//! script interpreters below (a `python` with a version suffix included), and the placeholder as
//! `${NAME}` or `$NAME` of the three path variables at the start of the word after the interpreter.

use std::sync::LazyLock;

use regex::Regex;

use super::substituter::VariableSource;
use super::variables::{PLUGIN_VARIABLES, PluginVariable};
use crate::js::{Pattern, is_whitespace, regex_test, trim};

static INTERPRETER: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:python[0-9.]*|node|bash|sh|zsh|ruby|perl|bun|deno|php)$").ok()
});
static PLACEHOLDER: Pattern =
    LazyLock::new(|| Regex::new(r"^\$(?:\{([A-Za-z0-9_]+)\}|([A-Za-z0-9_]+))").ok());

/// The notification, `Stop hook needs quotes around a path with a space (check.py)`.
#[must_use]
pub fn hook_needs_quotes(event: Option<&str>, script: &str) -> String {
    let event = event.map_or_else(|| "Hook".to_owned(), |event| format!("{event} hook"));
    format!("{event} needs quotes around a path with a space ({script})")
}

/// The last segment of a path, `/` or `\` separated.
fn last_segment(path: &str) -> &str {
    path.split(['\\', '/'])
        .rfind(|part| !part.is_empty())
        .unwrap_or(path)
}

/// The path variable an interpreter's first argument starts with, unquoted, and the rest of the word.
fn placeholder_after_interpreter(command: &str) -> Option<(PluginVariable, &str)> {
    let mut words = trim(command)
        .split(is_whitespace)
        .filter(|word| !word.is_empty());
    let (program, word) = (words.next()?, words.next()?);
    if !regex_test(&INTERPRETER, program) {
        return None;
    }
    let found = PLACEHOLDER.as_ref()?.captures(word)?;
    let name = found.get(1).or_else(|| found.get(2))?.as_str();
    let name = PLUGIN_VARIABLES
        .into_iter()
        .find(|variable| variable.as_str() == name)?;
    let rest = word.get(found.get(0)?.end()..).unwrap_or_default();
    Some((name, rest))
}

/// The script a command would run from an unquoted path that has a space once resolved, or
/// `undefined` when the command is safe as written.
pub async fn unquoted_space_path(
    command: &str,
    substitution: &dyn VariableSource,
) -> Option<String> {
    let (name, rest) = placeholder_after_interpreter(command)?;
    let value = substitution.variable(name).await;
    value
        .chars()
        .any(is_whitespace)
        .then(|| last_segment(&format!("{value}{rest}")).to_owned())
}
