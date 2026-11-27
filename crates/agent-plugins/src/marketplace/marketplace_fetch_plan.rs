//! Port of `src/marketplace/marketplace-fetch-plan.ts`, switched.
//!
//! Planning how a marketplace source is fetched (plugins/marketplace-reference.md, "Marketplace
//! sources" and "Fields by type"; plugins/host-marketplace.md, "Stay within the download limits for
//! hosted files"): a source in, the plan or the issue that refuses it out. Nothing is read here;
//! the host runs the plan. Issue paths are relative to the source object.

use std::sync::LazyLock;

use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::header_map::normalize_headers;
use super::source_safety::{
    github_repo_issues, read_git_address, ref_issues, repository_path_issues,
};
use super::types::{MarketplaceFetchPlan, MarketplaceRedirects};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::source_failures::{SOURCE_FAILURE_NPM, SOURCE_FAILURE_POLICY_ONLY};
use crate::format::marketplace::types::MarketplaceSource;
use crate::js::{JsValue, Pattern};
use crate::ports::marketplace::GitRepository;

/// The plan, or the issue that refuses the source.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Planned {
    /// How it is fetched.
    Plan(MarketplaceFetchPlan),
    /// Why it cannot be.
    Refused(PluginIssue),
}

/// The limits of a `marketplace.json` downloaded from a `url` source: 5 MiB, 10 seconds, and the
/// redirect rule of "Stay within the download limits for hosted files": a redirect to a different
/// origin must use `https://` and can't point at a loopback, link-local or cloud-metadata host
/// (`is_blocked_host`), and the request it sends carries none of the configured headers.
const MAX_BYTES: f64 = 5_242_880.0;
const TIMEOUT_MS: f64 = 10_000.0;

fn redirects() -> MarketplaceRedirects {
    MarketplaceRedirects {
        cross_origin: "https-only".to_owned(),
        blocked_hosts: "refused".to_owned(),
        headers_across_origins: "none".to_owned(),
    }
}

fn plan_url(
    url: &str,
    headers: Option<&IndexMap<String, String>>,
    helper: Option<&String>,
) -> Planned {
    Planned::Plan(MarketplaceFetchPlan::HttpFile {
        url: url.to_owned(),
        max_bytes: MAX_BYTES,
        timeout_ms: TIMEOUT_MS,
        redirects: redirects(),
        headers: headers.map_or_else(|| normalize_headers(&IndexMap::new()), normalize_headers),
        helper: helper.cloned(),
    })
}

/// What a `github` or `git` source clones.
struct Cloned<'a> {
    reference: Option<&'a String>,
    path: &'a str,
    sparse_paths: Option<&'a Vec<String>>,
}

/// The clone of a `github` or `git` source: its ref and sparse checkout when it has them.
fn clone_of(repo: GitRepository, source: &Cloned) -> Planned {
    Planned::Plan(MarketplaceFetchPlan::Git {
        repo,
        reference: source.reference.cloned(),
        path: source.path.to_owned(),
        sparse_paths: source.sparse_paths.cloned(),
        lfs: "pointer-files".to_owned(),
    })
}

/// What either clone refuses before it runs (spec/judgments.md): a `ref` git cannot name or would
/// read as an option, and a `path` or sparse path that leaves the repository.
fn clone_issues(source: &Cloned) -> Vec<PluginIssue> {
    let mut issues = ref_issues(source.reference.map(String::as_str), "ref");
    issues.extend(repository_path_issues(source.path, "path"));
    for (at, sparse) in source.sparse_paths.into_iter().flatten().enumerate() {
        issues.extend(repository_path_issues(
            sparse,
            &format!("sparsePaths[{at}]"),
        ));
    }
    issues
}

fn first_or(mut refused: Vec<PluginIssue>, plan: impl FnOnce() -> Planned) -> Planned {
    if refused.is_empty() {
        plan()
    } else {
        Planned::Refused(refused.swap_remove(0))
    }
}

/// `repo` names one repository: an owner wildcard is taken literally and the clone fails.
fn plan_github(repo: &str, source: &Cloned) -> Planned {
    let mut refused = github_repo_issues(repo, "repo");
    refused.extend(clone_issues(source));
    first_or(refused, || {
        clone_of(
            GitRepository::Shorthand {
                shorthand: repo.to_owned(),
            },
            source,
        )
    })
}

