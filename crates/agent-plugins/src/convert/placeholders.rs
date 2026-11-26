//! Port of `src/convert/placeholders.ts`, switched.
//!
//! The two forms' path placeholders (Agent Plugins Extended §18.3.3, §18.4.2 item 3, §18.4.3 item
//! 3): the core's `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` (core §9.2) and Claude Code's
//! `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`, which name the same directories (J18.13).
//! A rename is one pass over the original text, so a renamed placeholder is never renamed again.
//!
//! A `form` is a `PluginForm` of `src/loading/core/form.ts`, `core` or `claude-code`: held as its
//! string until that module is in the core.

use indexmap::IndexMap;

use crate::js::in_key_order;

/// The core's two placeholders and the Claude Code name of each.
const TO_CLAUDE_CODE: &[(&str, &str)] = &[
    ("${PLUGIN_ROOT}", "${CLAUDE_PLUGIN_ROOT}"),
    ("${PLUGIN_DATA}", "${CLAUDE_PLUGIN_DATA}"),
];

/// Claude Code's two placeholders and the core's name of each.
const TO_CORE: &[(&str, &str)] = &[
    ("${CLAUDE_PLUGIN_ROOT}", "${PLUGIN_ROOT}"),
    ("${CLAUDE_PLUGIN_DATA}", "${PLUGIN_DATA}"),
];

/// One global pass: each place the text holds a name of `table` (`CORE_PLACEHOLDERS`,
/// `CLAUDE_PLACEHOLDERS`) takes the name's row in `table`, or `replacement` when it is given.
fn replace_names(text: &str, names: &[(&str, &str)], replacement: Option<&str>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while !rest.is_empty() {
        let found = names.iter().find(|(name, _)| rest.starts_with(name));
        if let Some((name, renamed)) = found {
            out.push_str(replacement.unwrap_or(renamed));
            rest = rest.get(name.len()..).unwrap_or_default();
        } else {
            let width = rest.chars().next().map_or(1, char::len_utf8);
            out.push_str(rest.get(..width).unwrap_or_default());
            rest = rest.get(width..).unwrap_or_default();
        }
    }
    out
}

/// `${PLUGIN_ROOT}` and `${PLUGIN_DATA}` written as Claude Code's names.
#[must_use]
pub fn to_claude_code_placeholders(text: &str) -> String {
    replace_names(text, TO_CLAUDE_CODE, None)
}

/// `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}` written as the core's names.
#[must_use]
pub fn to_core_placeholders(text: &str) -> String {
    replace_names(text, TO_CORE, None)
}

/// Whether text holds a `${`, which the other form would expand or keep literally.
#[must_use]
pub fn holds_reference(text: &str) -> bool {
    text.contains("${")
}

/// The placeholders `form` writes its own paths with.
fn own_placeholders(form: &str) -> &'static [(&'static str, &'static str)] {
    if form == "core" {
        TO_CLAUDE_CODE
    } else {
        TO_CORE
    }
}

/// Whether text written for `form` holds a `${` other than that form's two path placeholders.
#[must_use]
pub fn holds_other_reference(text: &str, form: &str) -> bool {
    holds_reference(&replace_names(text, own_placeholders(form), Some("")))
}

/// Whether text holds the core's own placeholders, which a core client expands in `args`.
#[must_use]
pub fn holds_core_placeholder(text: &str) -> bool {
    TO_CLAUDE_CODE.iter().any(|(name, _)| text.contains(name))
}

/// The `env` entries that bridge the environment in configuration (J18.18), by the form whose
/// configuration holds them: a Claude Code stdio server gets the core's two variables, a core stdio
/// server Claude Code's two.
fn bridges(form: &str) -> [(&'static str, &'static str); 2] {
    if form == "core" {
        [
            ("CLAUDE_PLUGIN_ROOT", "${PLUGIN_ROOT}"),
            ("CLAUDE_PLUGIN_DATA", "${PLUGIN_DATA}"),
        ]
    } else {
        [
            ("PLUGIN_ROOT", "${CLAUDE_PLUGIN_ROOT}"),
            ("PLUGIN_DATA", "${CLAUDE_PLUGIN_DATA}"),
        ]
    }
}

/// The form a conversion from each form writes.
fn other_form(form: &str) -> &'static str {
    if form == "claude-code" {
        "core"
    } else {
        "claude-code"
    }
}

fn is_bridge(form: &str, key: &str, value: &str) -> bool {
    bridges(form)
        .iter()
        .any(|(name, bridged)| *name == key && *bridged == value)
}

/// A stdio server's `env` moved from one form to the other: the source's own bridge entries left out,
/// since the target sets those variables itself to the same directories (J18.13; judgment, so that a
/// converted package converts back to what it was), the values renamed by `rename`, then the
/// target's bridge added where the server sets no such variable.
///
/// Not exported: the TypeScript's `rename` is a synchronous host function and its callers are
/// synchronous, so the function stays TypeScript there (docs/RUST.md, "What stays TypeScript"); this
/// copy is the core's own.
pub fn bridged_environment(
    env: Option<&IndexMap<String, String>>,
    from: &str,
    rename: impl Fn(&str) -> String,
) -> IndexMap<String, String> {
    let mut kept: Vec<(String, String)> = Vec::new();
    for (key, value) in env.into_iter().flatten() {
        if !is_bridge(from, key, value) {
            kept.push((key.clone(), rename(value)));
        }
    }
    let added: Vec<(String, String)> = bridges(other_form(from))
        .iter()
        .filter(|(key, _)| !kept.iter().any(|(name, _)| name == key))
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    in_key_order(kept.into_iter().chain(added).collect())
}

crate::registry::exports! {
    "convert/placeholders" {
        "toClaudeCodePlaceholders" => ok |text: String| to_claude_code_placeholders(&text);
        "toCorePlaceholders" => ok |text: String| to_core_placeholders(&text);
        "holdsReference" => ok |text: String| holds_reference(&text);
        "holdsOtherReference" => ok |text: String, form: String| holds_other_reference(&text, &form);
        "holdsCorePlaceholder" => ok |text: String| holds_core_placeholder(&text);
    }
}
