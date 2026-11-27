//! Port of `src/agents/subagent-permission.ts`, switched.
//!
//! The permission mode a subagent runs in (sub-agents.md, "Permission modes"): "The main
//! conversation's permission mode decides whether Claude Code uses the value you set". Unset, the
//! subagent inherits the main conversation's mode. A value that is no mode counts as unset.

use serde::{Deserialize, Serialize};

use super::permission_mode::{GrantEnvironment, MainStance, permission_mode_traits};
use crate::protocol::permission_modes::normalize_permission_mode;

/// What decides the subagent's mode beyond its own `permissionMode`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPermissionInput {
    /// The main conversation's current mode.
    pub main: String,
    /// The subagent's `permissionMode` as text (a `PermissionModeInput`; "manual" is `default`);
    /// `None` when it sets none. Raw text from a host that is no mode counts as unset.
    #[serde(default)]
    pub set: Option<String>,
    /// Auto mode is available to the subagent: no `disableAutoMode`, a model that supports it.
    pub auto_available: bool,
}

/// How the mode came about: `inherited` (nothing set), `set` (its own applies), `main` (the main's held).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SubagentModeBasis {
    /// Nothing set.
    Inherited,
    /// Its own applies.
    Set,
    /// The main's held.
    Main,
}

/// The mode a subagent runs in, and how it came about.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPermission {
    /// The mode.
    pub mode: String,
    /// How it came about.
    pub basis: SubagentModeBasis,
    /// An action the mode would ask about is denied instead: `dontAsk`, and a subagent that
    /// "stays in plan mode" while it sets `dontAsk`.
    pub deny_instead_of_ask: bool,
}

fn outcome(mode: &str, basis: SubagentModeBasis, denies: bool) -> SubagentPermission {
    SubagentPermission {
        mode: mode.to_owned(),
        basis,
        deny_instead_of_ask: denies,
    }
}

fn denies(mode: Option<&str>) -> bool {
    mode.and_then(permission_mode_traits)
        .is_some_and(|traits| traits.denies_instead_of_asking)
}

const fn held(set: Option<&str>) -> SubagentModeBasis {
    if set.is_none() {
        SubagentModeBasis::Inherited
    } else {
        SubagentModeBasis::Main
    }
}

/// The three stances, each one of the bullets under "Permission modes" in sub-agents.md.
fn resolve(
    stance: MainStance,
    input: &SubagentPermissionInput,
    set: Option<&str>,
) -> SubagentPermission {
    let main = input.main.as_str();
    match stance {
        // "the subagent runs in that same mode and Claude Code ignores the `permissionMode` you set"
        MainStance::Forces => outcome(main, held(set), denies(Some(main))),
        // "stays in plan mode whichever `permissionMode` you set ... If you set `dontAsk`" it denies.
        MainStance::HoldsPlan => outcome(main, held(set), denies(Some(main)) || denies(set)),
        // "runs in the permission mode you set", except a mode it is not granted.
        MainStance::LetsSetApply => {
            let Some(set) = set else {
                return outcome(main, SubagentModeBasis::Inherited, denies(Some(main)));
            };
            let environment = GrantEnvironment {
                auto_available: input.auto_available,
            };
            let granted = permission_mode_traits(set)
                .is_some_and(|traits| traits.granted_when_set.granted(environment));
            if granted {
                outcome(set, SubagentModeBasis::Set, denies(Some(set)))
            } else {
                outcome(main, SubagentModeBasis::Main, denies(Some(main)))
            }
        }
    }
}

/// The permission mode a subagent runs in; `Err` for a main mode that is no mode, where the
/// TypeScript reads a row that is not there.
pub fn resolve_subagent_permission_mode(
    input: &SubagentPermissionInput,
) -> Result<SubagentPermission, String> {
    let set = input.set.as_deref().and_then(normalize_permission_mode);
    let traits = permission_mode_traits(&input.main)
        .ok_or_else(|| format!("{} is no permission mode", input.main))?;
    Ok(resolve(traits.when_main, input, set))
}

crate::registry::exports! {
    "agents/subagent-permission" {
        "resolveSubagentPermissionMode" => fallible |input: SubagentPermissionInput|
            resolve_subagent_permission_mode(&input);
    }
}
