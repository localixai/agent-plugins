//! Port of `src/settings/merge.ts`, switched.
//!
//! How the layers that set one key combine into its value. A strategy is typed over the key's own
//! value type `V` in the package: `combine` takes the key's value in every layer that sets it,
//! highest precedence first, and returns a `V`. So a row of `KEY_SPECS` cannot name a merge that
//! returns another shape, and the policy view needs no cast. One strategy per kind, never a branch.
//!
//! `MERGES` holds functions, which cannot cross the boundary: its export takes the kind first and
//! the values second, and the wrapper's table calls it with each row's kind.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue, in_property_order, property_order};
use crate::protocol::is_record::is_record;
use crate::settings::canonical::union_of;
use crate::settings::general::exceptions::{TRUE_FROM_ANY_SCOPE, strictest_of};
use crate::settings::hooks::merge_groups::{is_hooks_settings, merge_hook_settings};
use crate::settings::permissions::merge::{merge_auto_mode_blocks, merge_permission_blocks};

/// The merge strategies, by name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MergeKind {
    /// The highest layer's value.
    HighestWins,
    /// Every layer's list entries, once each.
    UnionArrays,
    /// Per name, the highest layer's entry.
    PerKeyHighestWins,
    /// The `hooks` groups.
    HookGroups,
    /// Blocks key by key, lists united, the rest the highest layer's.
    DeepMerge,
    /// The `permissions` blocks.
    PermissionBlocks,
    /// The `autoMode` blocks.
    AutoModeBlocks,
    /// A Boolean on when any layer sets `true`.
    AnyTrue,
}

/// A strategy: its kind, and how it combines the values of the layers that set the key.
#[derive(Clone, Copy, Debug)]
pub struct Merge {
    /// Its name.
    pub kind: MergeKind,
    /// The values, highest precedence first, as one value (`undefined` for none).
    pub combine: fn(&[JsValue]) -> JsValue,
}

/// settings.md, "Settings precedence": the value from the highest level that sets the key.
const HIGHEST_WINS: Merge = Merge {
    kind: MergeKind::HighestWins,
    combine: |values| values.first().cloned().unwrap_or_default(),
};

/// settings.md, "Lists merge instead of overriding": every file adds its entries, none removes.
const UNION_ARRAYS: Merge = Merge {
    kind: MergeKind::UnionArrays,
    combine: |values| {
        let lists: Vec<Vec<JsValue>> = values
            .iter()
            .filter_map(JsValue::as_array)
            .cloned()
            .collect();
        JsValue::Array(union_of(&lists))
    },
};

/// An object keyed by name: per name the highest level that sets it wins, whole.
const PER_KEY_HIGHEST_WINS: Merge = Merge {
    kind: MergeKind::PerKeyHighestWins,
    combine: |values| {
        let mut merged: JsObject = IndexMap::new();
        for value in values.iter().rev() {
            if is_record(value)
                && let Some(members) = value.as_object()
            {
                for (name, entry) in property_order(members) {
                    merged.insert(name.clone(), entry.clone());
                }
            }
        }
        // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` name stays a name.
        JsValue::Object(in_property_order(merged))
    },
};

const HOOK_GROUPS: Merge = Merge {
    kind: MergeKind::HookGroups,
    combine: |values| {
        let files: Vec<JsObject> = values
            .iter()
            .filter(|value| is_hooks_settings(value))
            .filter_map(JsValue::as_object)
            .cloned()
            .collect();
        JsValue::Object(merge_hook_settings(&files))
    },
};

/// A value of one file over the same key of a lower one: blocks merge, lists union, the rest replaces.
fn over_value(below: &JsValue, above: &JsValue) -> JsValue {
    match (below, above) {
        (JsValue::Object(lower), JsValue::Object(higher)) => {
            JsValue::Object(over_block(lower, higher))
        }
        (JsValue::Array(lower), JsValue::Array(higher)) => {
            JsValue::Array(union_of(&[higher.clone(), lower.clone()]))
        }
        _ => above.clone(),
    }
}

fn over_block(below: &JsObject, above: &JsObject) -> JsObject {
    let mut merged: JsObject = property_order(below)
        .into_iter()
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    for (name, value) in property_order(above) {
        if value.is_undefined() {
            continue;
        }
        let next = match merged.get(name) {
            Some(current) => over_value(current, value),
            None => value.clone(),
        };
        merged.insert(name.clone(), next);
    }
    // `Object.fromEntries` defines own properties, so a JSON-parsed `__proto__` name stays a name.
    in_property_order(merged)
}

/// settings.md, "Settings precedence" and "Lists merge instead of overriding", managed-settings.md,
/// "Split a file-based policy across teams": a nested block merges key by key and each key follows
/// the same rules; a list combines with the duplicates removed (the highest file's entries first);
/// any other value comes whole from the highest file that sets it.
const DEEP_MERGE: Merge = Merge {
    kind: MergeKind::DeepMerge,
    combine: |values| {
        values
            .iter()
            .rev()
            .fold(JsValue::Undefined, |below, above| over_value(&below, above))
    },
};

