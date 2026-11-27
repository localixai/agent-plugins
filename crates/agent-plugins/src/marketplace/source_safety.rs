//! Port of `src/marketplace/source-safety.ts`, switched.
//!
//! The checks made before anything is fetched (plugins/marketplace-reference.md, "url plugin
//! source", "git-subdir plugin source", "npm plugin source", "archive plugin source"): pure, they
//! return issues, and a refusal "names the value and the reason". The docs give the rules and, for
//! npm, the categories; the wording of each message is the kit's (spec/judgments.md).
//!
//! The TypeScript's `/i` patterns fold ASCII letters only (no `u` flag), so they are spelt here
//! with both cases or as an ASCII prefix test.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::install_failures::npm_refusal_message;
use super::node_deps_spec::{is_default_registry, package_value_refusal};
use super::semver::is_valid_range;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::blocked_hosts::is_blocked_host;
use crate::format::path_text::{PathPlatform, escapes_as_written};
use crate::js::url::parse;
use crate::js::{Pattern, SPACE_CLASS};
use crate::ports::marketplace::GitRepository;

/// `/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/`.
static SHORTHAND: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$").ok());

fn test(pattern: &Pattern, text: &str) -> bool {
    pattern.as_ref().is_some_and(|regex| regex.is_match(text))
}

/// Whether `text` starts with `prefix`, ASCII letters in either case.
fn starts_with_ascii(text: &str, prefix: &str) -> bool {
    text.get(..prefix.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(prefix))
}

/// The forms of a git URL: `https://`, `http://`, `file://` and `git@` (`/^(?:https?:\/\/|file:\/\/|git@)/i`).
fn is_git_url(text: &str) -> bool {
    ["https://", "http://", "file://", "git@"]
        .iter()
        .any(|prefix| starts_with_ascii(text, prefix))
}

/// Over `https` or SSH Claude Code asks the server for a partial clone (`/^(?:https:\/\/|git@)/i`).
fn is_partial_clone_url(text: &str) -> bool {
    starts_with_ascii(text, "https://") || starts_with_ascii(text, "git@")
}

/// A git address read.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitReading {
    /// The repository, when the address names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<GitRepository>,
    /// Claude Code asks for a partial clone.
    pub partial_clone: bool,
    /// What is wrong.
    pub issues: Vec<PluginIssue>,
}

/// A git address in a `url` or `git-subdir` plugin source, or a `git` marketplace source. `url`
/// "doesn't take `owner/repo` shorthand", nor does a marketplace's `git`; `git-subdir` accepts it
/// as GitHub shorthand.
#[must_use]
pub fn read_git_address(text: &str, shorthand: bool, path: &str) -> GitReading {
    if test(&SHORTHAND, text) {
        return if shorthand {
            GitReading {
                repo: Some(GitRepository::Shorthand {
                    shorthand: text.to_owned(),
                }),
                partial_clone: true,
                issues: vec![],
            }
        } else {
            refused(
                path,
                &format!(
                    "A full git URL is needed here, not owner/repo shorthand (\"{text}\"). Use a github source for a GitHub repository"
                ),
            )
        };
    }
    if !is_git_url(text) {
        return refused(
            path,
            &format!(
                "\"{text}\" is not a git URL: it must start with https://, http://, file:// or git@"
            ),
        );
    }
    GitReading {
        repo: Some(GitRepository::Url {
            url: text.to_owned(),
        }),
        partial_clone: is_partial_clone_url(text),
        issues: vec![],
    }
}

fn refused(path: &str, message: &str) -> GitReading {
    GitReading {
        repo: None,
        partial_clone: false,
        issues: vec![error(path, message)],
    }
}

/// `/^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$/`.
static GITHUB_OWNER: Pattern =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$").ok());
/// `/^[A-Za-z0-9._-]{1,100}$/`.
static GITHUB_REPOSITORY: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._-]{1,100}$").ok());

/// GitHub naming rules for an owner and a repository ("Owner and repo must follow GitHub naming rules").
#[must_use]
pub fn is_github_repository(owner: &str, repo: &str) -> bool {
    test(&GITHUB_OWNER, owner) && test(&GITHUB_REPOSITORY, repo) && repo != "." && repo != ".."
}

