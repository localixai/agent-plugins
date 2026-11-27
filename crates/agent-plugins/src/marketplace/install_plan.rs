//! Port of `src/marketplace/install-plan.ts` at sha256:b137d8a8f20a.
//!
//! Installing a marketplace plugin as two pure steps (plugins/loading.md, "Find plugins on disk",
//! "In-place and copied plugins", "Cleanup of previous versions"; plugins/marketplace-reference.md,
//! "Copy mode and link mode"): what to fetch and how it will be installed, then, once the host has
//! fetched it and the version is known, where it goes and what `installed_plugins.json` becomes.
//! The kit moves nothing; the host runs the fetch, the copy or the links and writes the file.

use serde::{Deserialize, Serialize};

use super::command_output::same_path;
use super::fetch_plan::plan_given;
use super::install_mode::{
    Decided, InstallDecision, InstallOrigin, ModeInput, decide_install_mode,
};
use super::install_scopes::InstallScope;
use super::installed_plugins::{InstallRecord, InstalledRecords};
use super::orphan_cleanup::dirs_to_orphan;
use super::submodules::MarketplaceSourceKind;
use super::types::{FetchPlan, FetchPlanContext};
use super::version_resolution::{PluginVersion, is_version_segment};
use crate::format::config_directory::Join;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::types::PluginSource;
use crate::format::path_text::PathPlatform;
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::registry::{Callback, Port};

/// What `planInstallStart` answers.
#[derive(Clone, Debug, Serialize)]
pub struct InstallStart {
    /// What to fetch; `None` when the source is refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch: Option<FetchPlan>,
    /// How the plugin will be installed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<InstallDecision>,
    /// Why it may not be.
    pub issues: Vec<PluginIssue>,
}

/// What `planInstallStart` is given.
#[derive(Clone, Debug, Deserialize)]
pub struct InstallStartInput {
    /// The entry's source, as the marketplace wrote it.
    pub source: JsValue,
    /// `marketplaceOrigin` is how the marketplace the plugin is listed in was added.
    pub fetch: FetchPlanContext,
}

/// A marketplace kind as its name, as `InstallOrigin` reads it.
fn kind_name(kind: MarketplaceSourceKind) -> Option<String> {
    to_js_value(&kind).ok()?.as_str().map(str::to_owned)
}

/// Before the fetch: its plan and how the plugin will be installed. A refusal of either is an issue and plans nothing.
///
/// # Errors
///
/// The thrown message when `source` is of a known kind but not a valid source of it.
pub fn plan_install_start(input: &InstallStartInput) -> Result<InstallStart, String> {
    let planned = plan_given(&input.source, &input.fetch)?;
    let Some(plan) = planned.plan else {
        return Ok(InstallStart {
            fetch: None,
            decision: None,
            issues: planned.issues,
        });
    };
    let source =
        PluginSource::deserialize(input.source.clone()).map_err(|cause| cause.to_string())?;
    let decided = decide_install_mode(&ModeInput {
        origin: InstallOrigin::Marketplace {
            marketplace: input
                .fetch
                .marketplace_origin
                .as_ref()
                .and_then(|origin| kind_name(origin.kind)),
            source,
        },
        platform: input.fetch.platform,
        printed: None,
    });
    Ok(match decided {
        Decided::Issue(issue) => InstallStart {
            fetch: None,
            decision: None,
            issues: vec![issue],
        },
        Decided::Decision(decision) => InstallStart {
            fetch: Some(plan),
            decision: Some(decision),
            issues: Vec::new(),
        },
    })
}

/// A link of a link-mode cache entry: one per top-level entry of the printed directory.
#[derive(Clone, Debug, Serialize)]
pub struct CacheLink {
    /// The entry's name inside the cache directory.
    pub name: String,
    /// What it links to: that entry of the printed directory.
    pub target: String,
}

