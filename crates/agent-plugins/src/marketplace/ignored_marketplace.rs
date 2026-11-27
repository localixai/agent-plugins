//! Port of `src/marketplace/ignored-marketplace.ts`, switched.
//!
//! `Marketplace "<name>" is added but ignored` (plugins/troubleshooting.md): an entry of
//! `known_marketplaces.json` that fails a check made every time the file is read, so the
//! marketplace "and the plugins installed from it stop loading". The checks are the ones the
//! section lists: the location, a git URL, an http(s) URL, a match with the `extraKnownMarketplaces`
//! entry of user or managed settings, and the name. Which location is on a network drive is the
//! host's to probe (an effect); everything else is decided here.
//!
//! Judgments (spec/judgments.md): the docs quote the line and the fix only for the location check,
//! so the fix for the others is the kit's wording; the name is refused when `loadNameIssues` finds an
//! issue with the entry's own source (another spelling of a reserved name, a member name of
//! JavaScript objects), and a reserved name refused only for its source is the untrusted-source case
//! (`untrustedSource`), not this one; a git URL is unusable when `readGitAddress` refuses
//! it or its host is loopback, link-local or a cloud-metadata address (`isBlockedHost`); the Errors
//! tab line ends with the reason's period.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::install_errors::ErrorsTabEntry;
use super::known_marketplaces::{KnownMarketplace, same_marketplace_source};
use super::source_safety::read_git_address;
use crate::format::marketplace::blocked_hosts::is_blocked_host;
use crate::format::marketplace::marketplace_names::{NameOrigin, load_name_issues};
use crate::format::marketplace::types::MarketplaceSource;
use crate::js::regexp::RegExp;
use crate::js::value::to_js_value;

/// `network`: on a network drive; `unchecked`: it could not be examined; `local`: a path on this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Location {
    /// A path on this computer.
    Local,
    /// On a network drive.
    Network,
    /// It could not be examined.
    Unchecked,
}

/// What the host found out about the entry's location (the clone's folder, or the local path it was added from).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct IgnoreChecks {
    /// Where the location is.
    pub location: Location,
    /// The `extraKnownMarketplaces` source of the same name in user or managed settings (not a repository's).
    #[serde(default)]
    pub declared: Option<MarketplaceSource>,
}

/// Why an entry is ignored.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IgnoredReason {
    /// The sentence after `is added but ignored`; absent when only the debug log has the reason.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sentence: Option<String>,
    /// What to do.
    pub fix: String,
}

const LOCATION_PROBLEM: &str =
    "is on a network drive, has \".\" or \"..\" in its path, or couldn't be checked";
const LOCATION_FIX: &str = "Re-add the marketplace (one added from a folder or file must be re-added from a copy on this computer), or, to trust a folder on a network drive, declare it under extraKnownMarketplaces in user or managed settings.";

/// A JavaScript regular expression literal, compiled once (`None` never happens: a test compiles each).
type Literal = LazyLock<Option<RegExp>>;

static DOT_SEGMENT: Literal =
    LazyLock::new(|| RegExp::new(r"(?:^|[\\/])\.{1,2}(?:[\\/]|$)", "").ok());
static HTTP_ADDRESS: Literal = LazyLock::new(|| RegExp::new(r"^https?:\/\/[^\s/?#]+", "i").ok());
static HOST_OF_URL: Literal =
    LazyLock::new(|| RegExp::new(r"^https?:\/\/(?:[^@/]*@)?(\[[^\]]+\]|[^/:?#]+)", "i").ok());

fn test(pattern: &Literal, text: &str) -> bool {
    pattern
        .as_ref()
        .is_some_and(|pattern| pattern.test_shared(text))
}

/// The sources the registry keeps no copy of: the path it recorded is the one you gave
/// (`'path' in source` without `url`, `repo` or `pathPattern`: a `file` or `directory` source).
const fn is_local_path(source: &MarketplaceSource) -> bool {
    matches!(
        source,
        MarketplaceSource::File { .. } | MarketplaceSource::Directory { .. }
    )
}

fn location_reason(entry: &KnownMarketplace, location: Location) -> Option<IgnoredReason> {
    let unsafe_location =
        location != Location::Local || test(&DOT_SEGMENT, &entry.install_location);
    if !unsafe_location {
        return None;
    }
    let subject = if is_local_path(&entry.source) {
        "The folder or file it was added from"
    } else {
        "Its location"
    };
    Some(IgnoredReason {
        sentence: Some(format!("{subject} {LOCATION_PROBLEM}")),
        fix: LOCATION_FIX.to_owned(),
    })
}

