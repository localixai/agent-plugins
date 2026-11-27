//! Ported from `src/host/plugin-set.ts`; only the core uses it now, so it has no wrapper.
//!
//! The plugins of one host generation or one session: which are given, under which id, and which of
//! those that share a manifest name runs (plugins/loading.md, "Find where a plugin came from", "Name
//! conflicts").

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer};

use super::core_view::GivenPlugin;
use crate::format::issues::{PluginIssue, error, warning};
use crate::js::JsValue;
use crate::marketplace::name_conflicts::{NameCandidate, PluginOrigin, resolve_name_conflict};
use crate::registry::Own;
use crate::settings::layers::PolicyScope;
use crate::settings::plugins::enabled_plugins::{DecidedBy, enabled_plugin};
use crate::settings::plugins::plugin_id::parse_plugin_id;
use crate::settings::view::PolicyView;

/// One plugin the host or a session gave (`HostPlugin`), with the core's copy of its `Plugin`.
#[derive(Clone, Debug)]
pub struct HostPlugin {
    /// The `name@marketplace` id.
    pub id: String,
    /// The core's copy of the plugin.
    pub plugin: Arc<GivenPlugin>,
    /// Whether the host runs it, when the host decides.
    pub enabled: Option<bool>,
    /// Where it came from, as `expandSkill` reports it (`pluginSource`).
    pub source: JsValue,
    /// Its place in the list the edge gave, which the edge maps back to its `HostPlugin`.
    pub given: usize,
}

/// A `HostPlugin` as the edge passes it: `plugin` is the core's copy (`pluginData`).
#[derive(Deserialize)]
struct HostPluginWire {
    id: String,
    plugin: Own<GivenPlugin>,
    #[serde(default)]
    enabled: Option<bool>,
    #[serde(default)]
    source: JsValue,
}

impl<'de> Deserialize<'de> for HostPlugin {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = HostPluginWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            plugin: wire.plugin.0,
            enabled: wire.enabled,
            source: wire.source,
            given: 0,
        })
    }
}

impl HostPlugin {
    /// The plugin's manifest name.
    #[must_use]
    pub fn name(&self) -> &str {
        self.plugin.name()
    }

    /// The plugin's manifest member `name`, `undefined` when it has no manifest.
    #[must_use]
    pub fn manifest(&self, name: &str) -> Option<&JsValue> {
        self.plugin
            .plugin
            .get("manifest")
            .and_then(|manifest| manifest.get(name))
    }
}

/// The reserved origins of an id (`<name>@inline` and the rest); any other is a marketplace's.
const RESERVED: [(&str, PluginOrigin); 3] = [
    ("inline", PluginOrigin::SessionOnly),
    ("skills-dir", PluginOrigin::SkillsDir),
    ("synced", PluginOrigin::Synced),
];

/// Where a plugin came from, read from its id as the docs' table does.
fn origin_of(id: &str) -> PluginOrigin {
    let origin = parse_plugin_id(id)
        .map(|parsed| parsed.origin)
        .unwrap_or_default();
    RESERVED
        .iter()
        .find(|(name, _)| *name == origin)
        .map_or(PluginOrigin::Installed, |(_, kind)| *kind)
}

/// The host's plugins with a session's own over them: one under a host plugin's id replaces it, the
/// others are added after the host's (judgment, spec/judgments.md: a project or local install and a
/// `--plugin-dir` plugin are the session's, plugins/install.md and plugins/loading.md).
#[must_use]
pub fn layered_plugins(host: Vec<HostPlugin>, own: Option<Vec<HostPlugin>>) -> Vec<HostPlugin> {
    let Some(own) = own else { return host };
    let ids: HashSet<&str> = own.iter().map(|entry| entry.id.as_str()).collect();
    let mut layered: Vec<HostPlugin> = host
        .iter()
        .filter(|entry| !ids.contains(entry.id.as_str()))
        .cloned()
        .collect();
    layered.extend(own.iter().cloned());
    layered
}

