//! Port of `src/marketplace/stage-report.ts`, switched.
//!
//! How far a plugin got (plugins/loading.md, "Check which stage a plugin reached"; plugins/
//! overview.md, "Make an installed plugin available in your session"): an `enabledPlugins` entry is
//! **declared** in settings, **fetched** to disk (`installed_plugins.json` records it and `cache/`
//! holds its files), and **loaded** in the running session, which "changes to settings or to disk
//! don't reach ... until you run `/reload-plugins` or start a new session". Pure over facts the host
//! reads: the settings, the records, which install paths exist, and the set the session loaded.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::install_errors::{ErrorsTabEntry, PLUGINS_CHANGED, not_cached};
use super::installed_plugins::InstallRecord;
use super::known_marketplaces::{KnownMarketplaces, same_marketplace_source};
use crate::format::marketplace::ids::plugin_name_of;
use crate::format::marketplace::types::MarketplaceSource;
use crate::registry::Set;

/// What the host read about one plugin.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageFacts {
    /// `<name>@<marketplace>`.
    pub id: String,
    /// Settings declare the plugin on (`enabledPlugins`, after precedence).
    pub enabled: bool,
    /// Its install records, as `installed_plugins.json` lists them.
    pub records: Vec<InstallRecord>,
    /// Which of the records' `installPath` exist on disk.
    pub existing: Set<String>,
    /// A relative-path plugin of a marketplace added from a local path: it loads in place and needs no record.
    #[serde(default)]
    pub in_place: Option<bool>,
    /// The plugin is in the set the session loaded at startup or at the last `/reload-plugins`.
    pub loaded: bool,
}

/// The furthest stage a plugin reached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Not even declared.
    None,
    /// In settings.
    Declared,
    /// On disk.
    Fetched,
    /// In the session.
    Loaded,
}

/// The three layers of overview.md: settings, disk and session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// Settings.
    Settings,
    /// Disk.
    Disk,
    /// Session.
    Session,
}

/// Where a plugin is.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's JSON shape: one fact per flag"
)]
pub struct StageReport {
    /// Settings declare it.
    pub declared: bool,
    /// `installed_plugins.json` has a record for it (or it loads in place).
    pub recorded: bool,
    /// A directory its record points at exists (or it loads in place).
    pub cached: bool,
    /// The session loaded it.
    pub loaded: bool,
    /// Settings or disk differ from the session: `/reload-plugins` or a new session applies the change.
    pub reload: bool,
    /// The furthest stage reached.
    pub stage: Stage,
    /// The layers it is not present at; empty when it is available in the session.
    pub missing: Vec<Layer>,
    /// It is available in the session.
    pub available: bool,
    /// `Plugin "<name>" not cached at <path>`, for an enabled plugin whose record points at nothing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<ErrorsTabEntry>,
}

/// `Pick<StageReport, 'declared' | 'recorded' | 'cached' | 'loaded' | 'reload'>`.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the fields of `StageReport` it picks"
)]
struct Reached {
    declared: bool,
    recorded: bool,
    cached: bool,
    loaded: bool,
    reload: bool,
}

/// Whether a report passes a stage's or a layer's test.
type Test = fn(Reached) -> bool;

/// The furthest stage first: a plugin is at the first stage whose test it passes.
const STAGES: [(Stage, Test); 3] = [
    (Stage::Loaded, |report| report.loaded),
    (Stage::Fetched, |report| report.cached),
    (Stage::Declared, |report| report.declared),
];

fn name_of(id: &str) -> String {
    plugin_name_of(id)
}

const LAYERS: [(Layer, Test); 3] = [
    (Layer::Settings, |report| !report.declared),
    (Layer::Disk, |report| !report.cached),
    (Layer::Session, |report| !report.loaded),
];

/// `Plugin "<name>" not cached at <path>`: an enabled plugin whose record points at a directory that is gone.
fn missing_cache(facts: &StageFacts, reached: Reached) -> Option<ErrorsTabEntry> {
    let first = facts.records.first();
    let lost = facts.enabled && reached.recorded && !reached.cached;
    if !lost {
        return None;
    }
    first.map(|first| not_cached(&name_of(&facts.id), &first.install_path))
}

