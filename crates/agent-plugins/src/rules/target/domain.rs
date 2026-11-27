//! Port of `src/rules/target/domain.ts`, switched.
//!
//! WebFetch domain rules (permissions.md, "WebFetch"): `domain:` is matched against the hostname
//! of the requested URL, case-insensitively, with a trailing `.` stripped from the rule and the
//! hostname. `example.com` is that host only; a leading `*.` is any subdomain at any depth but not
//! the domain itself; a bare `*` is every host; a `*` anywhere else stands for the text between
//! two dots only, so `example.*` is `example.org` and never `example.evil.com`.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::input::input_string;
use crate::format::wildcard::wildcard_source;
use crate::js::url::{Url, parse};
use crate::js::{Pattern, trim};
use crate::rules::restricts::restricts;
use crate::rules::specifier_guards::is_domain_specifier;
use crate::rules::types::{MatchContext, PermissionBehavior, PermissionRule, PreparedCall};

fn strip_trailing_dot(host: &str) -> &str {
    host.strip_suffix('.').unwrap_or(host)
}

fn normalize(text: &str) -> String {
    strip_trailing_dot(&trim(text).to_lowercase()).to_owned()
}

/// A label with a `*` or URL syntax in it is not a name to convert: it stays as written.
fn not_a_plain_label(label: &str) -> bool {
    label.contains(['*', ':', '/', '?', '#', '@', '\\'])
}

/// A label with non-ASCII letters, as the URL parser spells it (`bücher` is `xn--bcher-kva`).
fn ascii_label(label: &str) -> String {
    if label.is_ascii() || not_a_plain_label(label) {
        return label.to_owned();
    }
    parse_url(&format!("http://{label}/")).map_or_else(|| label.to_owned(), |url| url.hostname)
}

/// The rule's pattern spelled as `URL.hostname` spells the host, so `domain:bücher.de` meets
/// `https://bücher.de/`.
fn ascii_pattern(pattern: &str) -> String {
    pattern
        .split('.')
        .map(ascii_label)
        .collect::<Vec<_>>()
        .join(".")
}

/// The port a URL goes to: the one it names, else its scheme's default.
fn effective_port(url: &Url) -> &str {
    if !url.port.is_empty() {
        return &url.port;
    }
    match url.protocol.as_str() {
        "http:" => "80",
        "https:" => "443",
        _ => "",
    }
}

/// A URL that parses and has a host: anything else has no hostname for a rule to match
/// (spec/judgments.md).
fn parse_url(text: &str) -> Option<Url> {
    parse(text).filter(|url| !url.hostname.is_empty())
}

/// `*` in the middle of a label or between dots is one label's worth of text, never a dot.
const WITHIN_A_LABEL: &str = "[^.]*";

/// The host pattern; a leading `*.` is `.+\.`, JavaScript's `.` spelt for Rust.
fn wildcard_host(pattern: &str) -> Option<Regex> {
    let (leading, rest) = pattern
        .strip_prefix("*.")
        .map_or((false, pattern), |rest| (true, rest));
    let body = wildcard_source(rest, Some(WITHIN_A_LABEL));
    let prefix = if leading {
        r"[^\n\r\u{2028}\u{2029}]+\."
    } else {
        ""
    };
    Regex::new(&format!("^{prefix}{body}$")).ok()
}

/// `[::1]` and `[::1]:443` (sandboxing.md, "Network isolation"): the address, and optionally one
/// port.
static BRACKETED: Pattern = LazyLock::new(|| Regex::new(r"^(\[[^\]]*\])(?::([0-9]+))?$").ok());

/// The address as a URL spells its hostname, so `[0:0::1]` and `[::1]` are one address.
fn address_host(bracketed: &str) -> Option<String> {
    parse_url(&format!("http://{bracketed}/")).map(|url| url.hostname)
}

fn match_bracketed(host: &str, port: Option<&str>, url: &Url) -> bool {
    if address_host(host).as_deref() != Some(url.hostname.as_str()) {
        return false;
    }
    port.is_none_or(|port| port == effective_port(url))
}

/// Whether the `domain:` pattern of a WebFetch rule covers the host of `url`. An address in
/// brackets (`[::1]`) covers that address on every port, and `[::1]:443` on port 443 only; a URL
/// that does not parse has no host and matches no pattern. A pattern with non-ASCII labels is
/// compared in the punycode form `URL.hostname` gives the host (spec/judgments.md).
#[must_use]
pub fn match_domain_pattern(pattern: &str, url: &str) -> bool {
    let Some(parsed) = parse_url(url) else {
        return false;
    };
    let wanted = ascii_pattern(&normalize(pattern));
    if wanted == "*" {
        return true;
    }
    let address = BRACKETED
        .as_ref()
        .and_then(|bracketed| bracketed.captures(&wanted));
    if let Some(address) = address {
        let host = address.get(1).map_or("", |host| host.as_str());
        return match_bracketed(host, address.get(2).map(|port| port.as_str()), &parsed);
    }
    wildcard_host(&wanted).is_some_and(|host| host.is_match(&normalize(&parsed.hostname)))
}

/// The matcher of the `domain` rule format: a `WebFetch(domain:...)` rule against the call's
/// `url`. A specifier without the `domain:` prefix matches nothing, because the docs give WebFetch
/// no other form (spec/judgments.md).
#[must_use]
pub fn match_web_fetch_rule(rule: &PermissionRule, call: &PreparedCall) -> bool {
    let Some(pattern) = is_domain_specifier(rule.specifier.as_ref()) else {
        return false;
    };
    input_string(&call.call.input, "url").is_some_and(|url| match_domain_pattern(pattern, url))
}