/// The plugins kept under distinct ids, and one error per plugin given again under an id.
#[derive(Debug, Default)]
pub struct Distinct {
    /// The first plugin given under each id.
    pub kept: Vec<HostPlugin>,
    /// What was not loaded.
    pub issues: Vec<PluginIssue>,
}

/// The plugins under distinct ids: the id names a plugin's hooks, options and `enabledPlugins` row,
/// so of two given under one id the first is kept and the other reported.
#[must_use]
pub fn distinct_plugins(plugins: Vec<HostPlugin>) -> Distinct {
    let mut ids = HashSet::new();
    let mut distinct = Distinct::default();
    for entry in plugins {
        if ids.contains(&entry.id) {
            distinct.issues.push(error(
                &entry.id,
                "A plugin with this id was given before: this one is not loaded",
            ));
        } else {
            ids.insert(entry.id.clone());
            distinct.kept.push(entry);
        }
    }
    distinct
}

/// One plugin and whether it runs.
#[derive(Clone, Debug)]
pub struct PluginRun {
    /// The plugin.
    pub entry: HostPlugin,
    /// Whether it runs.
    pub enabled: bool,
}

/// One copy of a manifest name, as the conflict order reads it.
async fn candidate_of(view: &dyn PolicyView, run: &PluginRun) -> NameCandidate {
    let decision = enabled_plugin(view, &run.entry.id, true).await;
    NameCandidate {
        id: run.entry.id.clone(),
        name: run.entry.name().to_owned(),
        origin: origin_of(&run.entry.id),
        enabled: run.enabled,
        managed_locked: Some(decision.decided_by == DecidedBy::Scope(PolicyScope::Managed)),
        skills_dir: None,
    }
}

/// The runs after the name conflicts, and their warnings.
#[derive(Debug, Default)]
pub struct Named {
    /// Every run, a copy that does not run turned off.
    pub runs: Vec<PluginRun>,
    /// The documented row of each copy turned off.
    pub issues: Vec<PluginIssue>,
}

/// Which copies of each manifest name run. Copies from different origins follow the documented order
/// (managed, session-only, installed, skills-directory, synced); copies installed from different
/// marketplaces all stay enabled ("both appear as enabled", plugins/loading.md, "Hooks when two enabled
/// plugins share a name"). A copy that does not run is off, with its documented row as a warning.
pub async fn with_name_conflicts(view: &dyn PolicyView, runs: Vec<PluginRun>) -> Named {
    let mut by_name: IndexMap<&str, Vec<&PluginRun>> = IndexMap::new();
    for run in &runs {
        by_name.entry(run.entry.name()).or_default().push(run);
    }
    let mut off: IndexMap<String, Option<String>> = IndexMap::new();
    for group in by_name.values() {
        if group.len() < 2 {
            continue;
        }
        let mut candidates = Vec::with_capacity(group.len());
        for run in group {
            candidates.push(candidate_of(view, run).await);
        }
        let verdicts = resolve_name_conflict(&candidates);
        let winner = verdicts.iter().find(|verdict| verdict.loads);
        let installed_wins = winner.is_some_and(|it| origin_of(&it.id) == PluginOrigin::Installed);
        for verdict in &verdicts {
            let same_origin = installed_wins && origin_of(&verdict.id) == PluginOrigin::Installed;
            if !verdict.loads && !same_origin {
                off.insert(verdict.id.clone(), verdict.note.clone());
            }
        }
    }
    let off_ids: HashMap<&str, ()> = off.keys().map(|id| (id.as_str(), ())).collect();
    let turned = runs
        .iter()
        .map(|run| PluginRun {
            entry: run.entry.clone(),
            enabled: run.enabled && !off_ids.contains_key(run.entry.id.as_str()),
        })
        .collect();
    let issues = off
        .iter()
        .filter_map(|(id, note)| note.as_deref().map(|note| warning(id, note)))
        .collect();
    Named {
        runs: turned,
        issues,
    }
}
