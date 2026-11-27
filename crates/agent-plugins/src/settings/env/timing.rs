//! Port of `src/settings/env/timing.ts`, switched.

use indexmap::IndexMap;
use serde::Serialize;

use crate::js::in_key_order;
use crate::settings::env::resolve_env::{EnvResolution, NotAppliedVariable, SUBPROCESS_ONLY};
use crate::settings::key_spec::REPOSITORY_FILES;
use crate::settings::layers::PolicyScope;

/// When Claude Code applies `env` values again (settings-reference.md, "When Claude Code applies
/// `env` values"; env-vars.md, "Set variables in settings files"): in a running session when a
/// saved change alters the merged `env`, and after `/cd` with the new directory's project and local
/// values.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvChange {
    /// The new and the changed variables: what the running session writes into its environment.
    pub set: IndexMap<String, String>,
    /// The variables the new `env` no longer holds. "Removing a variable from the file doesn't
    /// unset it in a running session; the removal takes effect the next time you launch `claude`."
    pub kept_until_relaunch: Vec<String>,
}

/// What a saved change to the `env` of the files does to a running session.
#[must_use]
pub fn env_change(applied: &IndexMap<String, String>, next: IndexMap<String, String>) -> EnvChange {
    EnvChange {
        kept_until_relaunch: applied
            .keys()
            .filter(|name| !next.contains_key(*name))
            .cloned()
            .collect(),
        set: next
            .into_iter()
            .filter(|(name, value)| applied.get(name) != Some(value))
            .collect(),
    }
}

/// `{ ...base, ...more }`: `more`'s values over `base`'s, a name already there keeping its place.
fn spread<V: Clone>(base: &IndexMap<String, V>, more: &IndexMap<String, V>) -> IndexMap<String, V> {
    let mut out = base.clone();
    out.extend(
        more.iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    in_key_order(out)
}

/// An `EnvResolution` with its members in the order `envAfterCd` writes them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvAfterCd {
    /// What the files come to, by variable.
    pub settings: IndexMap<String, String>,
    /// The file each of `settings` came from.
    pub origin: IndexMap<String, PolicyScope>,
    /// The environment Claude Code's own process reads.
    pub process: IndexMap<String, String>,
    /// The environment every subprocess it starts inherits.
    pub subprocess: IndexMap<String, String>,
    /// The values left out, with the reason.
    pub not_applied: Vec<NotAppliedVariable>,
}

/// The session after `/cd` (permissions.md, "Move the session to another directory"; Claude Code
/// v2.1.246 or later): "the new directory's project and local `env` values, on top of the previous
/// directory's". `entered` is the resolution for the new directory's layers; its user, managed and
/// `--settings` values were applied at startup and stay as they are.
#[must_use]
pub fn env_after_cd(previous: &EnvResolution, entered: &EnvResolution) -> EnvAfterCd {
    let from_repository = |name: &str| {
        entered
            .origin
            .get(name)
            .is_some_and(|scope| REPOSITORY_FILES.contains(scope))
    };
    let added: IndexMap<String, String> = entered
        .settings
        .iter()
        .filter(|(name, _)| from_repository(name))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let own: IndexMap<String, String> = added
        .iter()
        .filter(|(name, _)| !SUBPROCESS_ONLY.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    let origins: IndexMap<String, PolicyScope> = added
        .keys()
        .map(|name| {
            (
                name.clone(),
                entered
                    .origin
                    .get(name)
                    .copied()
                    .unwrap_or(PolicyScope::Project),
            )
        })
        .collect();
    EnvAfterCd {
        settings: spread(&previous.settings, &added),
        origin: spread(&previous.origin, &origins),
        process: spread(&previous.process, &own),
        subprocess: spread(&previous.subprocess, &added),
        not_applied: [&previous.not_applied[..], &entered.not_applied[..]].concat(),
    }
}

crate::registry::exports! {
    "settings/env/timing" {
        "envChange" => ok |applied: IndexMap<String, String>, next: IndexMap<String, String>|
            env_change(&applied, next);
        "envAfterCd" => ok |previous: EnvResolution, entered: EnvResolution|
            env_after_cd(&previous, &entered);
    }
}
