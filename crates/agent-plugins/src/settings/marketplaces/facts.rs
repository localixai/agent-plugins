//! Ported from `src/settings/marketplaces/facts.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a matcher asks of a source or an entry, the same shape for every kind so a matcher never
//! branches on the kind of the thing it compares (rule K2).
//!
//! The package's `host` member is a function of `wide`; here it is [`HostOf`], what the function
//! reads, and [`Facts::host`] answers it.

use super::git_url::{RepoId, canonical_repo, github_repo};
use super::host_of::{host_of_git, host_of_url};
use super::source_kinds::{SourceKind, by_kind};
use crate::format::canonical_json::canonical_json;
use crate::js::JsValue;

/// The source's kind; `Claudeai` for a marketplace hosted on claude.ai, which has no source
/// object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactsKind {
    /// A source object's kind.
    Source(SourceKind),
    /// A marketplace hosted on claude.ai.
    Claudeai,
}

/// How the host a `hostPattern` is matched against is read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostOf {
    /// No host.
    None,
    /// Always this host.
    Fixed(&'static str),
    /// The host of a `git` source's URL; the blocklist reads `wide` forms.
    Git(String),
    /// The host of a `url` source.
    Url(String),
}

/// What a matcher asks of a source or an entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Facts {
    /// The kind.
    pub kind: FactsKind,
    /// Whether it names a place a marketplace comes from; `hostPattern`, `pathPattern`,
    /// `skills-dir` do not.
    pub concrete: bool,
    /// The text an exact comparison uses: the repo, url, path, package or name.
    pub locator: String,
    /// The `ref` as written (`undefined` when absent).
    pub ref_: JsValue,
    /// The subdirectory of a `github` or `git` source (its `path` field).
    pub subpath: JsValue,
    /// The repository identity of a source that clones one (`github`, `git`); `url` has one for
    /// entries.
    pub repo: Option<RepoId>,
    /// Whether the source is cloned from a repository, not fetched or read from a path.
    pub cloned: bool,
    /// The path of a `file` or `directory` source.
    pub file_path: Option<String>,
    /// The `registry` of an `npm` source.
    pub registry: JsValue,
    /// A `settings` source's inline catalog, in a form two identical catalogs share (key order
    /// aside).
    pub catalog: Option<String>,
    /// The host a `hostPattern` is matched against.
    pub host_of: HostOf,
}

impl Facts {
    /// The host a `hostPattern` is matched against; the blocklist reads `wide` forms of git
    /// addresses.
    #[must_use]
    pub fn host(&self, wide: bool) -> Option<String> {
        match &self.host_of {
            HostOf::None => None,
            HostOf::Fixed(host) => Some((*host).to_owned()),
            HostOf::Git(url) => host_of_git(url, wide),
            HostOf::Url(url) => host_of_url(url),
        }
    }
}

const fn base(kind: SourceKind) -> Facts {
    Facts {
        kind: FactsKind::Source(kind),
        concrete: true,
        locator: String::new(),
        ref_: JsValue::Undefined,
        subpath: JsValue::Undefined,
        repo: None,
        cloned: false,
        file_path: None,
        registry: JsValue::Undefined,
        catalog: None,
        host_of: HostOf::None,
    }
}

fn text(source: &JsValue, key: &str) -> String {
    source
        .get(key)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn member(source: &JsValue, key: &str) -> JsValue {
    source.get(key).cloned().unwrap_or_default()
}

fn not_a_place(kind: SourceKind, locator: String) -> Facts {
    Facts {
        concrete: false,
        locator,
        ..base(kind)
    }
}

fn facts(kind: SourceKind, source: &JsValue) -> Facts {
    match kind {
        SourceKind::Github => {
            let repo = text(source, "repo");
            Facts {
                repo: Some(github_repo(&repo)),
                locator: repo,
                ref_: member(source, "ref"),
                subpath: member(source, "path"),
                cloned: true,
                host_of: HostOf::Fixed("github.com"),
                ..base(kind)
            }
        }
        SourceKind::Git => {
            let url = text(source, "url");
            Facts {
                repo: canonical_repo(&url),
                ref_: member(source, "ref"),
                subpath: member(source, "path"),
                cloned: true,
                host_of: HostOf::Git(url.clone()),
                locator: url,
                ..base(kind)
            }
        }
        SourceKind::Url => {
            let url = text(source, "url");
            Facts {
                repo: canonical_repo(&url),
                host_of: HostOf::Url(url.clone()),
                locator: url,
                ..base(kind)
            }
        }
        SourceKind::File | SourceKind::Directory => {
            let path = text(source, "path");
            Facts {
                file_path: Some(path.clone()),
                locator: path,
                ..base(kind)
            }
        }
        // The docs' table has no row for these two; the SDK types them (spec/judgments.md).
        SourceKind::Npm => Facts {
            locator: text(source, "package"),
            registry: member(source, "registry"),
            ..base(kind)
        },
        SourceKind::Settings => Facts {
            locator: text(source, "name"),
            catalog: Some(canonical_json(&member(source, "plugins"))),
            ..base(kind)
        },
        SourceKind::HostPattern => not_a_place(kind, text(source, "hostPattern")),
        SourceKind::PathPattern => not_a_place(kind, text(source, "pathPattern")),
        SourceKind::SkillsDir => not_a_place(kind, String::new()),
    }
}

/// The facts of a source object; `None` for a kind no source has.
#[must_use]
pub fn facts_of(source: &JsValue) -> Option<Facts> {
    by_kind(facts, source)
}

/// A marketplace hosted on claude.ai (plugins/org.md, "Marketplaces hosted on claude.ai"; Agent
/// Plugins Extended §14.8.2 item 1): no source object types it, so it is a kind of its own whose
/// only fact is its host, `claude.ai`. An exact entry of any kind never equals it and a
/// `pathPattern` finds no path, so only a `hostPattern` names it, on either list.
#[must_use]
pub fn claude_ai_facts() -> Facts {
    Facts {
        kind: FactsKind::Claudeai,
        locator: "claude.ai".to_owned(),
        host_of: HostOf::Fixed("claude.ai"),
        ..base(SourceKind::SkillsDir)
    }
}
