//! Port of `src/format/names.ts`, switched.
//!
//! Plugin names (plugins/manifest-reference.md, "`name`"). Every component is namespaced under the
//! name, so it may not be empty or hold a space, `@`, `:`, a path separator, a control character
//! or a bidirectional-formatting character: such a name never loads. The rest is the validator's;
//! its wording is the oracle's (docs/SPEC.md, "Sources of truth").

use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, error, warning};
use crate::js::{is_whitespace, trim};

/// A control (C0 or C1) or bidirectional-formatting character (`Bidi_Control`: the embeddings,
/// overrides and isolates, and LRM, RLM and ALM): the one class the loader and the validator
/// refuse in a plugin or marketplace name. (`char::is_control` is `\p{Cc}`.)
#[must_use]
pub fn is_control_or_bidi(character: char) -> bool {
    character.is_control()
        || matches!(
            character,
            '\u{61C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}'
        )
}

/// Whether a text holds a character of `CONTROL_OR_BIDI`.
#[must_use]
pub fn contains_control_or_bidi(text: &str) -> bool {
    text.chars().any(is_control_or_bidi)
}

/// The separators of the forbidden class: `\s`, `@`, `:`, `/` and `\`.
fn is_separator(character: char) -> bool {
    is_whitespace(character) || matches!(character, '@' | ':' | '/' | '\\')
}

fn forbidden(name: &str) -> bool {
    name.chars().any(is_separator) || contains_control_or_bidi(name)
}

/// `^[a-z0-9]+(?:-[a-z0-9]+)*$`.
fn is_kebab(name: &str) -> bool {
    name.split('-').all(|word| {
        !word.is_empty()
            && word
                .chars()
                .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
    })
}

/// Names and prefixes that pass as one of Anthropic's own (the `name` section's table).
const RESERVED: [&str; 5] = [
    "claude",
    "anthropic",
    "anthropics",
    "claude-code",
    "claude-mods",
];
const RESERVED_PREFIXES: [&str; 4] = ["claude-", "anthropic-", "anthropics-", "cc-plugin-"];
const BESIDE_OFFICIAL: [&str; 2] = ["claude", "anthropic"];
const BRAND_WORDS: [&str; 3] = ["claude", "anthropic", "anthropics"];

/// The check ignores case and treats any run of separators as one.
fn folded(name: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for character in name.to_lowercase().chars() {
        if character.is_ascii_lowercase() || character.is_ascii_digit() {
            out.push(character);
            in_run = false;
        } else if !in_run {
            out.push('-');
            in_run = true;
        }
    }
    let without_start = out.strip_prefix('-').unwrap_or(&out);
    without_start
        .strip_suffix('-')
        .unwrap_or(without_start)
        .to_owned()
}

fn beside_official(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(index, word)| {
        let before = index
            .checked_sub(1)
            .and_then(|at| words.get(at))
            .copied()
            .unwrap_or("");
        let after = words.get(index + 1).copied().unwrap_or("");
        *word == "official"
            && (BESIDE_OFFICIAL.contains(&before) || BESIDE_OFFICIAL.contains(&after))
    })
}

/// Whether a plugin of this name can load at all.
#[must_use]
pub fn name_load_issues(name: &str) -> Vec<PluginIssue> {
    if trim(name).is_empty() {
        return vec![error("name", "Plugin name is empty")];
    }
    if !forbidden(name) {
        return Vec::new();
    }
    vec![error(
        "name",
        &format!(
            "Plugin name \"{name}\" may not contain spaces, \"@\", \":\", path separators or control characters"
        ),
    )]
}

/// Where a name is reported: the manifest's own `name`, or a marketplace entry's (shorter advice).
pub const NAME_CONTEXTS: [NameContext; 2] = [NameContext::Manifest, NameContext::Entry];

/// One of `NAME_CONTEXTS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NameContext {
    /// The manifest's own `name`.
    Manifest,
    /// A marketplace entry's.
    Entry,
}

const RESERVED_RULE: &str = "A third party's plugin name cannot start with \"claude-\", \"anthropic-\", \"anthropics-\", or \"cc-plugin-\", be \"claude\", \"anthropic\", \"anthropics\", \"claude-code\", or \"claude-mods\", or put \"official\" beside \"claude\" or \"anthropic\".";
const NAME_IT: &str = "Name it for what it does.";

