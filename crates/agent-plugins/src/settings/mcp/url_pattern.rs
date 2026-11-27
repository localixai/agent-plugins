//! Port of `src/settings/mcp/url-pattern.ts`, switched.
//!
//! A `serverUrl` pattern against a server URL (managed-mcp.md, "How `serverUrl` entries match"):
//! `*` wildcards in the scheme, host, port and path; scheme and host case-insensitive and a trailing
//! FQDN dot ignored; paths case-sensitive; a pattern with no path matches any path. With no port, a
//! host written out in full allows the scheme's default port only (443 for `https`, 80 for `http`),
//! and a host with a `*` in it allows every port; `:*` allows every port and `:8443` that port.
//!
//! Judgments (spec/judgments.md): the server URL is read with the URL parser, so userinfo is never
//! part of the host, the default port written out equals no port and an IPv6 host keeps its brackets;
//! the path compared is the path and the query string (a fragment is never sent); a pattern or URL that
//! does not parse matches nothing; the glob is matched without backtracking blow-up, since the server's
//! URL is text a user controls.

use std::sync::LazyLock;

use regex::Regex;

use crate::format::mcp_env::{braced_references, is_variable_name, split_default};
use crate::format::wildcard::matches_wildcard;
use crate::js;

struct PatternParts {
    scheme: String,
    host: String,
    /// `None` when the pattern gives no port (an empty `:` counts as none).
    port: Option<String>,
    /// `None` when the pattern gives no path.
    path: Option<String>,
}

/// JavaScript's `.` is any character but a line terminator.
static PATTERN: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r"^([^:/]+)://(\[[^\]]*\]|[^/:]*)(?::([^/]*))?(/[^\n\r\u{2028}\u{2029}]*)?$").ok()
});

fn parse_pattern(text: &str) -> Option<PatternParts> {
    let found = PATTERN.as_ref()?.captures(text)?;
    let scheme = found.get(1)?.as_str();
    let host = found.get(2)?.as_str();
    if host.is_empty() {
        return None;
    }
    Some(PatternParts {
        scheme: scheme.to_owned(),
        host: host.to_owned(),
        port: found
            .get(3)
            .map(|port| port.as_str())
            .filter(|port| !port.is_empty())
            .map(str::to_owned),
        path: found.get(4).map(|path| path.as_str().to_owned()),
    })
}

/// The ports a URL without one is on; a scheme not listed has no default.
const DEFAULT_PORTS: [(&str, &str); 4] = [
    ("http", "80"),
    ("https", "443"),
    ("ws", "80"),
    ("wss", "443"),
];

struct UrlParts {
    scheme: String,
    host: String,
    /// The port the server is on: the written one, or the scheme's default.
    port: String,
    /// Whether the URL names no port other than the scheme's default.
    default_port: bool,
    path: String,
}

fn parse_url(text: &str) -> Option<UrlParts> {
    let url = js::url::parse(text)?;
    let scheme = url
        .protocol
        .strip_suffix(':')
        .unwrap_or(&url.protocol)
        .to_owned();
    let port = if url.port.is_empty() {
        DEFAULT_PORTS
            .iter()
            .find(|(name, _)| *name == scheme)
            .map_or("", |(_, port)| port)
            .to_owned()
    } else {
        url.port.clone()
    };
    Some(UrlParts {
        host: url.hostname,
        port,
        default_port: url.port.is_empty(),
        path: format!("{}{}", url.pathname, url.search),
        scheme,
    })
}

fn without_dot(host: &str) -> String {
    let lower = host.to_lowercase();
    lower.strip_suffix('.').unwrap_or(&lower).to_owned()
}

/// Hostname rule: no port in the pattern allows every port only for a host with a `*` in it.
fn port_matches(want: &PatternParts, have: &UrlParts) -> bool {
    if let Some(port) = &want.port {
        return matches_wildcard(port, &have.port);
    }
    want.host.contains('*') || have.default_port
}

fn path_matches(want: &PatternParts, have: &UrlParts) -> bool {
    want.path
        .as_ref()
        .is_none_or(|path| matches_wildcard(path, &have.path))
}

/// Whether a `serverUrl` pattern allows a server URL.
#[must_use]
pub fn url_pattern_matches(pattern: &str, url: &str) -> bool {
    let (Some(want), Some(have)) = (parse_pattern(js::trim(pattern)), parse_url(js::trim(url)))
    else {
        return false;
    };
    matches_wildcard(&want.scheme.to_lowercase(), &have.scheme)
        && matches_wildcard(&without_dot(&want.host), &without_dot(&have.host))
        && port_matches(&want, &have)
        && path_matches(&want, &have)
}

/// Stands for a `${VAR}` while the shape of an entry is read; no URL holds it.
const HOLE: char = '\u{0000}';

/// Whether a `${VAR}` reference of an entry written as `template` sits in the scheme, host or path: the
/// "expansion that would change a URL entry's scheme, host, or path scope" that makes an allowlist
/// entry ignored (managed-mcp.md, "Environment variables in `serverCommand` and `serverUrl` entries").
/// A reference in the port or anywhere else leaves the scope as written. Judgment (spec/judgments.md):
/// the docs name the three parts without saying how an expansion "would change" one, so a reference
/// in any of them counts, and an entry that does not parse around its references (`${BASE}/*`) counts.
#[must_use]
pub fn reference_in_url_scope(template: &str) -> bool {
    let mut masked = String::new();
    let mut last = 0;
    for (range, inner) in braced_references(template) {
        masked.push_str(template.get(last..range.start).unwrap_or_default());
        if is_variable_name(&split_default(inner).name) {
            masked.push(HOLE);
        } else {
            masked.push_str(template.get(range.clone()).unwrap_or_default());
        }
        last = range.end;
    }
    masked.push_str(template.get(last..).unwrap_or_default());
    if !masked.contains(HOLE) {
        return false;
    }
    parse_pattern(&masked).is_none_or(|parts| {
        [
            parts.scheme.as_str(),
            parts.host.as_str(),
            parts.path.as_deref().unwrap_or(""),
        ]
        .into_iter()
        .any(has)
    })
}

fn has(part: &str) -> bool {
    part.contains(HOLE)
}

crate::registry::exports! {
    "settings/mcp/url-pattern" {
        "urlPatternMatches" => ok |pattern: String, url: String| url_pattern_matches(&pattern, &url);
        "referenceInUrlScope" => ok |template: String| reference_in_url_scope(&template);
    }
}