/// `url` takes the git URL forms a `url` plugin source takes.
fn plan_git(url: &str, source: &Cloned) -> Planned {
    let reading = read_git_address(url, false, "url");
    let mut refused = reading.issues;
    refused.extend(clone_issues(source));
    let repo = reading.repo.unwrap_or_else(|| GitRepository::Url {
        url: url.to_owned(),
    });
    first_or(refused, || clone_of(repo, source))
}

/// Where a path stops being its directory: the last `/`, or `\` for a Windows path.
fn last_separator(path: &str) -> Option<usize> {
    path.rfind(['/', '\\'])
}

/// `/^[A-Za-z]:$/`.
static DRIVE: Pattern = LazyLock::new(|| Regex::new("^[A-Za-z]:$").ok());

/// The directory that holds a path, as `dirname` gives it: `.` for a bare name, the separator for
/// a file at the root, and the drive with its separator (`C:\`) for a file directly under a drive.
fn directory_of(path: &str) -> String {
    let Some(at) = last_separator(path) else {
        return ".".to_owned();
    };
    // `at` is the byte of an ASCII separator, so every slice below falls on a character boundary.
    let head = path.get(..at).unwrap_or_default();
    let directory = if at == 0 {
        path.get(..1)
    } else if DRIVE.as_ref().is_some_and(|regex| regex.is_match(head)) {
        path.get(..=at)
    } else {
        Some(head)
    };
    directory.unwrap_or_default().to_owned()
}

/// The directory two levels above the file: `<root>/.claude-plugin/marketplace.json` gives
/// `<root>`. A relative path with fewer directories stops at `.` (spec/judgments.md).
fn two_levels_up(path: &str) -> String {
    directory_of(&directory_of(path))
}

fn refusal(message: &str) -> Planned {
    Planned::Refused(error("source", message))
}

/// How the marketplace file is fetched, or why it cannot be.
#[must_use]
pub fn plan_marketplace_fetch(source: &MarketplaceSource) -> Planned {
    match source {
        MarketplaceSource::Url {
            url,
            headers,
            headers_helper,
        } => plan_url(url, headers.as_ref(), headers_helper.as_ref()),
        MarketplaceSource::Github {
            repo,
            reference,
            path,
            sparse_paths,
            ..
        } => plan_github(
            repo,
            &Cloned {
                reference: reference.as_ref(),
                path,
                sparse_paths: sparse_paths.as_ref(),
            },
        ),
        MarketplaceSource::Git {
            url,
            reference,
            path,
            sparse_paths,
            ..
        } => plan_git(
            url,
            &Cloned {
                reference: reference.as_ref(),
                path,
                sparse_paths: sparse_paths.as_ref(),
            },
        ),
        MarketplaceSource::Npm { .. } => refusal(SOURCE_FAILURE_NPM),
        MarketplaceSource::File { path } => Planned::Plan(MarketplaceFetchPlan::ReadFile {
            path: path.clone(),
            root: two_levels_up(path),
        }),
        MarketplaceSource::Directory { path } => {
            Planned::Plan(MarketplaceFetchPlan::ReadDirectory { root: path.clone() })
        }
        MarketplaceSource::Settings { plugins, .. } => {
            Planned::Plan(MarketplaceFetchPlan::Inline {
                entries: plugins
                    .iter()
                    .filter_map(|entry| crate::js::value::to_js_value(entry).ok())
                    .collect(),
            })
        }
        MarketplaceSource::SkillsDir {}
        | MarketplaceSource::HostPattern { .. }
        | MarketplaceSource::PathPattern { .. } => refusal(SOURCE_FAILURE_POLICY_ONLY),
    }
}

/// A `settings` source's entries go into the plan as the host gave them, so only its `kind` and
/// `plugins` are read; any other source is read whole.
#[derive(Deserialize)]
struct SettingsEntries {
    kind: String,
    #[serde(default)]
    plugins: Vec<JsValue>,
}

fn plan_given(source: &JsValue) -> Result<Planned, String> {
    if let Ok(settings) = SettingsEntries::deserialize(source.clone())
        && settings.kind == "settings"
    {
        return Ok(Planned::Plan(MarketplaceFetchPlan::Inline {
            entries: settings.plugins,
        }));
    }
    let source =
        MarketplaceSource::deserialize(source.clone()).map_err(|cause| cause.to_string())?;
    Ok(plan_marketplace_fetch(&source))
}

crate::registry::exports! {
    "marketplace/marketplace-fetch-plan" {
        "planMarketplaceFetch" => fallible |source: JsValue| plan_given(&source);
    }
}