/// The `permissions` object: lists unite, locks and block flags hold when any file sets them (permissions/merge.ts).
const PERMISSION_BLOCKS: Merge = Merge {
    kind: MergeKind::PermissionBlocks,
    combine: merge_permission_blocks,
};

const AUTO_MODE_BLOCKS: Merge = Merge {
    kind: MergeKind::AutoModeBlocks,
    combine: merge_auto_mode_blocks,
};

/// A Boolean on when any file sets `true`, else the highest file's value: the ladder step of
/// general/exceptions.ts as a merge, for keys outside the table of managed exceptions
/// (`skipDangerousModePermissionPrompt`, `skipAutoPermissionPrompt`, `allowAllClaudeAiMcps`).
const ANY_TRUE: Merge = Merge {
    kind: MergeKind::AnyTrue,
    combine: |values| {
        let strictest = strictest_of(TRUE_FROM_ANY_SCOPE.ladder.clone(), values);
        // `??`: a missing or `null` strictest value gives way to the highest file's.
        if matches!(strictest, JsValue::Undefined | JsValue::Null) {
            values.first().cloned().unwrap_or_default()
        } else {
            strictest
        }
    },
};

/// The strategies, one member per kind.
#[derive(Clone, Copy, Debug)]
pub struct Merges {
    /// `highestWins`.
    pub highest_wins: Merge,
    /// `unionArrays`.
    pub union_arrays: Merge,
    /// `perKeyHighestWins`.
    pub per_key_highest_wins: Merge,
    /// `hookGroups`.
    pub hook_groups: Merge,
    /// `deepMerge`.
    pub deep_merge: Merge,
    /// `permissionBlocks`.
    pub permission_blocks: Merge,
    /// `autoModeBlocks`.
    pub auto_mode_blocks: Merge,
    /// `anyTrue`.
    pub any_true: Merge,
}

/// Every strategy.
pub const MERGES: Merges = Merges {
    highest_wins: HIGHEST_WINS,
    union_arrays: UNION_ARRAYS,
    per_key_highest_wins: PER_KEY_HIGHEST_WINS,
    hook_groups: HOOK_GROUPS,
    deep_merge: DEEP_MERGE,
    permission_blocks: PERMISSION_BLOCKS,
    auto_mode_blocks: AUTO_MODE_BLOCKS,
    any_true: ANY_TRUE,
};

impl Merges {
    /// The strategy of `kind`: `MERGES[kind]`.
    #[must_use]
    pub const fn of(&self, kind: MergeKind) -> &Merge {
        match kind {
            MergeKind::HighestWins => &self.highest_wins,
            MergeKind::UnionArrays => &self.union_arrays,
            MergeKind::PerKeyHighestWins => &self.per_key_highest_wins,
            MergeKind::HookGroups => &self.hook_groups,
            MergeKind::DeepMerge => &self.deep_merge,
            MergeKind::PermissionBlocks => &self.permission_blocks,
            MergeKind::AutoModeBlocks => &self.auto_mode_blocks,
            MergeKind::AnyTrue => &self.any_true,
        }
    }
}

crate::registry::exports! {
    "settings/merge" {
        "MERGES" => ok |kind: MergeKind, values: Vec<JsValue>| (MERGES.of(kind).combine)(&values);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    fn combined(merge: &Merge, values: &str) -> String {
        let values = json_parse(values).unwrap_or_default();
        (merge.combine)(values.as_array().map_or(&[][..], Vec::as_slice)).to_json_text()
    }

    #[test]
    fn a_deep_merge_unites_lists_and_merges_blocks_key_by_key() {
        let values = r#"[{"a":{"x":1,"l":["p"]},"b":2},{"a":{"y":2,"l":["q","p"]},"b":3,"c":4}]"#;
        assert_eq!(
            combined(&MERGES.deep_merge, values),
            r#"{"a":{"y":2,"l":["p","q"],"x":1},"b":2,"c":4}"#
        );
    }

    #[test]
    fn per_key_highest_wins_keeps_the_lowest_layer_s_order() {
        let values = r#"[{"b":1,"2":"x"},{"a":0,"b":0,"1":"y"}]"#;
        assert_eq!(
            combined(&MERGES.per_key_highest_wins, values),
            r#"{"1":"y","2":"x","a":0,"b":1}"#
        );
    }

    #[test]
    fn any_true_is_on_when_one_layer_sets_true() {
        assert_eq!(combined(&MERGES.any_true, "[false,true]"), "true");
        assert_eq!(combined(&MERGES.any_true, "[false,false]"), "false");
        assert_eq!(
            combined(&MERGES.highest_wins, "[]"),
            r#"{"$undefined":true}"#
        );
    }
}