/// What `planInstallFinish` is given; the host's record holds `layout` and `join`, so it crosses
/// as a port and each member is read from it (`finish_from_host`).
pub struct InstallFinish {
    /// The plugin's name.
    pub name: String,
    /// The marketplace it is listed in.
    pub marketplace: String,
    /// How it is installed.
    pub decision: InstallDecision,
    /// Its version.
    pub version: PluginVersion,
    /// The host's `StoreLayout`.
    pub layout: Port,
    /// The host's path join, for the targets of the links.
    pub join: Callback,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// The scope it is installed in.
    pub scope: InstallScope,
    /// The folder of a `project` or `local` install.
    pub project_path: Option<String>,
    /// Where a plugin loaded in place lies, or the directory a `command` source printed.
    pub source_directory: Option<String>,
    /// The top-level names of the printed directory, for link mode.
    pub top_level: Option<Vec<String>>,
    /// The commit a git-sourced plugin was installed at.
    pub git_commit_sha: Option<String>,
    /// The records now in `installed_plugins.json`.
    pub existing: InstalledRecords,
    /// The time of the install, as the text the file records (the kit reads no clock).
    pub at: String,
}

/// What `planInstallFinish` answers.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallFinished {
    /// The folder a copy is committed to, or a link-mode entry is filled in; `None` for a plugin loaded in place.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    /// The host copies the fetched files into `destination`; false for link mode, which copies nothing, and for a plugin loaded in place.
    pub copy: bool,
    /// The links of a link-mode entry.
    pub links: Vec<CacheLink>,
    /// Link mode: "Keep the directory in place". The links point into this directory, so it must stay where it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub must_stay_at: Option<String>,
    /// The record of this install.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<InstallRecord>,
    /// The records to write; `existing` when nothing could be planned.
    pub records: InstalledRecords,
    /// The version directories the update left behind: write `.orphaned_at` into each.
    pub orphaned: Vec<String>,
    /// Why nothing could be planned.
    pub issues: Vec<PluginIssue>,
}

fn problem(message: &str) -> PluginIssue {
    error("source", message)
}

/// The record that this install replaces: the same scope, and for a project or local install the
/// same folder, whatever way the platform spells it (on Windows `C:\work\app` and `c:\work\app`
/// are one folder, and two records of it are what plugins/troubleshooting.md describes).
fn same_install(one: &InstallRecord, input: &InstallFinish) -> bool {
    if one.scope != input.scope {
        return false;
    }
    match (&one.project_path, &input.project_path) {
        (Some(one), Some(given)) => same_path(one, given, input.platform),
        (one, given) => one == given,
    }
}

/// A top-level name that cannot be a link in the cache entry: empty, a dot entry, or with a separator.
fn unsafe_link_name(name: &str) -> bool {
    name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0')
}

/// Link mode needs the printed directory and what is in it, and names that stay inside the cache entry.
fn link_problem(input: &InstallFinish) -> Option<PluginIssue> {
    if !input.decision.linked {
        return None;
    }
    let names = input.top_level.as_deref().unwrap_or_default();
    if input.source_directory.is_none() || names.is_empty() {
        return Some(problem(
            "A link-mode plugin needs the printed directory and its top-level entries",
        ));
    }
    let unsafe_name = names.iter().find(|name| unsafe_link_name(name))?;
    Some(problem(&format!(
        "The printed directory has an entry that cannot be linked into the cache: \"{unsafe_name}\""
    )))
}

/// Where the plugin's files are, as `installed_plugins.json` records it, and the cache folder.
struct Place {
    version_dir: String,
    install_path: String,
}

/// The cache folder (`cache/<marketplace>/<plugin>/<version>/`) and the recorded path, or why they
/// cannot be named. Only a plugin with a folder in the cache needs a version that can name one; a
/// plugin loaded in place is recorded with the directory it lies in (judgment).
async fn place_of(input: &InstallFinish) -> Result<Place, PluginIssue> {
    let cached = input.decision.cached;
    if cached && !is_version_segment(&input.version.version) {
        return Err(problem(&format!(
            "The version \"{}\" cannot name a folder",
            input.version.version
        )));
    }
    let version_dir = input
        .layout
        .method::<String>(
            "versionDir",
            vec![
                JsValue::from(input.marketplace.as_str()),
                JsValue::from(input.name.as_str()),
                JsValue::from(input.version.version.as_str()),
            ],
        )
        .await;
    if cached {
        let install_path = version_dir.clone();
        return Ok(Place {
            version_dir,
            install_path,
        });
    }
    input.source_directory.clone().map_or_else(
        || {
            Err(problem(
                "A plugin loaded in place needs the directory it lies in",
            ))
        },
        |install_path| {
            Ok(Place {
                version_dir,
                install_path,
            })
        },
    )
}