/// `github` takes `owner/repo`: one rule for a plugin source, a marketplace source and `marketplace add`.
#[must_use]
pub fn github_repo_issues(repo: &str, path: &str) -> Vec<PluginIssue> {
    let parts: Vec<&str> = repo.split('/').collect();
    let owner = parts.first().copied().unwrap_or_default();
    let name = parts.get(1).copied().unwrap_or_default();
    if parts.len() <= 2 && is_github_repository(owner, name) {
        vec![]
    } else {
        vec![error(
            path,
            &format!("\"{repo}\" is not a GitHub repository in owner/repo form"),
        )]
    }
}

/// A path inside a cloned repository stays there: a `git-subdir` path (the docs say only "the
/// subdirectory that holds the plugin"), and a marketplace's `path` and each of its `sparsePaths`.
#[must_use]
pub fn repository_path_issues(inside: &str, path: &str) -> Vec<PluginIssue> {
    if escapes_as_written(inside, PathPlatform::Windows) {
        vec![error(
            path,
            &format!("The path \"{inside}\" must be inside the repository"),
        )]
    } else {
        vec![]
    }
}

/// What git refuses in a branch or tag name (`git check-ref-format`): a control character or space,
/// `~ ^ : ? * [ \`, `..`, `@{`, `//`, a component starting with `.`, a name starting with `-` (git
/// would read it as an option) or `/`, and one ending in `/`, `.` or `.lock`.
static NOT_A_REF: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"[\p{{Cc}}{SPACE_CLASS}~^:?*\[\\]|\.\.|@\{{|//|/\.|^[-/.]|[/.]$|\.lock$"
    ))
    .ok()
});

/// `ref` is "a branch or tag": one rule for the plugin and the marketplace git sources.
#[must_use]
pub fn ref_issues(reference: Option<&str>, path: &str) -> Vec<PluginIssue> {
    match reference {
        Some(reference) if test(&NOT_A_REF, reference) => vec![error(
            path,
            &format!("\"{reference}\" is not a branch or tag name"),
        )],
        _ => vec![],
    }
}

// ---- npm ----

/// What the npm checks know of the user.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NpmSafety {
    /// The registry URL of the installing user's own npm configuration.
    #[serde(default)]
    pub default_registry: Option<String>,
}

/// What an npm source names: a tarball link, or a package name and the version npm fetches.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NpmSpec {
    /// A tarball link.
    Tarball {
        /// The link.
        #[serde(rename = "tarballUrl")]
        tarball_url: String,
    },
    /// A package and its version.
    Package {
        /// The name.
        package: String,
        /// The version npm fetches.
        version: String,
    },
}

/// `/^https?:\/\//i`.
fn is_tarball(text: &str) -> bool {
    starts_with_ascii(text, "https://") || starts_with_ascii(text, "http://")
}

/// `pkg.indexOf('@', 1)`: an `@` past the first character, which may start a scope.
fn at_past_scope(pkg: &str) -> Option<usize> {
    let first = pkg.chars().next().map_or(0, char::len_utf8);
    Some(pkg.get(first..)?.find('@')? + first)
}

/// `package` as npm reads it: a tarball link, or a name with a version appended after the first `@`
/// past a scope (it wins over `version`), or a bare name whose version is `version`, else `latest`.
/// A package name holds no other `@`, so `x@git+ssh://git@host/x.git` appends `git+ssh://git@host/x.git`.
#[must_use]
pub fn npm_spec_of(pkg: &str, version: Option<&str>) -> NpmSpec {
    if is_tarball(pkg) {
        return NpmSpec::Tarball {
            tarball_url: pkg.to_owned(),
        };
    }
    let at = at_past_scope(pkg);
    let appended = at.and_then(|at| pkg.get(at + 1..)).unwrap_or_default();
    let name = at.and_then(|at| pkg.get(..at)).unwrap_or(pkg);
    NpmSpec::Package {
        package: name.to_owned(),
        version: if appended.is_empty() {
            version.unwrap_or("latest").to_owned()
        } else {
            appended.to_owned()
        },
    }
}

/// npm's naming rule for a package: an optional `@scope/`, URL-safe characters, and no leading `.`,
/// `_` or `-` (a leading `-` would read as an option of the npm client).
static NPM_NAME: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:@[A-Za-z0-9~][A-Za-z0-9_.~-]*/)?[A-Za-z0-9~][A-Za-z0-9_.~-]*$").ok()
});
/// A dist-tag such as `latest` or `beta`: npm requires a tag to be URL-safe.
static DIST_TAG: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9_.-]*$").ok());