fn git_url_problem(url: &str) -> Option<String> {
    if let Some(issue) = read_git_address(url, false, "url")
        .issues
        .into_iter()
        .next()
    {
        return Some(issue.message);
    }
    let host = HOST_OF_URL
        .as_ref()
        .and_then(|pattern| pattern.clone().exec(url))
        .and_then(|found| found.captures.into_iter().nth(1).flatten())?;
    is_blocked_host(&host)
        .then(|| format!("\"{host}\" is a loopback, link-local or cloud-metadata address"))
}

fn source_reason(name: &str, source: &MarketplaceSource) -> Option<IgnoredReason> {
    let remove = format!(
        "Remove the marketplace with claude plugin marketplace remove {name}, then add it again from a supported source."
    );
    // `'url' in source && 'path' in source`: a `git` source.
    let problem = match source {
        MarketplaceSource::Git { url, .. } => git_url_problem(url),
        MarketplaceSource::Url { .. }
        | MarketplaceSource::Github { .. }
        | MarketplaceSource::Npm { .. }
        | MarketplaceSource::File { .. }
        | MarketplaceSource::Directory { .. }
        | MarketplaceSource::Settings { .. }
        | MarketplaceSource::SkillsDir {}
        | MarketplaceSource::HostPattern { .. }
        | MarketplaceSource::PathPattern { .. } => None,
    };
    if let Some(problem) = problem {
        return Some(IgnoredReason {
            sentence: Some(format!("Its git URL can't be used: {problem}")),
            fix: remove,
        });
    }
    // `'url' in source && !('path' in source)`: a `url` source.
    let bad_url = matches!(source, MarketplaceSource::Url { url, .. } if !test(&HTTP_ADDRESS, url));
    bad_url.then(|| IgnoredReason {
        sentence: Some("Its URL can't be read as an https:// or http:// address".to_owned()),
        fix: remove,
    })
}

fn mismatch_reason(name: &str) -> IgnoredReason {
    IgnoredReason {
        sentence: Some(
            "Its source doesn't match its extraKnownMarketplaces entry in user or managed settings"
                .to_owned(),
        ),
        fix: format!(
            "Re-add the marketplace from the declared source, or change the declaration; claude plugin marketplace remove {name} clears the entry."
        ),
    }
}

/// An official name's own source: the one place `loadNameIssues` lets a reserved name load from.
fn official_origin() -> MarketplaceSource {
    MarketplaceSource::Github {
        repo: "anthropics/claude-plugins-official".to_owned(),
        path: ".claude-plugin/marketplace.json".to_owned(),
        reference: None,
        sparse_paths: None,
        skip_lfs: None,
    }
}

fn origin_of(source: &MarketplaceSource) -> NameOrigin {
    NameOrigin {
        origin: to_js_value(source).unwrap_or_default(),
        hosted_on_claude_ai: None,
    }
}

fn refused(name: &str, origin: &MarketplaceSource) -> bool {
    !load_name_issues(name, &origin_of(origin)).is_empty()
}

/// A reserved name whose registered source is not an `anthropics` GitHub repository: the name is
/// refused with the entry's source and accepted with an official one, so the source is the only
/// thing wrong with it. Reserved names are "re-checked every time a marketplace loads or refreshes".
fn refused_for_source_only(name: &str, source: &MarketplaceSource) -> bool {
    refused(name, source) && !refused(name, &official_origin())
}

/// `Marketplace "<name>" is registered from an untrusted source: ...` for an entry registered under a
/// reserved name from a source that is not an `anthropics` GitHub repository: it and its plugins
/// stop loading. `undefined` for any other entry. The host checks this on every load beside
/// `ignoredReason`, which leaves this case to it.
#[must_use]
pub fn untrusted_source(name: &str, entry: &KnownMarketplace) -> Option<String> {
    refused_for_source_only(name, &entry.source)
        .then(|| untrusted_source_message(name, Some(&entry.source)))
}

