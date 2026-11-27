//! Ported from `src/settings/mcp/entry-env.ts`; only the core uses it now, so it has no wrapper.
//!
//! The environment a policy entry expands in (managed-mcp.md, "Environment variables in
//! `serverCommand` and `serverUrl` entries"): a pinned one, "so a variable set by a project or user
//! settings file can't change what an allowlist entry means". Both lists read "the environment
//! Claude Code started with, plus `env` values from managed settings"; the denylist also fills "a
//! variable with no startup value and no `:-default`" from "settings files outside the repository,
//! such as user or managed settings, which only ever widens what the entry matches".
//!
//! Judgments (spec/judgments.md): the startup environment wins over a managed `env` value;
//! "outside the repository" is the managed, parent, `--settings` and user layers (project, local
//! and `--add-dir` files live in the folder), the highest layer winning per variable.

use indexmap::IndexMap;

use crate::format::mcp_env::{
    braced_references, is_variable_name, resolve_env_reference, split_default,
};
use crate::js::JsValue;
use crate::js::value::JsObject;
use crate::ports::{BoxFuture, now};
use crate::settings::layers::PolicyLayer;
use crate::settings::mcp::command_match::ExpandEntry;
use crate::settings::view::PolicyView;

/// An environment as the policy reads it (`PolicyEnv`): a name to its value.
pub type PolicyEnv = IndexMap<String, String>;

/// The host's `PolicyEnv`: the variables whose value is a string (an unset one reads as unset).
#[must_use]
pub fn policy_env_of(env: &JsObject) -> PolicyEnv {
    env.iter()
        .filter_map(|(name, value)| Some((name.clone(), value.as_str()?.to_owned())))
        .collect()
}

/// A lookup that reads own entries only, so `constructor` is a variable name and not a function.
#[must_use]
pub fn lookup_in(env: &PolicyEnv, name: &str) -> Option<String> {
    env.get(name).cloned()
}

/// The `env` block of the layers of these scopes, each variable from the highest layer that sets
/// it.
fn settings_env(layers: &[PolicyLayer], scopes: &[&str]) -> PolicyEnv {
    let mut merged = PolicyEnv::new();
    for layer in layers.iter().rev() {
        let Some(JsValue::Object(block)) = layer.settings.get("env") else {
            continue;
        };
        if !scopes.contains(&layer.scope.name()) {
            continue;
        }
        for (name, value) in crate::js::property_order(block) {
            if let Some(text) = value.as_str() {
                merged.insert(name.clone(), text.to_owned());
            }
        }
    }
    merged
}

const OUTSIDE_THE_REPOSITORY: [&str; 4] = ["managed", "parent", "flag", "user"];

/// How an entry's `${VAR}` text becomes the text it is matched as.
#[derive(Clone, Debug)]
pub struct EntryExpansion {
    started: PolicyEnv,
    managed: PolicyEnv,
    /// The lookups a variable unset in the pinned one without a default is tried in.
    fill: Vec<PolicyEnv>,
}

impl EntryExpansion {
    /// The startup environment, then managed `env` values.
    fn pinned(&self, name: &str) -> Option<String> {
        lookup_in(&self.started, name).or_else(|| lookup_in(&self.managed, name))
    }

    /// Expands one pass; a variable unset in the pinned lookup without a default is tried in each
    /// `fill`, then kept as written.
    #[must_use]
    pub fn expand(&self, text: &str) -> String {
        let pinned = |name: &str| self.pinned(name);
        let mut expanded = String::new();
        let mut last = 0;
        for (range, inner) in braced_references(text) {
            expanded.push_str(text.get(last..range.start).unwrap_or_default());
            let raw = text.get(range.clone()).unwrap_or_default();
            last = range.end;
            let reference = split_default(inner);
            if !is_variable_name(&reference.name) {
                expanded.push_str(raw);
                continue;
            }
            let resolved = now(resolve_env_reference(&reference, &pinned, None));
            match resolved {
                Some(resolved) if !resolved.missing => expanded.push_str(&resolved.text),
                _ => {
                    let filled = self
                        .fill
                        .iter()
                        .find_map(|each| lookup_in(each, &reference.name));
                    expanded.push_str(filled.as_deref().unwrap_or(raw));
                }
            }
        }
        expanded.push_str(text.get(last..).unwrap_or_default());
        expanded
    }
}

impl ExpandEntry for EntryExpansion {
    fn expand<'a>(&'a self, text: &'a str) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(Self::expand(self, text)))
    }
}

/// The expansion of each list's entries.
#[derive(Clone, Debug)]
pub struct EntryExpansions {
    /// `allowedMcpServers`.
    pub allowed: EntryExpansion,
    /// `deniedMcpServers`.
    pub denied: EntryExpansion,
}

/// The expansion of each list's entries over the layers and the environment Claude Code started
/// with.
pub async fn entry_expansions(view: &dyn PolicyView, startup: &PolicyEnv) -> EntryExpansions {
    let layers = view.layers().await;
    let managed = settings_env(&layers, &["managed"]);
    let outside = settings_env(&layers, &OUTSIDE_THE_REPOSITORY);
    let pinned = |fill| EntryExpansion {
        started: startup.clone(),
        managed: managed.clone(),
        fill,
    };
    EntryExpansions {
        allowed: pinned(Vec::new()),
        denied: pinned(vec![outside]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_denylist_fills_an_unset_variable_from_outside_the_repository() {
        let expansion = EntryExpansion {
            started: PolicyEnv::from([("A".to_owned(), "1".to_owned())]),
            managed: PolicyEnv::new(),
            fill: vec![PolicyEnv::from([("B".to_owned(), "2".to_owned())])],
        };
        assert_eq!(
            expansion.expand("${A}-${B}-${C}-${C:-d}-${1x}"),
            "1-2-${C}-d-${1x}"
        );
    }
}