/// The `Artifact` call that reads a published artifact (the SDK's `ArtifactInput.action`).
const ARTIFACT_READ: &str = "read";

/// A host of the `*.claudeusercontent.com` content host that serves artifact pages
/// (artifacts.md, "Allowlist the viewer domain"; errors.md names it
/// `*.frame.claudeusercontent.com`). The kit sees no real one, so a rule covers the content host
/// when it covers this probe.
const CONTENT_HOST_PROBE: &str = "https://artifact.frame.claudeusercontent.com/";

/// Whether a `WebFetch(domain:...)` rule covers an `Artifact` read (permissions.md, "Allow or
/// deny every fetch"): a `domain:` rule covering `claude.ai`, as the read's URL names it, or the
/// `*.claudeusercontent.com` content host applies to the read in a deny or ask rule. A bare
/// `WebFetch` rule never does, because its tool name is not the call's; the `Artifact` rule is a
/// rule on that tool and matches by name.
#[must_use]
pub fn domain_rule_covers_artifact_read(rule: &PermissionRule, call: &PreparedCall) -> bool {
    let specifier = is_domain_specifier(rule.specifier.as_ref());
    let Some(pattern) = specifier.filter(|_| !rule.tool_glob && rule.value.tool_name == "WebFetch")
    else {
        return false;
    };
    // A deny or ask `domain:` rule applies to an artifact read; an allow rule does not pre-approve
    // one.
    if !restricts(rule.behavior) || call.call.tool_name != "Artifact" {
        return false;
    }
    let input = &call.call.input;
    let Some(url) = input_string(input, "url") else {
        return false;
    };
    if input_string(input, "action") != Some(ARTIFACT_READ) {
        return false;
    }
    match_domain_pattern(pattern, url) || match_domain_pattern(pattern, CONTENT_HOST_PROBE)
}

/// The sandbox's domain lists (sandboxing.md, "Network isolation").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SandboxList {
    /// `allowed`.
    Allowed,
    /// `denied`.
    Denied,
}

/// The host a `WebFetch(domain:...)` rule adds to a sandbox list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SandboxDomainEntry {
    /// The list.
    pub list: SandboxList,
    /// The host.
    pub domain: String,
}

/// An allow rule adds its domain to the allowed list, a deny rule to the denied one; an ask rule
/// to neither.
const fn sandbox_list(behavior: PermissionBehavior) -> Option<SandboxList> {
    match behavior {
        PermissionBehavior::Allow => Some(SandboxList::Allowed),
        PermissionBehavior::Deny => Some(SandboxList::Denied),
        PermissionBehavior::Ask => None,
    }
}

/// "The sandbox honors two wildcard forms: a leading `*.` and a bare `*`": any other `*` has no
/// effect on it.
static SANDBOX_HONORED: Pattern = LazyLock::new(|| Regex::new(r"^(?:\*|\*\.[^*]+|[^*]+)$").ok());

/// What a rule changes about the hosts sandboxed commands reach (permissions.md, "Allow or deny
/// every fetch"; sandboxing.md): only a `WebFetch(domain:...)` rule adds its domain to a list, and
/// a bare `WebFetch` rule changes nothing there. A wildcard in a position the sandbox does not
/// honor (`example.*`) still matches fetches and adds nothing. `None` is "no entry".
#[must_use]
pub fn sandbox_domain_entry(rule: &PermissionRule) -> Option<SandboxDomainEntry> {
    let list = sandbox_list(rule.behavior)?;
    let pattern = is_domain_specifier(rule.specifier.as_ref())
        .filter(|_| rule.value.tool_name == "WebFetch")?;
    let domain = normalize(pattern);
    crate::js::regex_test(&SANDBOX_HONORED, &domain).then_some(SandboxDomainEntry { list, domain })
}

/// The two sandbox lists the `WebFetch(domain:...)` rules in `rules` contribute to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SandboxDomainLists {
    /// The allowed list.
    pub allowed_domains: Vec<String>,
    /// The denied list.
    pub denied_domains: Vec<String>,
}

/// The two sandbox lists the `WebFetch(domain:...)` rules in `rules` contribute to.
#[must_use]
pub fn sandbox_domain_lists(rules: &[PermissionRule]) -> SandboxDomainLists {
    let entries: Vec<SandboxDomainEntry> = rules.iter().filter_map(sandbox_domain_entry).collect();
    let domains = |list: SandboxList| {
        entries
            .iter()
            .filter(|entry| entry.list == list)
            .map(|entry| entry.domain.clone())
            .collect()
    };
    SandboxDomainLists {
        allowed_domains: domains(SandboxList::Allowed),
        denied_domains: domains(SandboxList::Denied),
    }
}

crate::registry::exports! {
    "rules/target/domain" {
        "matchDomainPattern" => ok |pattern: String, url: String| match_domain_pattern(&pattern, &url);
        "matchWebFetchRule" => ok |rule: PermissionRule, call: PreparedCall, _ctx: Option<MatchContext>| match_web_fetch_rule(&rule, &call);
        "domainRuleCoversArtifactRead" => ok |rule: PermissionRule, call: PreparedCall| domain_rule_covers_artifact_read(&rule, &call);
        "sandboxDomainEntry" => ok |rule: PermissionRule| sandbox_domain_entry(&rule);
        "sandboxDomainLists" => ok |rules: Vec<PermissionRule>| sandbox_domain_lists(&rules);
    }
}
