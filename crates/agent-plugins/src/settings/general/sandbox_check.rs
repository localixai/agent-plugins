//! Ported from `src/settings/general/sandbox-check.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! A managed `sandbox` block, validated field by field (managed-settings.md, "Invalid values
//! inside `sandbox`"): one invalid value never drops the block. A quoted Boolean counts as that
//! Boolean; an invalid Boolean reads as the value that keeps the sandbox strictest; an invalid list
//! entry is dropped and a list with no valid entry does not apply; an invalid deny list withholds
//! the allow lists it restrains.

use indexmap::IndexSet;

use super::sandbox_credentials::repair_credentials;
use super::sandbox_fields::{BOOLEAN_STANCES, SANDBOX_LISTS, Stance, WITHHELD_BY, stance_value};
use crate::format::issues::PluginIssue;
use crate::js::{JsObject, JsValue};
use crate::settings::paths::{SettingsPath, value_at, with_value_at};
use crate::settings::repair::support::{
    RepairAt, Repaired, kept, must_be, note, present_but_invalid,
};

struct State<'a> {
    at: &'a RepairAt,
    settings: JsValue,
    found: Vec<PluginIssue>,
}

fn field(path: &str) -> SettingsPath {
    SettingsPath::Dotted(format!("sandbox.{path}"))
}

fn name_of(path: &str) -> String {
    format!("sandbox.{path}")
}

fn at_of(path: &str) -> String {
    format!(".{path}")
}

/// One repair of the field at a path: the new value (`undefined` removes it) and what to say;
/// `None` leaves the field as it is.
type Changed = Option<(JsValue, Vec<PluginIssue>)>;

fn apply_at<'a>(
    mut state: State<'a>,
    path: &str,
    step: impl Fn(&JsValue, &RepairAt) -> Changed,
) -> State<'a> {
    let current = value_at(&state.settings, &field(path));
    if current.is_undefined() {
        return state;
    }
    let Some((value, found)) = step(&current, state.at) else {
        return state;
    };
    state.settings = with_value_at(&state.settings, &field(path), &value);
    state.found.extend(found);
    state
}

fn boolean_step(stance: Stance, path: &str, value: &JsValue, at: &RepairAt) -> Changed {
    if value.is_boolean() {
        return None;
    }
    let name = name_of(path);
    if let Some(text @ ("true" | "false")) = value.as_str() {
        return Some((
            JsValue::Bool(text == "true"),
            vec![note(
                at,
                &at_of(path),
                &format!("\"{name}\" is the text \"{text}\": drop the quotes"),
            )],
        ));
    }
    Some(match stance_value(stance) {
        None => (
            JsValue::Undefined,
            vec![note(
                at,
                &at_of(path),
                &format!("\"{name}\" must be true or false; the key is ignored"),
            )],
        ),
        Some(reads) => (
            JsValue::Bool(reads),
            vec![present_but_invalid(
                at,
                &format!("it is treated as {reads}"),
                &at_of(path),
            )],
        ),
    })
}

/// What a list comes to, and whether it or any entry was invalid. A list that is no array, or
/// that lost every entry it had, does not apply (`entries` is `None`); an empty list has no invalid
/// entry and stays (spec/judgments.md).
fn read_list(value: &JsValue) -> (Option<Vec<JsValue>>, bool) {
    let Some(items) = value.as_array() else {
        return (None, true);
    };
    let entries: Vec<JsValue> = items
        .iter()
        .filter(|item| item.is_string())
        .cloned()
        .collect();
    let faulty = entries.len() < items.len();
    let applies = items.is_empty() || !entries.is_empty();
    (applies.then_some(entries), faulty)
}

fn list_step(path: &str, value: &JsValue, at: &RepairAt) -> Changed {
    let (entries, faulty) = read_list(value);
    if !faulty {
        return None;
    }
    let message = if entries.is_none() {
        format!(
            "\"{}\" is not a list with a valid entry; the list does not apply",
            name_of(path)
        )
    } else {
        format!(
            "\"{}\" has an invalid entry; the entry is skipped",
            name_of(path)
        )
    };
    Some((
        entries.map_or(JsValue::Undefined, JsValue::Array),
        vec![note(at, &at_of(path), &message)],
    ))
}

fn withheld_by(deny: &str) -> &'static [&'static str] {
    WITHHELD_BY
        .iter()
        .find(|(name, _)| *name == deny)
        .map_or(&[], |(_, allows)| allows)
}

/// The allow lists a faulty deny list withholds.
fn withheld<'a>(state: State<'a>, faulty: &[&str]) -> State<'a> {
    let paths: IndexSet<&str> = faulty
        .iter()
        .flat_map(|deny| withheld_by(deny))
        .copied()
        .collect();
    let names = faulty
        .iter()
        .map(|deny| name_of(deny))
        .collect::<Vec<_>>()
        .join(", ");
    paths.into_iter().fold(state, |current, path| {
        apply_at(current, path, |_, at| {
            Some((
                JsValue::Undefined,
                vec![note(
                    at,
                    &at_of(path),
                    &format!("\"{}\" is withheld until you fix {names}", name_of(path)),
                )],
            ))
        })
    })
}

fn is_faulty(settings: &JsValue, path: &str) -> bool {
    let current = value_at(settings, &field(path));
    !current.is_undefined() && read_list(&current).1
}

fn credentials_of(mut state: State<'_>) -> State<'_> {
    let block = value_at(&state.settings, &field("credentials"));
    if block.is_undefined() {
        return state;
    }
    let repaired = repair_credentials(state.at, &block);
    state.settings = with_value_at(&state.settings, &field("credentials"), &repaired.value);
    state.found.extend(repaired.found);
    state
}

/// `sandbox`: outside the policy tier the block stays as written (a managed-only lock inside it is
/// left out by its own row's scope line, `readSettingsLayer`); in the policy tier it is repaired
/// field by field.
#[must_use]
pub fn sandbox(value: &JsValue, at: &RepairAt) -> Repaired {
    if !value.is_object() {
        return must_be(at, "an object");
    }
    if !at.closed {
        return kept(value.clone(), Vec::new());
    }
    closed_sandbox(State {
        at,
        settings: JsValue::Object(JsObject::from([("sandbox".to_owned(), value.clone())])),
        found: Vec::new(),
    })
}

fn closed_sandbox(start: State<'_>) -> Repaired {
    let faulty: Vec<&str> = WITHHELD_BY
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| is_faulty(&start.settings, path))
        .collect();
    let booleans = BOOLEAN_STANCES
        .iter()
        .fold(credentials_of(start), |state, (path, stance)| {
            apply_at(state, path, |value, at| {
                boolean_step(*stance, path, value, at)
            })
        });
    let lists = SANDBOX_LISTS.iter().fold(booleans, |state, path| {
        apply_at(state, path, |value, at| list_step(path, value, at))
    });
    let done = withheld(lists, &faulty);
    let block = done.settings.get("sandbox").cloned().unwrap_or_default();
    kept(block, done.found)
}