/// Where the plugin is.
#[must_use]
pub fn stage_report(facts: &StageFacts) -> StageReport {
    let in_place = facts.in_place == Some(true);
    let cached = in_place
        || facts
            .records
            .iter()
            .any(|one| facts.existing.members.contains(&one.install_path));
    let base = Reached {
        declared: facts.enabled,
        recorded: in_place || !facts.records.is_empty(),
        cached,
        loaded: facts.loaded,
        reload: (facts.enabled && cached) != facts.loaded,
    };
    let missing: Vec<Layer> = LAYERS
        .iter()
        .filter(|(_, absent)| absent(base))
        .map(|(layer, _)| *layer)
        .collect();
    let problem = missing_cache(facts, base);
    StageReport {
        declared: base.declared,
        recorded: base.recorded,
        cached: base.cached,
        loaded: base.loaded,
        reload: base.reload,
        stage: STAGES
            .iter()
            .find(|(_, reached)| reached(base))
            .map_or(Stage::None, |(stage, _)| *stage),
        available: missing.is_empty(),
        missing,
        problem,
    }
}

// ---- after the session starts ----

/// What Claude Code does after the session starts for a marketplace that settings declare.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum StartupAction {
    /// One `known_marketplaces.json` lacks.
    Clone {
        /// Its name.
        marketplace: String,
        /// `clone`.
        action: &'static str,
        /// The declared source.
        source: MarketplaceSource,
        /// Next, plugins reload and enabled plugins that are not cached yet are downloaded:
        /// `reload-and-download-enabled`.
        next: &'static str,
    },
    /// One whose source changed in settings.
    Refetch {
        /// Its name.
        marketplace: String,
        /// `refetch`.
        action: &'static str,
        /// The declared source.
        source: MarketplaceSource,
        /// `notify`.
        next: &'static str,
        /// What the user is told.
        notice: &'static str,
    },
}

/// "Plugins load at session start from `installed_plugins.json` and the cache without using the
/// network. After the session starts, Claude Code checks the declared marketplaces in the
/// background": one `known_marketplaces.json` lacks is cloned (then it reloads plugins and downloads
/// the enabled ones not cached yet), and one whose source changed in settings is re-fetched from the
/// new source with `Plugins changed. Run /reload-plugins to activate.` Sources match as in
/// `sameMarketplaceSource`. A known marketplace that settings do not declare is not touched.
#[must_use]
pub fn plan_marketplace_startup(
    declared: &IndexMap<String, MarketplaceSource>,
    known: &KnownMarketplaces,
) -> Vec<StartupAction> {
    declared
        .iter()
        .flat_map(|(marketplace, source)| {
            let Some(existing) = known.get(marketplace) else {
                return vec![StartupAction::Clone {
                    marketplace: marketplace.clone(),
                    action: "clone",
                    source: source.clone(),
                    next: "reload-and-download-enabled",
                }];
            };
            if same_marketplace_source(&existing.source, source) {
                Vec::new()
            } else {
                vec![StartupAction::Refetch {
                    marketplace: marketplace.clone(),
                    action: "refetch",
                    source: source.clone(),
                    next: "notify",
                    notice: PLUGINS_CHANGED,
                }]
            }
        })
        .collect()
}

/// The `/plugin` Errors tab entries after the start: an enabled plugin that neither the clone nor
/// the re-fetch brought in (`fetchedAfterStart`) and that has no usable cache directory shows
/// `Plugin "<name>" not cached at <path>`.
#[must_use]
pub fn unresolved_after_start(
    plugins: &[StageFacts],
    fetched_after_start: &Set<String>,
) -> Vec<ErrorsTabEntry> {
    plugins
        .iter()
        .filter(|one| !fetched_after_start.members.contains(&one.id))
        .filter_map(|one| stage_report(one).problem)
        .collect()
}

crate::registry::exports! {
    "marketplace/stage-report" {
        "stageReport" => ok |facts: StageFacts| stage_report(&facts);
        "planMarketplaceStartup" => ok |declared: IndexMap<String, MarketplaceSource>, known: KnownMarketplaces| plan_marketplace_startup(&declared, &known);
        "unresolvedAfterStart" => ok |plugins: Vec<StageFacts>, fetched_after_start: Set<String>| unresolved_after_start(&plugins, &fetched_after_start);
    }
}
