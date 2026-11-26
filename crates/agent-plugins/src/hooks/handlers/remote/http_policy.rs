//! Port of `src/hooks/handlers/remote/http-policy.ts`, switched.
//!
//! The two outer limits on `http` hooks (hooks.md, "Hook locations"; settings-reference.md,
//! `allowedHttpHookUrls` and `httpHookAllowedEnvVars`). They "apply to hooks from every source,
//! including managed policy settings", and the merged lists are the settings stream's: the host
//! passes them, already merged, in `RunContext.httpPolicy`.

use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::format::issues::{PluginIssue, warning};
use crate::format::wildcard::matches_wildcard;
use crate::js::Pattern;

/// The two outer limits on `http` hooks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpHookPolicy {
    /// "When you define this key, Claude Code runs an HTTP hook only if its URL matches one of the
    /// patterns and blocks the rest without running them; an empty array blocks every HTTP hook."
    /// Patterns are URLs with `*` as a wildcard; unset allows any URL.
    #[serde(default)]
    pub allowed_urls: Option<Vec<String>>,
    /// "A hook can use a variable only if both its own `allowedEnvVars` and this key name it." Unset
    /// leaves each hook's own list.
    #[serde(default)]
    pub allowed_env_vars: Option<Vec<String>>,
}

/// `/^([a-z][a-z0-9+.-]*:\/\/)([^/?#]*)(.*)$/i` (`i` folds ASCII only; `.` is `[^\n\r  ]`, RS6).
static AUTHORITY: Pattern = LazyLock::new(|| {
    Regex::new("^([A-Za-z][A-Za-z0-9+.-]*://)([^/?#]*)([^\\n\\r\u{2028}\u{2029}]*)$").ok()
});

/// `authority` without its first `.` that ends it or sits before the port (`/\.(?=:|$)/`).
fn without_trailing_dot(authority: &str) -> String {
    let mut characters = authority.char_indices().peekable();
    while let Some((at, character)) = characters.next() {
        let ends_host = characters.peek().is_none_or(|(_, next)| *next == ':');
        if character == '.' && ends_host {
            let after = at + character.len_utf8();
            return format!(
                "{}{}",
                authority.get(..at).unwrap_or_default(),
                authority.get(after..).unwrap_or_default()
            );
        }
    }
    authority.to_owned()
}

/// "Hostname matching is case-insensitive and treats `hooks.example.com.`, with the trailing dot
/// that marks a fully qualified domain name, the same as `hooks.example.com`." Judgment
/// (spec/judgments.md): the scheme is case-insensitive too (RFC 3986), and only the host is
/// normalised; a path, a query and a port stay as written.
fn normalize_url(text: &str) -> String {
    let Some(parts) = AUTHORITY.as_ref().and_then(|regex| regex.captures(text)) else {
        return text.to_owned();
    };
    let part = |index: usize| parts.get(index).map_or("", |group| group.as_str());
    let host = without_trailing_dot(&part(2).to_lowercase());
    format!("{}{host}{}", part(1).to_ascii_lowercase(), part(3))
}

/// Whether `url` may run under the merged allowlist; no list allows every URL.
#[must_use]
pub fn url_allowed(url: &str, allowed_urls: Option<&[String]>) -> bool {
    let Some(allowed_urls) = allowed_urls else {
        return true;
    };
    let target = normalize_url(url);
    allowed_urls
        .iter()
        .any(|pattern| matches_wildcard(&normalize_url(pattern), &target))
}

/// The variables a hook may interpolate: its own list, cut to the settings list when one is set.
#[must_use]
pub fn permitted_env_vars(
    own: Option<Vec<String>>,
    policy: Option<&HttpHookPolicy>,
) -> Option<Vec<String>> {
    let Some(outer) = policy.and_then(|found| found.allowed_env_vars.as_ref()) else {
        return own;
    };
    Some(
        own.unwrap_or_default()
            .into_iter()
            .filter(|name| outer.contains(name))
            .collect(),
    )
}

/// The issue of an `http` hook whose URL the allowlist blocks; the hook does not run.
#[must_use]
pub fn blocked_url_issue(path: &str, url: &str) -> PluginIssue {
    warning(
        path,
        &format!("The http hook for {url} is blocked by allowedHttpHookUrls and does not run."),
    )
}

crate::registry::exports! {
    "hooks/handlers/remote/http-policy" {
        "urlAllowed" => ok |url: String, allowed_urls: Option<Vec<String>>| url_allowed(&url, allowed_urls.as_deref());
        "permittedEnvVars" => ok |own: Option<Vec<String>>, policy: Option<HttpHookPolicy>| permitted_env_vars(own, policy.as_ref());
        "blockedUrlIssue" => ok |path: String, url: String| blocked_url_issue(&path, &url);
    }
}
