//! Port of `src/settings/marketplaces/git-url.ts`, switched.
//!
//! Canonical identity of a git repository address, the comparison `blockedMarketplaces` makes
//! ("Git URLs are canonicalized, so the `git@` and `https://` forms, `.git` suffixes, and trailing
//! slashes of one `github.com` repository all match the same entry", plugins/org.md).

use serde::{Deserialize, Serialize};

use crate::js::url::parse;
use crate::js::{code_unit_length, is_whitespace};

/// A repository as the blocklist compares it: the host (lower case) and the path on it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoId {
    /// The host, in lower case.
    pub host: String,
    /// The path on it.
    pub path: String,
}

/// `https://`, `ssh://` and the like: `^[A-Za-z][A-Za-z0-9+.-]*:\/\/`.
#[must_use]
pub fn is_scheme_url(address: &str) -> bool {
    address
        .chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && address.split_once("://").is_some_and(|(scheme, _)| {
            scheme.chars().all(|character| {
                character.is_ascii_alphanumeric() || matches!(character, '+' | '.' | '-')
            })
        })
}

fn is_line_terminator(character: char) -> bool {
    matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

/// `([^@:/\s]+):(?!\/\/)(.*)$` from the start of the text.
fn host_and_path(text: &str) -> Option<(&str, &str)> {
    let not_host =
        |character: char| matches!(character, '@' | ':' | '/') || is_whitespace(character);
    let end = text.find(not_host).unwrap_or(text.len());
    let (host, rest) = text.split_at(end);
    let path = rest.strip_prefix(':')?;
    (!host.is_empty() && !path.starts_with("//") && !path.contains(is_line_terminator))
        .then_some((host, path))
}

/// `[user@]host:path`, git's SSH address without a scheme (judgment: the user part is optional here):
/// `^(?:[^@/\s]+@)?([^@:/\s]+):(?!\/\/)(.*)$`. The host and the path; `None` when it does not match.
fn scp_address(address: &str) -> Option<(&str, &str)> {
    let not_user = |character: char| matches!(character, '@' | '/') || is_whitespace(character);
    // The user part, when `[^@/\s]+@` opens the address; the regex tries it first.
    let after_user = address.find(not_user).and_then(|end| {
        let (user, rest) = address.split_at(end);
        if user.is_empty() {
            None
        } else {
            rest.strip_prefix('@')
        }
    });
    after_user
        .and_then(host_and_path)
        .or_else(|| host_and_path(address))
}

/// The path without a `#ref`, trailing slashes and a `.git` suffix.
fn tidy_path(path: &str) -> String {
    let without_fragment = path.split('#').next().unwrap_or("");
    let without_slashes = without_fragment.trim_end_matches('/');
    let without_suffix = without_slashes
        .strip_suffix(".git")
        .unwrap_or(without_slashes);
    without_suffix.trim_end_matches('/').to_owned()
}

fn parse_scheme_url(text: &str) -> Option<RepoId> {
    let parsed = parse(text)?;
    Some(RepoId {
        host: parsed.hostname.to_lowercase(),
        path: tidy_path(parsed.pathname.trim_start_matches('/')),
    })
}

/// A repository address as one identity. Scheme, user and port do not count (judgment: `https://`
/// and `ssh://` reach the same repository, and the blocklist is the wider list), the host is lower
/// case, and the path keeps its case (the docs make only an `owner/*` owner case-insensitive). An
/// address with no host, such as a local path, is its own identity under the empty host.
#[must_use]
pub fn canonical_repo(address: &str) -> Option<RepoId> {
    if is_scheme_url(address) {
        return parse_scheme_url(address);
    }
    let (host, path) = scp_address(address).unwrap_or(("", ""));
    // One letter before the colon is a Windows drive, which git reads as a path.
    if code_unit_length(host) > 1 {
        return Some(RepoId {
            host: host.to_lowercase(),
            path: tidy_path(path.trim_start_matches('/')),
        });
    }
    Some(RepoId {
        host: String::new(),
        path: tidy_path(address),
    })
}

/// `owner/repo` of a `github` source as a repository on github.com.
#[must_use]
pub fn github_repo(repo: &str) -> RepoId {
    RepoId {
        host: "github.com".to_owned(),
        path: tidy_path(repo),
    }
}

/// Whether two addresses are one repository.
#[must_use]
pub fn same_repo(first: &RepoId, second: &RepoId) -> bool {
    first.host == second.host && first.path == second.path
}

crate::registry::exports! {
    "settings/marketplaces/git-url" {
        "canonicalRepo" => ok |address: String| canonical_repo(&address);
    }
}