/// `NO_CHANGE`: what is answered when nothing could be planned.
fn no_change(input: &InstallFinish, issues: Vec<PluginIssue>) -> InstallFinished {
    InstallFinished {
        destination: None,
        copy: false,
        links: Vec::new(),
        must_stay_at: None,
        record: None,
        records: input.existing.clone(),
        orphaned: Vec::new(),
        issues,
    }
}

/// The links of a link-mode entry: "a link to each top-level entry of the printed directory".
async fn links_of(input: &InstallFinish) -> Vec<CacheLink> {
    let Some(printed) = input.source_directory.as_deref() else {
        return Vec::new();
    };
    if !input.decision.linked {
        return Vec::new();
    }
    let mut links = Vec::new();
    for name in input.top_level.as_deref().unwrap_or_default() {
        links.push(CacheLink {
            name: name.clone(),
            target: input.join.join(printed, name).await,
        });
    }
    links
}

fn record_of(
    input: &InstallFinish,
    install_path: String,
    replaced: Option<&InstallRecord>,
) -> InstallRecord {
    InstallRecord {
        scope: input.scope,
        install_path,
        version: input.version.version.clone(),
        installed_at: Some(
            replaced
                .and_then(|one| one.installed_at.clone())
                .unwrap_or_else(|| input.at.clone()),
        ),
        last_updated: Some(input.at.clone()),
        git_commit_sha: input.git_commit_sha.clone(),
        project_path: input.project_path.clone(),
        extra: None,
    }
}

/// After the fetch: the cache folder and the record. An update puts the new record in the old one's
/// place and orphans the version folder it leaves behind, "so a session that already loaded the old
/// version keeps running"; a folder outside the cache (a plugin loaded in place) is never marked,
/// since a sweep would delete the user's own files. Link mode copies nothing: it links each
/// top-level entry of the printed directory, which `mustStayAt` says must stay where it is.
pub async fn plan_install_finish(input: &InstallFinish) -> InstallFinished {
    let place = match place_of(input).await {
        Ok(place) => place,
        Err(issue) => return no_change(input, vec![issue]),
    };
    if let Some(refused) = link_problem(input) {
        return no_change(input, vec![refused]);
    }
    let id = format!("{}@{}", input.name, input.marketplace);
    let before = input.existing.get(&id).cloned().unwrap_or_default();
    // `one !== replaced`: the record found is the one at its index.
    let replaced_at = before.iter().position(|one| same_install(one, input));
    let replaced = replaced_at.and_then(|at| before.get(at));
    let record = record_of(input, place.install_path, replaced);
    let mut kept: Vec<InstallRecord> = before
        .iter()
        .enumerate()
        .filter(|(at, _)| Some(*at) != replaced_at)
        .map(|(_, one)| one.clone())
        .collect();
    kept.push(record.clone());
    let mut records = input.existing.clone();
    records.insert(id, kept);
    let links = links_of(input).await;
    let remaining: Vec<InstallRecord> = records.values().flatten().cloned().collect();
    let removed = replaced.map(std::slice::from_ref).unwrap_or_default();
    let orphaned = dirs_to_orphan(removed, &remaining, &input.layout).await;
    let printed = input.source_directory.clone();
    InstallFinished {
        destination: input.decision.cached.then_some(place.version_dir),
        copy: input.decision.cached && !input.decision.linked,
        links,
        must_stay_at: printed.filter(|_| input.decision.linked),
        record: Some(record),
        records,
        orphaned,
        issues: Vec::new(),
    }
}

/// `InstallFinish` as the host passed it: a record that holds its `layout` and `join`, so it
/// crossed as a port and each member is read from it.
async fn finish_from_host(input: Port) -> InstallFinished {
    let data = InstallFinish {
        name: input.member("name").await,
        marketplace: input.member("marketplace").await,
        decision: input.member("decision").await,
        version: input.member("version").await,
        layout: input.member("layout").await,
        join: input.member("join").await,
        platform: input.member("platform").await,
        scope: input.member("scope").await,
        project_path: input.member("projectPath").await,
        source_directory: input.member("sourceDirectory").await,
        top_level: input.member("topLevel").await,
        git_commit_sha: input.member("gitCommitSha").await,
        existing: input.member("existing").await,
        at: input.member("at").await,
    };
    plan_install_finish(&data).await
}

crate::registry::exports! {
    "marketplace/install-plan" {
        "planInstallStart" => fallible |input: InstallStartInput| plan_install_start(&input);
        "planInstallFinish" => async ok |input: Port| finish_from_host(input).await;
    }
}