/// Why a version is refused: "a version, a semver range, or a dist-tag" is all the field takes, so a
/// git, folder, alias or tarball specifier after the name is refused with the reason of its kind.
fn version_refusal(version: &str, safety: &NpmSafety) -> Option<&'static str> {
    if let Some(kind) = package_value_refusal(version, safety.default_registry.as_deref()) {
        return Some(kind);
    }
    if is_valid_range(version) || test(&DIST_TAG, version) {
        None
    } else {
        Some("its version is not a version, a semver range or a dist-tag")
    }
}

/// An npm source's package and version.
#[derive(Clone, Debug, Deserialize)]
pub struct NpmPackageSource {
    /// `package`.
    pub package: String,
    /// `version`.
    #[serde(default)]
    pub version: Option<String>,
}

/// Whether the `version` field, not text appended to `package`, supplied the version.
fn version_from_field(source: &NpmPackageSource) -> bool {
    let at = at_past_scope(&source.package);
    source.version.is_some() && at.is_none_or(|at| at + 1 == source.package.len())
}

struct Refusal {
    field: &'static str,
    value: String,
    reason: &'static str,
}

/// The refusal of the name and the version a package value names.
fn spec_refusal(source: &NpmPackageSource, safety: &NpmSafety) -> Option<Refusal> {
    let NpmSpec::Package { package, version } =
        npm_spec_of(&source.package, source.version.as_deref())
    else {
        return None;
    };
    let named = |reason| Refusal {
        field: "package",
        value: source.package.clone(),
        reason,
    };
    if !test(&NPM_NAME, &package) {
        return Some(named("it is not an npm package name"));
    }
    let reason = version_refusal(&version, safety)?;
    Some(if version_from_field(source) {
        Refusal {
            field: "version",
            value: version,
            reason,
        }
    } else {
        named(reason)
    })
}

/// "Claude Code checks the `package` value before fetching anything." The kinds of value and what
/// each means are the registry-only rule of `node-deps-spec`, shared with the dependency install;
/// the same rule judges the version a name carries or the `version` field gives, so `name@<git
/// address>` is refused as the git address is. `path` is the source's (`source`).
#[must_use]
pub fn npm_package_issues(
    source: &NpmPackageSource,
    safety: &NpmSafety,
    path: &str,
) -> Vec<PluginIssue> {
    let refusal = match package_value_refusal(&source.package, safety.default_registry.as_deref()) {
        None => spec_refusal(source, safety),
        Some(whole) => Some(Refusal {
            field: "package",
            value: source.package.clone(),
            reason: whole,
        }),
    };
    refusal.map_or_else(Vec::new, |refusal| {
        vec![error(
            &format!("{path}.{}", refusal.field),
            &npm_refusal_message(&refusal.value, refusal.reason),
        )]
    })
}

/// The `registry` URL "must use `https` unless it is the installing user's own default npm registry".
#[must_use]
pub fn npm_registry_issues(registry: &str, safety: &NpmSafety, path: &str) -> Vec<PluginIssue> {
    if is_default_registry(registry, safety.default_registry.as_deref())
        || starts_with_ascii(registry, "https://")
    {
        return vec![];
    }
    vec![error(
        path,
        &format!(
            "The npm registry \"{registry}\" must use https unless it is your own default npm registry"
        ),
    )]
}

// ---- archive ----

fn host_of(link: &str) -> Option<String> {
    parse(link).map(|url| url.hostname.to_lowercase())
}

/// `url` "must use `https://` and can't point at a loopback, link-local, or cloud-metadata host".
#[must_use]
pub fn archive_url_issues(url: &str, path: &str) -> Vec<PluginIssue> {
    let Some(host) = host_of(url) else {
        return vec![error(path, &format!("\"{url}\" is not a URL"))];
    };
    if !starts_with_ascii(url, "https://") {
        return vec![error(
            path,
            &format!("The archive URL \"{url}\" must use https://"),
        )];
    }
    if is_blocked_host(&host) {
        vec![error(
            path,
            &format!(
                "The archive URL \"{url}\" points at a loopback, link-local or cloud-metadata host"
            ),
        )]
    } else {
        vec![]
    }
}

crate::registry::exports! {
    "marketplace/source-safety" {
        "readGitAddress" => ok |text: String, shorthand: bool, path: String| read_git_address(&text, shorthand, &path);
        "isGitHubRepository" => ok |owner: String, repo: String| is_github_repository(&owner, &repo);
    }
}
