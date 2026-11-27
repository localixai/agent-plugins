//! Port of `src/format/validate/mcp-remote-checks.ts`, switched.
//!
//! The checks of `claude plugin validate` on a remote server's `url` and `headers` (plugins/
//! manifest-reference.md, "Validate the manifest"; spec §8 item 3): an error for a `url` that is not
//! a valid absolute URL once its references are set aside, warnings for an `http://` or `ws://` URL
//! to a host that is not loopback and for a header value that looks like a literal credential. The
//! wording and the credential shapes are oracle 2.1.325's (spec/judgments.md).

use std::sync::LazyLock;

use regex::Regex;

use super::finding::{Finding, error_at, warning_at};
use crate::js::url::parse;
use crate::js::{JsObject, JsValue, Pattern, SPACE_CLASS, property_order, regex_test};

const NOT_ABSOLUTE: &str = "url is not a valid absolute URL (for example \"https://host/path\"). The plugin loader marks this server as invalid config and it never connects.";

fn cleartext(scheme: &str, secure: &str) -> String {
    format!(
        "url uses {scheme}// to a non-loopback host — requests, headers and any credentials travel in cleartext. Use {secure}//; plugin directories may reject insecure remote server urls."
    )
}

/// The cleartext schemes and the secure scheme each message names.
const CLEARTEXT_SCHEMES: [(&str, &str); 2] = [("http:", "https:"), ("ws:", "wss:")];

const CREDENTIAL: &str = "header value looks like a literal credential. Everything shipped in a plugin is readable by everyone who installs it; reference a sensitive userConfig option (${user_config.KEY}) or an environment variable (${VAR}) instead of committing the value.";

const LOOPBACK: [&str; 4] = ["localhost", "127.0.0.1", "[::1]", "::1"];

/// A `${...}` reference: a value that holds one is not a literal and is checked without it.
const REFERENCE: &str = "${";

fn url_findings(path: &str, url: Option<&JsValue>) -> Vec<Finding> {
    // An empty url is a placeholder the server shows as `not configured` (spec §8.2, §8.8 item 6).
    let Some(url) = url
        .and_then(JsValue::as_str)
        .filter(|text| !text.is_empty() && !text.contains(REFERENCE))
    else {
        return Vec::new();
    };
    let Some(parsed) = parse(url) else {
        return vec![error_at(&format!("{path}.url"), NOT_ABSOLUTE)];
    };
    let secure = CLEARTEXT_SCHEMES
        .iter()
        .find(|(scheme, _)| *scheme == parsed.protocol)
        .map(|(_, secure)| *secure);
    match secure {
        Some(secure) if !LOOPBACK.contains(&parsed.hostname.as_str()) => vec![warning_at(
            &format!("{path}.url"),
            &cleartext(&parsed.protocol, secure),
        )],
        Some(_) | None => Vec::new(),
    }
}

/// Header names whose value is a secret by their name (oracle 2.1.325: not `Cookie`); the ASCII
/// letters fold only (RS6).
static SECRET_NAME: Pattern = LazyLock::new(|| {
    Regex::new(r"(?i-u:authorization|api[-_]?key|token|secret|passw(?:or)?d|credential)").ok()
});
static AUTH_SCHEME: Pattern =
    LazyLock::new(|| Regex::new(&format!(r"^(?i-u:bearer|basic|token)[{SPACE_CLASS}]+")).ok());
static LONG_SECRET: Pattern = LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._~+/=-]{20,}$").ok());

/// The shapes of well-known tokens, under any header name.
static TOKEN_SHAPES: [Pattern; 6] = [
    LazyLock::new(|| Regex::new(r"(?-u:\b)sk-[A-Za-z0-9_-]{20,}").ok()),
    LazyLock::new(|| Regex::new(r"(?-u:\b)gh[pousr]_[A-Za-z0-9]{36,}").ok()),
    LazyLock::new(|| Regex::new(r"(?-u:\b)glpat-[A-Za-z0-9_-]{20,}").ok()),
    LazyLock::new(|| Regex::new(r"(?-u:\b)xox[abprs]-[A-Za-z0-9-]{10,}").ok()),
    LazyLock::new(|| Regex::new(r"(?-u:\b)AKIA[A-Z0-9]{16}(?-u:\b)").ok()),
    LazyLock::new(|| {
        Regex::new(r"(?-u:\b)eyJ[A-Za-z0-9_-]{10,}\.eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]+").ok()
    }),
];

/// The documentation placeholders of those shapes (`AKIAIOSFODNN7EXAMPLE`).
const PLACEHOLDER: &str = "EXAMPLE";

/// `value.replace(AUTH_SCHEME, '')`: the first match only.
fn without_auth_scheme(value: &str) -> String {
    match AUTH_SCHEME.as_ref() {
        Some(scheme) => scheme.replace(value, "").into_owned(),
        None => value.to_owned(),
    }
}

fn looks_like_credential(name: &str, value: &str) -> bool {
    if value.contains(REFERENCE) || value.contains(PLACEHOLDER) {
        return false;
    }
    if TOKEN_SHAPES.iter().any(|shape| regex_test(shape, value)) {
        return true;
    }
    regex_test(&SECRET_NAME, name) && regex_test(&LONG_SECRET, &without_auth_scheme(value))
}

fn header_findings(path: &str, headers: Option<&JsValue>) -> Vec<Finding> {
    let Some(JsValue::Object(headers)) = headers else {
        return Vec::new();
    };
    property_order(headers)
        .into_iter()
        .filter_map(|(name, value)| {
            let text = value.as_str()?;
            looks_like_credential(name, text)
                .then(|| warning_at(&format!("{path}.headers.{name}"), CREDENTIAL))
        })
        .collect()
}

/// The url and header findings of one server at `path`.
#[must_use]
pub fn remote_findings(path: &str, server: &JsObject) -> Vec<Finding> {
    let mut findings = url_findings(path, server.get("url"));
    findings.extend(header_findings(path, server.get("headers")));
    findings
}

crate::registry::exports! {
    "format/validate/mcp-remote-checks" {
        "remoteFindings" => ok |path: String, server: JsObject| remote_findings(&path, &server);
    }
}
