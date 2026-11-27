//! Port of `src/settings/general/exceptions.ts`, switched.
//!
//! A key whose managed value is not an override (settings.md, "Exceptions to managed settings
//! precedence"): a restrictive value from a lower scope stays in force. The rule is the same for
//! every key and only its data differs, so a key's row names a `ManagedException` and one function
//! applies it.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::js::{JsValue, strict_equals};
use crate::protocol::effort::EFFORT_LEVELS;
use crate::settings::layers::PolicyScope;

/// What a value no rung can stand for does when a source of `from` carries it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Unreadable {
    /// What it counts as.
    pub floor: JsValue,
    /// The scopes it counts from.
    pub from: Vec<PolicyScope>,
}

/// A key's exception to precedence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManagedException {
    /// The values the key reads, from the least restrictive to the most; a value off the ladder is not honored.
    pub ladder: Vec<JsValue>,
    /// The scopes whose more restrictive value beats the value that wins by precedence.
    pub honored_from: Vec<PolicyScope>,
    /// The scopes whose value never takes part in precedence and counts only when it is more
    /// restrictive than the value that does (a repository's own files: the docs say a project or local
    /// value "that isn't stricter is ignored").
    pub only_when_stricter: Vec<PolicyScope>,
    /// What a value of this key that no rung can stand for does when a source of `from` carries it: it
    /// counts as `floor` whatever precedence says, so a higher source cannot undo it
    /// (settings-reference.md, `crossSessionInbound`: an unrecognized value holds messages "even when a
    /// source that takes precedence sets `accept`").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreadable: Option<Unreadable>,
}

/// The scopes a key set in "Any file" may come from.
const ANY_SCOPE: [PolicyScope; 6] = [
    PolicyScope::Managed,
    PolicyScope::Parent,
    PolicyScope::Flag,
    PolicyScope::Local,
    PolicyScope::Project,
    PolicyScope::User,
];

const NO_SCOPE: [PolicyScope; 0] = [];

fn booleans(ladder: [bool; 2], honored_from: &[PolicyScope]) -> ManagedException {
    ManagedException {
        ladder: ladder.into_iter().map(JsValue::Bool).collect(),
        honored_from: honored_from.to_vec(),
        only_when_stricter: NO_SCOPE.to_vec(),
        unreadable: None,
    }
}

/// `true` restricts and any scope's `true` is honored: `disableClaudeAiConnectors`, `isolatePeerMachines`, the two `block…Outside…` keys.
pub static TRUE_FROM_ANY_SCOPE: LazyLock<ManagedException> =
    LazyLock::new(|| booleans([false, true], &ANY_SCOPE));

/// `true` restricts and only `~/.claude/settings.json` or `--settings` may beat a managed `false`:
/// `autoMode.classifyAllShell` and `permissions.denyInsteadOfPrompting`.
pub static TRUE_FROM_USER_OR_FLAG: LazyLock<ManagedException> =
    LazyLock::new(|| booleans([false, true], &[PolicyScope::User, PolicyScope::Flag]));

/// `false` restricts and the scopes the key's own line admits may set it: the claude.ai sync keys, `useAutoModeDuringPlan`, `enableArtifact`.
pub static FALSE_FROM_ANY_SCOPE: LazyLock<ManagedException> =
    LazyLock::new(|| booleans([true, false], &ANY_SCOPE));

/// `maxEffortLevel`: "When several scopes set a cap, the lowest applies"; `"max"` sets no cap.
pub static LOWEST_CAP: LazyLock<ManagedException> = LazyLock::new(|| ManagedException {
    ladder: EFFORT_LEVELS
        .iter()
        .rev()
        .map(|level| JsValue::from(*level))
        .collect(),
    honored_from: ANY_SCOPE.to_vec(),
    only_when_stricter: NO_SCOPE.to_vec(),
    unreadable: None,
});

/// `ladder.indexOf(value)`: its place, or -1 off the ladder.
fn index_of(ladder: &[JsValue], value: &JsValue) -> i64 {
    ladder
        .iter()
        .position(|rung| strict_equals(rung, value))
        .and_then(|place| i64::try_from(place).ok())
        .unwrap_or(-1)
}

/// Whether `a` restricts more than `b` on the ladder; `undefined` stands for no value, as unrestricted as the first rung.
fn restricts_more(ladder: &[JsValue], a: &JsValue, b: &JsValue) -> bool {
    index_of(ladder, a)
        > if matches!(b, JsValue::Undefined) {
            0
        } else {
            index_of(ladder, b)
        }
}

/// The value in force: the one precedence picks (`winning`, `undefined` when nothing applies), or the
/// most restrictive of the honored values when one restricts more.
#[must_use]
pub fn honor_exception(rule: &ManagedException, winning: JsValue, honored: &[JsValue]) -> JsValue {
    honored.iter().fold(winning, |current, value| {
        if restricts_more(&rule.ladder, value, &current) {
            value.clone()
        } else {
            current
        }
    })
}

/// The most restrictive of `values` on the ladder, `undefined` when none restricts more than the
/// first rung: the one "strict wins" step, for the view's rows and for the admin sources alike.
#[must_use]
pub fn strictest_of(ladder: Vec<JsValue>, values: &[JsValue]) -> JsValue {
    let rule = ManagedException {
        ladder,
        honored_from: ANY_SCOPE.to_vec(),
        only_when_stricter: NO_SCOPE.to_vec(),
        unreadable: None,
    };
    honor_exception(&rule, JsValue::Undefined, values)
}

crate::registry::exports! {
    "settings/general/exceptions" {
        "LOWEST_CAP" => ok || LOWEST_CAP.clone();
        "strictestOf" => ok |ladder: Vec<JsValue>, values: Vec<JsValue>| strictest_of(ladder, &values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lowest_cap_wins_and_an_unknown_value_is_not_honored() {
        let values = [
            JsValue::from("high"),
            JsValue::from("low"),
            JsValue::from("huge"),
        ];
        assert_eq!(
            strictest_of(LOWEST_CAP.ladder.clone(), &values),
            JsValue::from("low")
        );
        let none = [JsValue::from("max")];
        assert_eq!(
            strictest_of(LOWEST_CAP.ladder.clone(), &none),
            JsValue::Undefined
        );
    }
}
