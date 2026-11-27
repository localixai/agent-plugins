//! Port of `src/settings/layers.ts`, switched.
//!
//! Where a settings object comes from, the layers in precedence order, and the layers that set a
//! key, read through each key's row (`key_spec.rs`).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::settings::key_spec::{SPECS, admits};

/// Where a settings object comes from, highest precedence first (settings.md, "Settings
/// precedence": managed, command line, project local, shared project, user). `flag` is the
/// `--settings` JSON. `parent` is the policy-tier settings an embedding host passes through the
/// Agent SDK's `managedSettings`; `addDir` is a `--add-dir` directory's `.claude/settings*.json`,
/// which carries two keys only. The docs rank `parent` nowhere (spec/judgments.md); they rank
/// `addDir` lowest of the sources of `enabledPlugins` (plugins/loading.md, "Find where a plugin is
/// enabled").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PolicyScope {
    /// Managed settings.
    Managed,
    /// An embedding host's policy-tier settings.
    Parent,
    /// The `--settings` JSON.
    Flag,
    /// The project's local file.
    Local,
    /// The project's shared file.
    Project,
    /// The user's file.
    User,
    /// A `--add-dir` directory's file.
    AddDir,
}

/// The scopes, highest precedence first.
pub const POLICY_SCOPES: [PolicyScope; 7] = [
    PolicyScope::Managed,
    PolicyScope::Parent,
    PolicyScope::Flag,
    PolicyScope::Local,
    PolicyScope::Project,
    PolicyScope::User,
    PolicyScope::AddDir,
];

impl PolicyScope {
    /// The scope's name as the package writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Managed => "managed",
            Self::Parent => "parent",
            Self::Flag => "flag",
            Self::Local => "local",
            Self::Project => "project",
            Self::User => "user",
            Self::AddDir => "addDir",
        }
    }
}

/// A higher number wins.
pub const SCOPE_RANK: [(PolicyScope, u8); 7] = [
    (PolicyScope::Managed, 7),
    (PolicyScope::Parent, 6),
    (PolicyScope::Flag, 5),
    (PolicyScope::Local, 4),
    (PolicyScope::Project, 3),
    (PolicyScope::User, 2),
    (PolicyScope::AddDir, 1),
];

/// `SCOPE_RANK[scope]`.
#[must_use]
pub fn rank_of(scope: PolicyScope) -> u8 {
    SCOPE_RANK
        .iter()
        .find(|(one, _)| *one == scope)
        .map_or(0, |(_, rank)| *rank)
}

/// One settings source with the scope it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PolicyLayer {
    /// Where it comes from.
    pub scope: PolicyScope,
    /// The settings object.
    pub settings: JsValue,
    /// The keys this source carried with a value Claude Code could not read and no value can
    /// stand for: a repair removed them from `settings`, and the restrictive effect the docs give
    /// them is read from here (managed-settings.md, "Keys that fail closed"; settings-reference.md,
    /// `crossSessionInbound`). Absent when every key was readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalid: Option<Vec<String>>,
}

/// What a layer is ranked by: its scope. The package sorts the host's own objects, whatever other
/// members they carry, so the export sorts the values as given; a value whose scope is no scope
/// ranks below every scope (the package's comparison is `NaN` there, which sorts it nowhere).
pub trait Ranked {
    /// The layer's scope.
    fn scope_of(&self) -> Option<PolicyScope>;
}

impl Ranked for PolicyLayer {
    fn scope_of(&self) -> Option<PolicyScope> {
        Some(self.scope)
    }
}

impl Ranked for JsValue {
    fn scope_of(&self) -> Option<PolicyScope> {
        let scope = self.get("scope")?.as_str()?;
        POLICY_SCOPES.into_iter().find(|one| one.name() == scope)
    }
}

/// The layers, highest precedence first; two layers of one scope keep the order they were given.
#[must_use]
pub fn ordered_layers<L: Ranked + Clone>(layers: &[L]) -> Vec<L> {
    let mut ordered = layers.to_vec();
    ordered.sort_by_key(|layer| std::cmp::Reverse(layer.scope_of().map_or(0, rank_of)));
    ordered
}

/// One layer's value of a key, with the layer it came from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct KeyHolder {
    /// The layer's scope.
    pub scope: PolicyScope,
    /// Position among the ordered layers: two files of one scope have two positions.
    pub position: usize,
    /// The key's value in that layer.
    pub value: JsValue,
}

/// The key's value in one settings object: where its row reads it (a key inside a block), else
/// the key's own name.
fn value_in(settings: &JsValue, key: &str) -> JsValue {
    match SPECS.get(key).and_then(|row| row.read.as_ref()) {
        Some(read) => read(settings),
        None => settings.get(key).cloned().unwrap_or_default(),
    }
}

/// Every layer of `ordered` (highest precedence first, as `orderedLayers` gives them) that sets
/// `key` to a value its scope admits. A layer that does not carry the key (the key's scope line, or
/// the `--add-dir` rule) is skipped even when the layer was built by hand rather than read.
#[must_use]
pub fn holders_of(ordered: &[PolicyLayer], key: &str) -> Vec<KeyHolder> {
    ordered
        .iter()
        .enumerate()
        .filter_map(|(position, layer)| {
            let value = value_in(&layer.settings, key);
            if matches!(value, JsValue::Undefined | JsValue::Null)
                || !admits(key, layer.scope, &value)
            {
                return None;
            }
            Some(KeyHolder {
                scope: layer.scope,
                position,
                value,
            })
        })
        .collect()
}

crate::registry::exports! {
    "settings/layers" {
        "POLICY_SCOPES" => ok || POLICY_SCOPES;
        "SCOPE_RANK" => ok || SCOPE_RANK
            .iter()
            .map(|(scope, rank)| (scope.name(), *rank))
            .collect::<IndexMap<_, _>>();
        "orderedLayers" => ok |layers: Vec<JsValue>| ordered_layers(&layers);
        "holdersOf" => ok |ordered: Vec<PolicyLayer>, key: String| holders_of(&ordered, &key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    #[test]
    fn a_nested_key_is_read_where_its_row_reads_it() {
        let layers = json_parse(
            r#"[{"scope":"user","settings":{"permissions":{"defaultMode":"plan"}}},
                {"scope":"project","settings":{"permissions":{"defaultMode":"auto"}}}]"#,
        )
        .unwrap_or_default();
        let layers: Vec<PolicyLayer> = layers
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|layer| PolicyLayer::deserialize(layer.clone()).ok())
            .collect();
        let ordered = ordered_layers(&layers);
        let holders = holders_of(&ordered, "permissions.defaultMode");
        assert_eq!(holders.len(), 1);
        assert_eq!(
            holders.first().map(|one| one.scope),
            Some(PolicyScope::User)
        );
    }
}
