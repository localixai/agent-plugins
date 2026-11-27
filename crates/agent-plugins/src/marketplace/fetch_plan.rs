//! Port of `src/marketplace/fetch-plan.ts`, switched.
//!
//! Planning how a plugin source is fetched (plugins/marketplace-reference.md, "Plugin sources" and
//! its sub-sections; plugins/host-marketplace.md, "Stay within the download limits for hosted
//! files"): pure, a source and what the host knows in, a `FetchPlan` or the issues that refuse it
//! out. Issue paths are relative to the entry (`source`, `source.url`): the caller prefixes
//! `plugins[i].`. Nothing is fetched here; a host runs the plan, or an engine over ports does.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::command_mode::{LINKS_NEED_POSIX, link_on_windows_issue};
use super::install_failures::UNSUPPORTED_SOURCE_TYPE;
use super::source_safety::{
    NpmPackageSource, NpmSafety, archive_url_issues, github_repo_issues, npm_package_issues,
    npm_registry_issues, npm_spec_of, read_git_address, ref_issues, repository_path_issues,
};
use super::submodules::{MarketplaceSourceKind, submodule_rule_for};
use super::types::{
    ArchiveFetchPlan, ArchiveLimits, Checkout, CommandCwd, CommandFetchPlan, CommandMode,
    CommandShell, FetchPlan, FetchPlanContext, GitFetchPlan, NpmFetchPlan, RootDepth,
};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::types::{
    self as source_types, PLUGIN_SOURCE_KINDS, PluginSource, PluginSourceKind,
};
use crate::format::path_text::PathPlatform;
use crate::js::JsValue;
use crate::ports::marketplace::GitRepository;

/// A plan, or the issues that refuse the source.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Planned {
    /// How the source is fetched; `None` when it is refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plan: Option<FetchPlan>,
    /// Why it may not be fetched.
    pub issues: Vec<PluginIssue>,
}

const fn planned(plan: FetchPlan) -> Planned {
    Planned {
        plan: Some(plan),
        issues: Vec::new(),
    }
}
const fn refused(issues: Vec<PluginIssue>) -> Planned {
    Planned { plan: None, issues }
}

const MIB: f64 = 1024.0 * 1024.0;

/// The limits of an `archive` source: the download and the extraction ("Stay within the download
/// limits").
pub const ARCHIVE_LIMITS: ArchiveLimits = ArchiveLimits {
    max_download_bytes: 256.0 * MIB,
    timeout_ms: 120_000.0,
    max_redirects: 5.0,
    max_entries: 100_000.0,
    max_file_bytes: 512.0 * MIB,
    max_uncompressed_bytes: 1024.0 * MIB,
    max_compression_ratio: 50.0,
};

/// The install refusal of an entry path that cannot resolve inside the marketplace directory
/// (errors.md, "Marketplace entry path does not stay inside the marketplace directory"): the
/// prefix is as quoted, the parenthesis is the kit's own (spec J17.30).
fn entry_path_refusal(context: &FetchPlanContext, path: &str, why: &str) -> String {
    let who = context
        .plugin_id
        .clone()
        .unwrap_or_else(|| format!("the entry \"{path}\""));
    format!(
        "Cannot install {who}: its marketplace entry path does not stay inside the marketplace directory ({why})"
    )
}

/// What a relative path means for each way the marketplace was added: the refusal, or `None` when
/// its files are there.
fn origin_refusal_of(
    kind: MarketplaceSourceKind,
    path: &str,
    context: &FetchPlanContext,
) -> Option<String> {
    match kind {
        MarketplaceSourceKind::Github
        | MarketplaceSourceKind::Git
        | MarketplaceSourceKind::File
        | MarketplaceSourceKind::Directory
        | MarketplaceSourceKind::Npm
        | MarketplaceSourceKind::SkillsDir
        | MarketplaceSourceKind::HostPattern
        | MarketplaceSourceKind::PathPattern => None,
        MarketplaceSourceKind::Url => Some(entry_path_refusal(
            context,
            path,
            &format!(
                "\"{path}\" is relative, and a marketplace added by URL has only its marketplace.json; give the entry a github, url or git-subdir source"
            ),
        )),
        MarketplaceSourceKind::Settings => Some(format!(
            "Relative path \"{path}\" is rejected in a marketplace declared in settings: it has no files. Give the entry a github, url or git-subdir source"
        )),
    }
}

