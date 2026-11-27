//! Port of `src/host/session-skill-preload.ts`, switched.
//!
//! The skills a subagent preloads: the plan over the session's catalog (`planPreloadedSkills`), then
//! each planned skill's body as its file holds it now. An entry crosses as its data and its reader
//! (`PreloadEntry`): its plugin as its id and name, which is what the resolver reads of it.

use serde::{Deserialize, Serialize};

use super::session_skill_expand::{ContentEntry, content_of};
use crate::agents::preload_skills::{
    SkillCatalog, SkillSkipReason, SkippedSkill, plan_preloaded_skills,
};
use crate::js::{JsObject, JsValue};
use crate::ports::{BoxFuture, Signal};
use crate::skills::expand_policy::SkillOriginKind;
use crate::skills::invocation::{Invocable, invocation_policy};
use crate::skills::lookup::{LookupRow, SkillLookup, skill_lookup};
use crate::skills::visibility::SkillOverrides;

/// An entry's plugin, as the resolver reads it.
#[derive(Clone, Debug, Deserialize)]
pub struct EntryPlugin {
    /// The plugin's id.
    pub id: String,
    /// Its manifest name, which scopes its commands.
    pub name: String,
}

/// One entry of the session's catalog, as the preload reads it.
#[derive(Clone, Debug, Deserialize)]
pub struct PreloadEntry {
    /// The head, the place and the reader.
    #[serde(flatten)]
    pub content: ContentEntry,
    /// Every name that invokes it.
    pub names: Vec<String>,
    /// The plugin it belongs to, for a plugin's skill.
    #[serde(default)]
    pub plugin: Option<EntryPlugin>,
}

/// The listed skills with their full content, in the list's order, and the ones skipped.
#[derive(Clone, Debug, Serialize)]
pub struct PreloadedSkills {
    /// The listed skills with their full content, in the list's order.
    pub preloaded: Vec<JsObject>,
    /// The listed skills that are not injected, and why.
    pub skipped: Vec<SkippedSkill>,
}

/// What a preload reads besides the entries: the overrides in force and the agent.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct PreloadFacts {
    /// `skillOverrides`.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
    /// The agent's `CatalogAgent.source`.
    #[serde(default)]
    pub source: Option<JsValue>,
    /// The definition's `skills`.
    #[serde(default)]
    pub skills: Option<Vec<String>>,
}

fn command_of(entry: &PreloadEntry) -> &str {
    let command = entry.content.invocable.get("command");
    command.and_then(JsValue::as_str).unwrap_or_default()
}

fn lookup_of(entries: &[&PreloadEntry]) -> SkillLookup {
    let rows: Vec<LookupRow> = entries
        .iter()
        .map(|entry| LookupRow {
            command: command_of(entry).to_owned(),
            kind: entry
                .content
                .invocable
                .get("origin")
                .and_then(|origin| origin.get("kind"))
                .and_then(|kind| SkillOriginKind::deserialize(kind.clone()).ok())
                .unwrap_or(SkillOriginKind::Plugin),
            names: entry.names.clone(),
        })
        .collect();
    skill_lookup(&rows)
}

/// A plugin entry that answers to `<its plugin>:<name>`; `plugin_id` narrows it to one plugin.
fn plugin_skill_named(entry: &PreloadEntry, name: &str, plugin_id: Option<&str>) -> bool {
    entry.plugin.as_ref().is_some_and(|plugin| {
        plugin_id.is_none_or(|id| plugin.id == id)
            && entry.names.contains(&format!("{}:{name}", plugin.name))
    })
}

/// The skill a listed name preloads, among the skills Claude can invoke. Oracle 2.1.322
/// (spec/judgments.md): the exact name (every name a skill answers to, as `expandSkill` finds it);
/// else, for a plugin agent, its own plugin's `<plugin>:<name>`; else the first plugin, in load
/// order, with a skill `<plugin>:<name>`.
struct Resolver<'a> {
    entries: &'a [PreloadEntry],
    invocable: Vec<&'a PreloadEntry>,
    exact: SkillLookup,
    known: SkillLookup,
    own: Option<String>,
}

fn resolver_of<'a>(entries: &'a [PreloadEntry], facts: &PreloadFacts) -> Resolver<'a> {
    let overrides = facts.overrides.clone().unwrap_or_default();
    let invocable: Vec<&PreloadEntry> = entries
        .iter()
        .filter(|entry| {
            Invocable::deserialize(JsValue::Object(entry.content.invocable.clone()))
                .is_ok_and(|skill| invocation_policy(&skill, &overrides).preloadable)
        })
        .collect();
    let exact = lookup_of(&invocable);
    let known = lookup_of(&entries.iter().collect::<Vec<_>>());
    let own = facts
        .source
        .as_ref()
        .and_then(|source| source.get("pluginId"))
        .map(crate::js::string_of);
    Resolver { entries, invocable, exact, known, own }
}

impl<'a> Resolver<'a> {
    fn find(&self, name: &str) -> Option<&'a PreloadEntry> {
        let exact = self.exact.find(name).and_then(|index| self.invocable.get(index)).copied();
        let own = || {
            let own = self.own.as_deref()?;
            self.invocable.iter().copied().find(|each| plugin_skill_named(each, name, Some(own)))
        };
        let any = || self.invocable.iter().copied().find(|each| plugin_skill_named(each, name, None));
        exact.or_else(own).or_else(any)
    }

    fn exists(&self, name: &str) -> bool {
        self.known.find(name).is_some()
            || self.entries.iter().any(|each| plugin_skill_named(each, name, None))
    }
}

impl SkillCatalog for Resolver<'_> {
    fn has<'b>(&'b self, name: &'b str) -> BoxFuture<'b, bool> {
        let exists = self.exists(name);
        Box::pin(async move { exists })
    }

    fn is_model_invocable<'b>(&'b self, name: &'b str) -> BoxFuture<'b, bool> {
        let found = self.find(name).is_some();
        Box::pin(async move { found })
    }
}

/// The plan over the session's catalog, then each planned skill's body as its file holds it now.
pub async fn preload_from(
    entries: Vec<PreloadEntry>,
    facts: PreloadFacts,
    signal: Option<Signal>,
) -> PreloadedSkills {
    let resolver = resolver_of(&entries, &facts);
    let plan = plan_preloaded_skills(facts.skills.as_deref(), &resolver).await;
    let mut preloaded = Vec::new();
    let mut skipped = plan.skipped;
    for name in plan.load {
        let content = match resolver.find(&name) {
            Some(entry) => content_of(&entry.content, signal.as_ref()).await.ok(),
            None => None,
        };
        match content {
            Some(content) => preloaded.push(content),
            None => skipped.push(SkippedSkill { name, reason: SkillSkipReason::Missing }),
        }
    }
    PreloadedSkills { preloaded, skipped }
}

crate::registry::exports! {
    "host/session-skill-preload" {
        "preloadFrom" => async ok |entries: Vec<PreloadEntry>, facts: PreloadFacts, signal: Option<Signal>|
            preload_from(entries, facts, signal).await;
    }
}
