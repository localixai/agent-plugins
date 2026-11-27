//! Port of `src/settings/marketplaces/match-source.ts`, switched.
//!
//! The matcher both marketplace lists share (settings-reference.md, `strictKnownMarketplaces`
//! "Allowed source types", "Owner wildcards", "Exact matching"; plugins/org.md "How entries
//! match", "Blocklist with `blockedMarketplaces`"). The allowlist matches by exact entry, owner
//! wildcard or regular expression; the blocklist matches the same entries more widely: canonical
//! git URLs, `github` and `git` as one, owner comparison in any case, no `ref` or `path` meaning
//! every one.

use serde::{Deserialize, Serialize};

use super::facts::{Facts, claude_ai_facts, facts_of};
use super::git_url::same_repo;
use super::source_kinds::{SourceKind, by_kind_with};
use crate::js::regexp::RegExp;
use crate::js::{JsValue, is_whitespace};

/// Which list an entry is read as.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListKind {
    /// `strictKnownMarketplaces`.
    Strict,
    /// `blockedMarketplaces`.
    Blocked,
}

/// `<owner>/*`: "Only the whole repository-name position can be a wildcard":
/// `/^([^/*\s]+)\/\*$/`.
fn owner_of(repo: &str) -> Option<&str> {
    let owner = repo.strip_suffix("/*")?;
    (!owner.is_empty()
        && !owner
            .chars()
            .any(|character| matches!(character, '/' | '*') || is_whitespace(character)))
    .then_some(owner)
}

/// A name with `*` that is not a valid wildcard is "ignored as invalid, so it matches nothing".
fn is_broken_wildcard(repo: &str) -> bool {
    repo.contains('*') && owner_of(repo).is_none()
}

/// Regex flavour is JavaScript's (spec/judgments.md); a pattern that does not compile matches
/// nothing.
fn matches_pattern(pattern: &str, text: Option<&str>) -> bool {
    let Some(text) = text else {
        return false;
    };
    RegExp::new(pattern, "").is_ok_and(|compiled| compiled.test_shared(text))
}

/// `repo`/`url`, `ref` and `path` "must match exactly, or both must be undefined".
fn same_exactly(own: &Facts, source: &Facts) -> bool {
    own.kind == source.kind
        && own.locator == source.locator
        && own.ref_ == source.ref_
        && own.subpath == source.subpath
}

fn exact(own: &Facts, source: &Facts) -> bool {
    source.concrete && same_exactly(own, source)
}

fn pinned(wanted: &JsValue, actual: &JsValue) -> bool {
    wanted.is_undefined() || wanted == actual
}

/// The two-segment `owner/name` of a repository on github.com, as its owner and name.
fn github_owner(source: &Facts) -> Option<String> {
    let path = source
        .repo
        .as_ref()
        .filter(|repo| repo.host == "github.com")
        .map_or("", |repo| repo.path.as_str());
    let mut parts = path.split('/');
    let owner = parts.next()?;
    let name = parts.next()?;
    (parts.next().is_none() && !name.contains('*')).then(|| owner.to_owned())
}

/// Owner-wildcard entry, allowlist: case-sensitive owner, exact `ref`, any `path` unless pinned.
fn strict_wildcard(owner: &str, own: &Facts, source: &Facts) -> bool {
    let mut parts = source.locator.split('/');
    let source_owner = parts.next();
    let name = parts.next();
    source.kind == own.kind
        && source_owner == Some(owner)
        && name.is_some_and(|name| !name.is_empty() && !name.contains('*'))
        && parts.next().is_none()
        && own.ref_ == source.ref_
        && pinned(&own.subpath, &source.subpath)
}

fn entry_repo(entry: &JsValue) -> &str {
    entry
        .get("repo")
        .and_then(JsValue::as_str)
        .unwrap_or_default()
}

fn strict_github(entry: &JsValue, own: &Facts, source: &Facts) -> bool {
    let repo = entry_repo(entry);
    if is_broken_wildcard(repo) {
        return false;
    }
    let owner = owner_of(repo);
    if !source.concrete {
        return false;
    }
    owner.map_or_else(
        || same_exactly(own, source),
        |owner| strict_wildcard(owner, own, source),
    )
}

/// Blocklist: an entry without `ref` or `path` blocks every one; a pinned value must be the same.
fn blocked_repo_scope(own: &Facts, source: &Facts) -> bool {
    pinned(&own.ref_, &source.ref_) && pinned(&own.subpath, &source.subpath)
}

fn blocked_wildcard(owner: &str, own: &Facts, source: &Facts) -> bool {
    github_owner(source)
        .is_some_and(|source_owner| source_owner.to_lowercase() == owner.to_lowercase())
        && blocked_repo_scope(own, source)
}

fn blocked_github(entry: &JsValue, own: &Facts, source: &Facts) -> bool {
    let repo = entry_repo(entry);
    if is_broken_wildcard(repo) || !source.cloned {
        return false;
    }
    if let Some(owner) = owner_of(repo) {
        return blocked_wildcard(owner, own, source);
    }
    blocked_repo_scope(own, source) && same_repo_of(own, source)
}