/// On macOS and Linux a backslash in an entry path is not a separator: the path cannot be trusted
/// to stay inside (§15.6.1).
fn backslash_refusal(path: &str, context: &FetchPlanContext) -> Option<String> {
    (context.platform == PathPlatform::Posix && path.contains('\\')).then(|| {
        entry_path_refusal(
            context,
            path,
            &format!("\"{path}\" contains a backslash; write the path with forward slashes"),
        )
    })
}

fn origin_refusal(path: &str, context: &FetchPlanContext) -> Option<String> {
    let origin = context.marketplace_origin.as_ref()?;
    origin_refusal_of(origin.kind, path, context)
}

fn plan_relative(path: &str, context: &FetchPlanContext) -> Planned {
    let refusal = backslash_refusal(path, context).or_else(|| origin_refusal(path, context));
    if let Some(refusal) = refusal {
        return refused(vec![error("source", &refusal)]);
    }
    planned(FetchPlan::Local {
        from_marketplace_root: path.to_owned(),
        submodules: submodule_rule_for(path, context.marketplace_origin.as_ref()),
    })
}

const fn checkout_of(reference: Option<&String>, sha: Option<&String>) -> Checkout {
    if sha.is_some() {
        return Checkout::Sha;
    }
    if reference.is_none() {
        Checkout::DefaultBranch
    } else {
        Checkout::Ref
    }
}

/// A source's `ref` and `sha`.
struct Pin<'a> {
    reference: Option<&'a String>,
    sha: Option<&'a String>,
}

/// `ref` and `sha`: "When you set both, Claude Code checks out `sha`".
fn pinned(pin: &Pin<'_>, repo: GitRepository) -> GitFetchPlan {
    GitFetchPlan {
        repo,
        reference: pin.reference.cloned(),
        sha: pin.sha.cloned(),
        checkout: checkout_of(pin.reference, pin.sha),
        subdir: None,
        sparse: false,
        partial_clone: false,
        lfs: "pointer-files".to_owned(),
    }
}

fn plan_github(repo: &str, pin: &Pin<'_>) -> Planned {
    let mut issues = github_repo_issues(repo, "source.repo");
    issues.extend(ref_issues(pin.reference.map(String::as_str), "source.ref"));
    if !issues.is_empty() {
        return refused(issues);
    }
    let shorthand = GitRepository::Shorthand {
        shorthand: repo.to_owned(),
    };
    planned(FetchPlan::Git(pinned(pin, shorthand)))
}

fn plan_url(url: &str, pin: &Pin<'_>) -> Planned {
    let reading = read_git_address(url, false, "source.url");
    let mut issues = reading.issues;
    issues.extend(ref_issues(pin.reference.map(String::as_str), "source.ref"));
    match reading.repo {
        Some(repo) if issues.is_empty() => planned(FetchPlan::Git(pinned(pin, repo))),
        _ => refused(issues),
    }
}

/// "Claude Code checks out only that subdirectory", with a partial clone over `https` or SSH.
fn plan_git_subdir(url: &str, path: &str, pin: &Pin<'_>) -> Planned {
    let reading = read_git_address(url, true, "source.url");
    let mut issues = reading.issues;
    issues.extend(repository_path_issues(path, "source.path"));
    issues.extend(ref_issues(pin.reference.map(String::as_str), "source.ref"));
    match reading.repo {
        Some(repo) if issues.is_empty() => planned(FetchPlan::Git(GitFetchPlan {
            subdir: Some(path.to_owned()),
            sparse: true,
            partial_clone: reading.partial_clone,
            ..pinned(pin, repo)
        })),
        _ => refused(issues),
    }
}

/// An `npm` source's fields.
struct NpmSource<'a> {
    package: &'a str,
    version: Option<&'a String>,
    registry: Option<&'a String>,
}

fn plan_npm(source: &NpmSource<'_>, context: &FetchPlanContext) -> Planned {
    let safety = NpmSafety {
        default_registry: context.default_npm_registry.clone(),
    };
    let package = NpmPackageSource {
        package: source.package.to_owned(),
        version: source.version.cloned(),
    };
    let mut issues = npm_package_issues(&package, &safety, "source");
    if let Some(registry) = source.registry {
        issues.extend(npm_registry_issues(registry, &safety, "source.registry"));
    }
    if !issues.is_empty() {
        return refused(issues);
    }
    planned(FetchPlan::Npm(NpmFetchPlan {
        spec: npm_spec_of(source.package, source.version.map(String::as_str)),
        registry: source.registry.cloned(),
        run_install_scripts: false,
        install_dependencies: false,
    }))
}

