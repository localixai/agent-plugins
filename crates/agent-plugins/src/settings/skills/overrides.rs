//! Port of `src/settings/skills/overrides.ts`, switched.
//!
//! What a `skillOverrides` state does to one skill (skills.md, "Override skill visibility from
//! settings": the table of "Listed to Claude" and "In `/` menu"). `inClientCommandLists` is the
//! command list advertised to Remote Control clients and Agent SDK callers;
//! `invokableByFullName` is false when invoking the skill by its full name returns the
//! `skillOverrides` error (`blockedBy`) instead of running it.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::protocol::settings_keys::SKILL_OVERRIDE_STATES;
use crate::registry::Port;
use crate::settings::layers::{KeyHolder, PolicyScope};
use crate::settings::view::PolicyView;

/// What a state does to one skill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVisibility {
    /// `full`, `nameOnly` or `hidden`.
    pub listed_to_model: &'static str,
    /// In the `/` menu.
    pub in_slash_menu: bool,
    /// In the command lists of Remote Control clients and Agent SDK callers.
    pub in_client_command_lists: bool,
    /// Invoking it by its full name runs it.
    pub invokable_by_full_name: bool,
    /// `skillOverrides` when the call returns its error.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub blocked_by: Option<&'static str>,
}

/// What the host knows about the skill it asks for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct SkillSubject {
    /// A skill a plugin provides: "Overrides don't apply to plugin skills".
    #[serde(default)]
    pub plugin: Option<bool>,
    /// The other names of a bundled skill, such as `checkup` for `/doctor`.
    #[serde(default)]
    pub aliases: Option<Vec<String>>,
}

const SHOWN: SkillVisibility = SkillVisibility {
    listed_to_model: "full",
    in_slash_menu: true,
    in_client_command_lists: true,
    invokable_by_full_name: true,
    blocked_by: None,
};

/// One row per state, in the order the docs list them.
const VISIBILITY: [(&str, SkillVisibility); 4] = [
    ("on", SHOWN),
    (
        "name-only",
        SkillVisibility {
            listed_to_model: "nameOnly",
            ..SHOWN
        },
    ),
    (
        "user-invocable-only",
        SkillVisibility {
            listed_to_model: "hidden",
            ..SHOWN
        },
    ),
    // `off` hides the skill from the terminal `/` menu "and" from the Remote Control and SDK lists.
    (
        "off",
        SkillVisibility {
            listed_to_model: "hidden",
            in_slash_menu: false,
            in_client_command_lists: false,
            invokable_by_full_name: false,
            blocked_by: Some("skillOverrides"),
        },
    ),
];

/// Files whose entries under a bundled skill's alias apply (settings-reference.md,
/// `skillOverrides`).
const ALIAS_SCOPES: [PolicyScope; 2] = [PolicyScope::Managed, PolicyScope::Flag];

/// Position in the order the docs list the states: later hides more.
fn strictness(state: &str) -> Option<usize> {
    SKILL_OVERRIDE_STATES.iter().position(|one| *one == state)
}

fn strictest(states: Vec<String>) -> String {
    states.into_iter().fold("on".to_owned(), |a, b| {
        if strictness(&b) > strictness(&a) {
            b
        } else {
            a
        }
    })
}

fn entry_of(entries: &JsValue, name: &str) -> Option<String> {
    entries
        .get(name)
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

/// The state of a skill after the files combine. Per name the highest file that sets it decides
/// (judgment, spec/judgments.md). Entries under an alias count only from managed settings and
/// `--settings` files; they can only restrict further, and an entry under the skill's own name in
/// managed settings beats them (skills.md).
fn state_of(holders: &[KeyHolder], name: &str, aliases: &[String]) -> String {
    let first = holders
        .iter()
        .find_map(|holder| entry_of(&holder.value, name))
        .unwrap_or_else(|| "on".to_owned());
    let managed_own = holders.iter().any(|holder| {
        holder.scope == PolicyScope::Managed && entry_of(&holder.value, name).is_some()
    });
    if managed_own {
        return first;
    }
    let mut states = vec![first];
    states.extend(aliases.iter().filter_map(|alias| {
        holders
            .iter()
            .filter(|holder| ALIAS_SCOPES.contains(&holder.scope))
            .find_map(|holder| entry_of(&holder.value, alias))
    }));
    strictest(states)
}

/// What `skillOverrides` does to one skill; a plugin skill and a skill with no entry are `on`.
/// A state off the table has no row (`None`, the package's `undefined`).
pub async fn skill_visibility(
    view: &dyn PolicyView,
    name: &str,
    subject: &SkillSubject,
) -> Option<SkillVisibility> {
    let state = if subject.plugin == Some(true) {
        "on".to_owned()
    } else {
        let holders = view.each_from_highest("skillOverrides").await;
        state_of(
            &holders,
            name,
            subject.aliases.as_deref().unwrap_or_default(),
        )
    };
    VISIBILITY
        .iter()
        .find(|(one, _)| *one == state)
        .map(|(_, visibility)| visibility.clone())
}

crate::registry::exports! {
    "settings/skills/overrides" {
        "skillVisibility" => async ok |view: Port, name: String, subject: Option<SkillSubject>|
            skill_visibility(&view, &name, &subject.unwrap_or_default()).await;
    }
}
