//! Port of `src/agents/permission-mode.ts`, switched.
//!
//! The values `permissionMode` accepts in a subagent file, and what the docs say of each mode
//! (sub-agents.md, "Frontmatter reference" and "Permission modes").

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::protocol::permission_modes::PERMISSION_MODES;
use crate::registry::Function;

/// The values `permissionMode` accepts in a subagent file: the six modes' config values "and
/// `manual` as an alias for `default`" (sub-agents.md, "Frontmatter reference").
#[must_use]
pub fn permission_mode_inputs() -> Vec<&'static str> {
    let mut inputs = PERMISSION_MODES.to_vec();
    inputs.push("manual");
    inputs
}

/// What a subagent does when the MAIN conversation is in a mode (sub-agents.md, "Permission
/// modes"): `forces` the subagent runs in that same mode and ignores its own `permissionMode`;
/// `holds-plan` it stays in plan mode whichever mode it sets; `lets-set-apply` it runs in the mode
/// it sets, with two exceptions (`grantedWhenSet`).
pub const MAIN_STANCES: [MainStance; 3] = [
    MainStance::Forces,
    MainStance::HoldsPlan,
    MainStance::LetsSetApply,
];

/// One of `MAIN_STANCES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MainStance {
    /// The subagent runs in the main conversation's mode.
    Forces,
    /// The subagent stays in plan mode.
    HoldsPlan,
    /// The subagent runs in the mode it sets.
    LetsSetApply,
}

/// What decides whether a subagent may be set to a mode when the main conversation allows it.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GrantEnvironment {
    /// Auto mode is available to the subagent (no `disableAutoMode`, a model that supports it).
    pub auto_available: bool,
}

/// Whether a subagent may be set to a mode: always, never, or while auto mode is available.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grant {
    /// `always`.
    Always,
    /// `never`.
    Never,
    /// `(environment) => environment.autoAvailable`.
    WhenAutoAvailable,
}

impl Grant {
    /// Whether the mode is granted in `environment`.
    #[must_use]
    pub const fn granted(self, environment: GrantEnvironment) -> bool {
        match self {
            Self::Always => true,
            Self::Never => false,
            Self::WhenAutoAvailable => environment.auto_available,
        }
    }
}

/// What the docs say of a mode, apart from its prompting table row (`baselineDecision`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PermissionModeTraits {
    /// permission-modes.md, `dontAsk`: "auto-denies every tool call that would otherwise prompt".
    pub denies_instead_of_asking: bool,
    /// How a subagent follows the main conversation while the main conversation is in this mode.
    pub when_main: MainStance,
    /// Whether a subagent may be set to this mode while the main conversation is `default` or
    /// `dontAsk`: `bypassPermissions` "only when the main conversation does" (from v2.1.267) and
    /// `auto` only while auto mode is available to it.
    pub granted_when_set: Grant,
}

const fn traits(denies: bool, when_main: MainStance, grant: Grant) -> PermissionModeTraits {
    PermissionModeTraits {
        denies_instead_of_asking: denies,
        when_main,
        granted_when_set: grant,
    }
}

/// The registry, a row per mode in the TypeScript's order; `None` for a text that is no mode.
#[must_use]
pub fn permission_mode_traits(mode: &str) -> Option<PermissionModeTraits> {
    match mode {
        "default" => Some(traits(false, MainStance::LetsSetApply, Grant::Always)),
        "acceptEdits" => Some(traits(false, MainStance::Forces, Grant::Always)),
        "auto" => Some(traits(false, MainStance::Forces, Grant::WhenAutoAvailable)),
        "dontAsk" => Some(traits(true, MainStance::LetsSetApply, Grant::Always)),
        "bypassPermissions" => Some(traits(false, MainStance::Forces, Grant::Never)),
        "plan" => Some(traits(false, MainStance::HoldsPlan, Grant::Always)),
        _ => None,
    }
}

/// The modes in the order the registry lists them.
const TRAIT_ORDER: [&str; 6] = [
    "default",
    "acceptEdits",
    "auto",
    "dontAsk",
    "bypassPermissions",
    "plan",
];

/// `grantedWhenSet` as the host's function.
#[derive(Debug)]
struct GrantedWhenSet(Grant);

crate::registry::methods! {
    GrantedWhenSet {
        "call" => ok |this, environment: GrantEnvironment| this.0.granted(environment);
    }
}

/// A row of the registry as the host holds it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TraitsRow {
    denies_instead_of_asking: bool,
    when_main: MainStance,
    granted_when_set: Function,
}

fn traits_table() -> IndexMap<&'static str, TraitsRow> {
    TRAIT_ORDER
        .into_iter()
        .filter_map(|mode| {
            permission_mode_traits(mode).map(|row| {
                (
                    mode,
                    TraitsRow {
                        denies_instead_of_asking: row.denies_instead_of_asking,
                        when_main: row.when_main,
                        granted_when_set: Function::new(GrantedWhenSet(row.granted_when_set)),
                    },
                )
            })
        })
        .collect()
}

crate::registry::exports! {
    "agents/permission-mode" {
        "PERMISSION_MODE_INPUTS" => ok permission_mode_inputs;
        "MAIN_STANCES" => ok || MAIN_STANCES;
        "PERMISSION_MODE_TRAITS" => ok traits_table;
    }
}