fn plan_archive(url: &str, sha256: Option<&String>) -> Planned {
    let issues = archive_url_issues(url, "source.url");
    if !issues.is_empty() {
        return refused(issues);
    }
    planned(FetchPlan::Archive(ArchiveFetchPlan {
        url: url.to_owned(),
        sha256: sha256.map(|sha256| sha256.to_lowercase()),
        limits: ARCHIVE_LIMITS,
        // Entry and source headers are merged by the download-headers step; none are known here.
        headers: IndexMap::new(),
        root_depth: RootDepth::TopOrOneDown,
    }))
}

/// "Claude Code runs the command through `sh`, or through `cmd.exe` on Windows, from the user's
/// home directory."
fn plan_command(
    command: &str,
    timeout_seconds: f64,
    mode: source_types::CommandMode,
    context: &FetchPlanContext,
) -> Planned {
    let windows = context.platform == PathPlatform::Windows;
    let links_need_posix = match mode {
        source_types::CommandMode::Copy => LINKS_NEED_POSIX.copy,
        source_types::CommandMode::Link => LINKS_NEED_POSIX.link,
    };
    if windows && links_need_posix {
        return refused(vec![link_on_windows_issue()]);
    }
    planned(FetchPlan::Command(CommandFetchPlan {
        command: command.to_owned(),
        shell: if windows {
            CommandShell::Cmd
        } else {
            CommandShell::Sh
        },
        cwd: CommandCwd::Home,
        timeout_ms: timeout_seconds * 1000.0,
        mode: match mode {
            source_types::CommandMode::Copy => CommandMode::Copy,
            source_types::CommandMode::Link => CommandMode::Link,
        },
    }))
}

/// The planner of each kind (`PLANNERS`), a `match` over the closed union.
fn plan_of(source: &PluginSource, context: &FetchPlanContext) -> Planned {
    match source {
        PluginSource::Relative { path, .. } => plan_relative(path, context),
        PluginSource::Github {
            repo,
            reference,
            sha,
        } => plan_github(
            repo,
            &Pin {
                reference: reference.as_ref(),
                sha: sha.as_ref(),
            },
        ),
        PluginSource::Url {
            url,
            reference,
            sha,
        } => plan_url(
            url,
            &Pin {
                reference: reference.as_ref(),
                sha: sha.as_ref(),
            },
        ),
        PluginSource::GitSubdir {
            url,
            path,
            reference,
            sha,
        } => plan_git_subdir(
            url,
            path,
            &Pin {
                reference: reference.as_ref(),
                sha: sha.as_ref(),
            },
        ),
        PluginSource::Npm {
            package,
            version,
            registry,
        } => plan_npm(
            &NpmSource {
                package,
                version: version.as_ref(),
                registry: registry.as_ref(),
            },
            context,
        ),
        PluginSource::Archive { url, sha256 } => plan_archive(url, sha256.as_ref()),
        PluginSource::Command {
            command,
            timeout_seconds,
            mode,
        } => plan_command(command, *timeout_seconds, *mode, context),
    }
}

/// How `source` is fetched, or why it may not be.
#[must_use]
pub fn plan_plugin_fetch(source: &PluginSource, context: &FetchPlanContext) -> Planned {
    plan_of(source, context)
}

/// Only a source's `kind`, read before the whole source.
#[derive(Deserialize)]
struct KindOnly {
    kind: PluginSourceKind,
}

/// A source kind this build has no planner for (an entry written for a newer Claude Code) is
/// refused with the message the docs print, "Update Claude Code and try again" (plugins/
/// troubleshooting.md, "This plugin uses a source type your Claude Code version does not
/// support"), instead of failing in the planner lookup.
fn plan_given(source: &JsValue, context: &FetchPlanContext) -> Result<Planned, String> {
    let kind = KindOnly::deserialize(source.clone()).map(|only| only.kind);
    if !kind.is_ok_and(|kind| PLUGIN_SOURCE_KINDS.contains(&kind)) {
        return Ok(refused(vec![error("source", UNSUPPORTED_SOURCE_TYPE)]));
    }
    let source = PluginSource::deserialize(source.clone()).map_err(|cause| cause.to_string())?;
    Ok(plan_plugin_fetch(&source, context))
}

crate::registry::exports! {
    "marketplace/fetch-plan" {
        "ARCHIVE_LIMITS" => ok || ARCHIVE_LIMITS;
        "planPluginFetch" => fallible |source: JsValue, context: FetchPlanContext| plan_given(&source, &context);
    }
}