fn same_repo_of(own: &Facts, source: &Facts) -> bool {
    match (&own.repo, &source.repo) {
        (Some(first), Some(second)) => same_repo(first, second),
        _ => false,
    }
}

fn blocked_git(own: &Facts, source: &Facts) -> bool {
    source.cloned && blocked_repo_scope(own, source) && same_repo_of(own, source)
}

/// A `url` entry blocks that url, and a bare repository URL that is cloned (`.git` and `#ref`
/// ignored).
fn blocked_url(own: &Facts, source: &Facts) -> bool {
    if source.cloned {
        same_repo_of(own, source)
    } else {
        source.concrete && same_exactly(own, source)
    }
}

fn host_match(wide: bool, entry: &JsValue, source: &Facts) -> bool {
    let pattern = entry
        .get("hostPattern")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    source.concrete && matches_pattern(pattern, source.host(wide).as_deref())
}

fn path_match(entry: &JsValue, source: &Facts) -> bool {
    let pattern = entry
        .get("pathPattern")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    source.concrete && matches_pattern(pattern, source.file_path.as_deref())
}

/// An inline marketplace on the allowlist "allows an entry with the same `name` and identical
/// `plugins`" (marketplace-reference.md, "Marketplace sources"); the blocklist "blocks the same
/// `name`", whatever its catalog.
fn strict_settings(own: &Facts, source: &Facts) -> bool {
    exact(own, source) && own.catalog == source.catalog
}

/// The matchers both lists share (`COMMON`), then each list's own (`STRICT`, `BLOCKED`).
fn matches(list: ListKind, kind: SourceKind, entry: &JsValue, facts: &(Facts, Facts)) -> bool {
    let (own, source) = facts;
    match (kind, list) {
        (SourceKind::File | SourceKind::Directory, _) => exact(own, source),
        // No `npm` marketplace registers, so an `npm` entry matches nothing on either list
        // (§14.8.3).
        (SourceKind::Npm | SourceKind::SkillsDir, _) => false,
        (SourceKind::PathPattern, _) => path_match(entry, source),
        (SourceKind::Settings, ListKind::Strict) => strict_settings(own, source),
        (SourceKind::Github, ListKind::Strict) => strict_github(entry, own, source),
        (SourceKind::Git | SourceKind::Url, ListKind::Strict)
        | (SourceKind::Settings, ListKind::Blocked) => exact(own, source),
        (SourceKind::HostPattern, ListKind::Strict) => host_match(false, entry, source),
        (SourceKind::Github, ListKind::Blocked) => blocked_github(entry, own, source),
        (SourceKind::Git, ListKind::Blocked) => blocked_git(own, source),
        (SourceKind::Url, ListKind::Blocked) => blocked_url(own, source),
        (SourceKind::HostPattern, ListKind::Blocked) => host_match(true, entry, source),
    }
}

fn match_with(list: ListKind, entry: &JsValue, source: Option<Facts>) -> bool {
    let (Some(own), Some(source)) = (facts_of(entry), source) else {
        return false;
    };
    by_kind_with(
        |kind, entry, facts| matches(list, kind, entry, facts),
        entry,
        &(own, source),
    )
    .unwrap_or(false)
}

/// Whether a list entry matches the source of a marketplace. `skills-dir` entries match no
/// marketplace; they only decide whether skills-directory plugins load (see `gate.ts`).
#[must_use]
pub fn match_marketplace_source(list: ListKind, entry: &JsValue, source: &JsValue) -> bool {
    match_with(list, entry, facts_of(source))
}

/// Whether a list entry matches a marketplace hosted on claude.ai, by its host `claude.ai`: only a
/// `hostPattern` entry can (plugins/org.md, "Marketplaces hosted on claude.ai"). Whose the
/// marketplace is decides the allowlist's admission (`MarketplaceGate.checkClaudeAi`), not the
/// match.
#[must_use]
pub fn match_claude_ai_marketplace(list: ListKind, entry: &JsValue) -> bool {
    match_with(list, entry, Some(claude_ai_facts()))
}

crate::registry::exports! {
    "settings/marketplaces/match-source" {
        "matchMarketplaceSource" => ok |list: ListKind, entry: JsValue, source: JsValue|
            match_marketplace_source(list, &entry, &source);
        "matchClaudeAiMarketplace" => ok |list: ListKind, entry: JsValue|
            match_claude_ai_marketplace(list, &entry);
    }
}

#[cfg(test)]
mod tests {
    use super::owner_of;

    #[test]
    fn an_owner_wildcard_names_the_whole_name() {
        assert_eq!(owner_of("acme/*"), Some("acme"));
        assert_eq!(owner_of("acme/x*"), None);
        assert_eq!(owner_of("a b/*"), None);
        assert_eq!(owner_of("/*"), None);
    }
}