/// What the manifest adds: a marketplace of Anthropic's own is validated on its own.
const fn own_marketplace(context: NameContext) -> &'static str {
    match context {
        NameContext::Manifest => {
            " If this is one of Anthropic's own plugins, validate the marketplace that lists it."
        }
        NameContext::Entry => "",
    }
}

/// The reserved-name report of `claude plugin validate`: an error for a name that passes as
/// Anthropic's own, a warning for one that reads as such (oracle 2.1.319).
#[must_use]
pub fn name_reserved_issues(name: &str, context: Option<NameContext>) -> Vec<PluginIssue> {
    let context = context.unwrap_or(NameContext::Manifest);
    let key = folded(name);
    let words: Vec<&str> = key.split('-').collect();
    let reserved = RESERVED.contains(&key.as_str())
        || RESERVED_PREFIXES
            .iter()
            .any(|prefix| key.starts_with(prefix))
        || beside_official(&words);
    let tail = format!("{NAME_IT}{}", own_marketplace(context));
    if reserved {
        return vec![error(
            "name",
            &format!(
                "Plugin name \"{name}\" is reserved: it passes as one of Anthropic's own. {RESERVED_RULE} {tail}"
            ),
        )];
    }
    if words.iter().any(|word| BRAND_WORDS.contains(word)) {
        return vec![warning(
            "name",
            &format!("Plugin name \"{name}\" reads as one of Anthropic's own. {tail}"),
        )];
    }
    Vec::new()
}

/// The characters a path load refuses beside the ones that are errors: whitespace other than a
/// space, `@`, `:`, separators and invisible marks.
fn refused_by_path(character: char) -> bool {
    is_separator(character)
        || matches!(
            character,
            '\u{200B}'..='\u{200D}' | '\u{2060}' | '\u{FEFF}' | '\u{61C}'
        )
}

/// `^[A-Za-z0-9][A-Za-z0-9._-]*$`.
fn is_install_id(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

const KEBAB_SYNC: &str = "Claude.ai marketplace sync requires kebab-case (lowercase letters, digits, and hyphens only, e.g., \"my-plugin\").";

/// What Claude Code does with a name that is not kebab-case, by how badly it breaks.
fn kebab_advice(name: &str) -> String {
    if name.chars().any(refused_by_path) {
        return format!(
            "Claude Code refuses a plugin with this name when it is loaded by path (such as --plugin-dir), and cannot install it from a marketplace that lists it under this name. The {KEBAB_SYNC}"
        );
    }
    if is_install_id(name) {
        format!("Claude Code accepts it, but the {KEBAB_SYNC}")
    } else {
        format!(
            "Claude Code cannot install it from a marketplace that lists it under this name, and the {KEBAB_SYNC}"
        )
    }
}

/// The kebab-case advice of `claude plugin validate`.
fn name_kebab_issues(name: &str) -> Vec<PluginIssue> {
    if is_kebab(name) {
        Vec::new()
    } else {
        vec![warning(
            "name",
            &format!(
                "Plugin name \"{name}\" is not kebab-case. {}",
                kebab_advice(name)
            ),
        )]
    }
}

/// What `claude plugin validate` adds about a name that loads. The reserved-name report is left
/// out for a plugin an official marketplace lists (Anthropic's own plugins carry such names) and
/// for one its marketplace entry already reported.
#[must_use]
pub fn name_validation_issues(
    name: &str,
    skip_reserved: bool,
    context: Option<NameContext>,
) -> Vec<PluginIssue> {
    let mut issues = if skip_reserved {
        Vec::new()
    } else {
        name_reserved_issues(name, context)
    };
    issues.extend(name_kebab_issues(name));
    issues
}

crate::registry::exports! {
    "format/names" {
        "CONTROL_OR_BIDI" => ok || crate::registry::RegExp { source: "[\\p{Cc}\\p{Bidi_C}]".to_owned(), flags: "u".to_owned() };
        "nameLoadIssues" => ok |name: String| name_load_issues(&name);
        "NAME_CONTEXTS" => ok || NAME_CONTEXTS;
        "nameReservedIssues" => ok |name: String, context: Option<NameContext>| name_reserved_issues(&name, context);
        "nameValidationIssues" => ok |name: String, skip_reserved: Option<bool>, context: Option<NameContext>| name_validation_issues(&name, skip_reserved.unwrap_or(false), context);
    }
}