/// Why an entry is ignored, or `undefined` when it loads: the first check that fails, in the order
/// the docs list them (location, git URL or URL, settings declaration, name).
#[must_use]
pub fn ignored_reason(
    name: &str,
    entry: &KnownMarketplace,
    checks: &IgnoreChecks,
) -> Option<IgnoredReason> {
    let mismatched = checks
        .declared
        .as_ref()
        .is_some_and(|declared| !same_marketplace_source(&entry.source, declared));
    let name_refused =
        refused(name, &entry.source) && !refused_for_source_only(name, &entry.source);
    location_reason(entry, checks.location)
        .or_else(|| source_reason(name, &entry.source))
        .or_else(|| mismatched.then(|| mismatch_reason(name)))
        .or_else(|| {
            name_refused.then(|| IgnoredReason {
                sentence: None,
                fix: format!("Remove it with: claude plugin marketplace remove {name}"),
            })
        })
}

/// Where the line shows: `claude plugin list` (one line, name bare) or the `/plugin` Errors tab (name quoted, fix beneath).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum IgnoredSurface {
    /// `claude plugin list`.
    #[serde(rename = "list")]
    List,
    /// The `/plugin` Errors tab.
    #[serde(rename = "errors-tab")]
    ErrorsTab,
}

/// The line of a surface: text, or an Errors tab entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum IgnoredLine {
    /// `claude plugin list`'s text.
    Text(String),
    /// The Errors tab entry.
    Entry(ErrorsTabEntry),
}

fn tail(reason: &IgnoredReason) -> String {
    reason.sentence.as_ref().map_or_else(
        || " (see the debug log)".to_owned(),
        |sentence| format!(". {sentence}."),
    )
}

/// The line `claude plugin list` prints (text), or the Errors tab entry (message and guidance).
#[must_use]
pub fn ignored_line(name: &str, reason: &IgnoredReason, surface: IgnoredSurface) -> IgnoredLine {
    match surface {
        IgnoredSurface::List => IgnoredLine::Text(if reason.sentence.is_none() {
            format!("Marketplace {name} is added but ignored (see the debug log)")
        } else {
            format!(
                "Marketplace {name} is added but ignored{} {}",
                tail(reason),
                reason.fix
            )
        }),
        IgnoredSurface::ErrorsTab => IgnoredLine::Entry(ErrorsTabEntry {
            message: format!(
                "Marketplace \"{name}\" is added but ignored{}",
                tail(reason)
            ),
            guidance: reason.fix.clone(),
        }),
    }
}

fn github_refusal(name: &str) -> String {
    format!(
        "The name '{name}' is reserved for official Anthropic marketplaces. Only repositories from 'github.com/anthropics/' can use this name."
    )
}

/// `Marketplace "<name>" is registered from an untrusted source: ...` (troubleshooting.md): a
/// marketplace registered under a reserved name whose source is not an `anthropics` GitHub
/// repository stops loading with its plugins. The middle sentence is the name refusal for that
/// source (Agent Plugins Extended §13.20 item 2): for a source that is not GitHub or git it reads
/// `can only be used with GitHub sources from the 'anthropics' organization`. Without a source, the
/// GitHub sentence.
#[must_use]
pub fn untrusted_source_message(name: &str, source: Option<&MarketplaceSource>) -> String {
    let reserved = format!("The name '{name}' is reserved");
    let refusal = source.and_then(|source| {
        load_name_issues(name, &origin_of(source))
            .into_iter()
            .find(|issue| issue.message.starts_with(&reserved))
            .map(|issue| issue.message)
    });
    format!(
        "Marketplace \"{name}\" is registered from an untrusted source: {} To fix it, remove the marketplace and re-add it from the official source.",
        refusal.unwrap_or_else(|| github_refusal(name))
    )
}

crate::registry::exports! {
    "marketplace/ignored-marketplace" {
        "untrustedSource" => ok |name: String, entry: KnownMarketplace| untrusted_source(&name, &entry);
        "ignoredReason" => ok |name: String, entry: KnownMarketplace, checks: IgnoreChecks| ignored_reason(&name, &entry, &checks);
        "ignoredLine" => ok |name: String, reason: IgnoredReason, surface: IgnoredSurface| ignored_line(&name, &reason, surface);
        "untrustedSourceMessage" => ok |name: String, source: Option<MarketplaceSource>| untrusted_source_message(&name, source.as_ref());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_literal_compiles() {
        assert!(DOT_SEGMENT.is_some() && HTTP_ADDRESS.is_some() && HOST_OF_URL.is_some());
    }

    #[test]
    fn a_dot_segment_is_unsafe() {
        assert!(test(&DOT_SEGMENT, "/a/../b"));
        assert!(!test(&DOT_SEGMENT, "/a/.b"));
    }
}
