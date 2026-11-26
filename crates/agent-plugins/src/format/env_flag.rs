//! Port of `src/format/env-flag.ts`, switched.
//! An on/off environment variable (env-vars.md, "Variables"): "set `1`, `true`, `yes`, or `on` to
//! turn it on and `0`, `false`, `no`, or `off` to turn it off, in any casing." Surrounding blanks
//! are ignored (judgment, spec/judgments.md). A variable read only for being set at all ("any
//! non-empty value including `0`") is not one of these: its reader tests for a non-empty value.

use crate::js::trim;

fn spelling(value: Option<&str>) -> String {
    trim(value.unwrap_or_default()).to_lowercase()
}

/// Whether the value turns the switch on; anything but the four on spellings, unset included,
/// does not.
#[must_use]
pub fn is_truthy_env(value: Option<&str>) -> bool {
    matches!(spelling(value).as_str(), "1" | "true" | "yes" | "on")
}

/// Whether the value turns the switch off by one of the four off spellings (unset and empty are
/// neither).
#[must_use]
pub fn is_falsy_env(value: Option<&str>) -> bool {
    matches!(spelling(value).as_str(), "0" | "false" | "no" | "off")
}

crate::registry::exports! {
    "format/env-flag" {
        "isTruthyEnv" => ok |value: Option<String>| is_truthy_env(value.as_deref());
        "isFalsyEnv" => ok |value: Option<String>| is_falsy_env(value.as_deref());
    }
}
