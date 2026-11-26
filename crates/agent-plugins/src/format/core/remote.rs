//! Port of `src/format/core/remote.ts`, switched.
//!
//! The remote fields of a core MCP server (core §7.2.1, "Streamable HTTP and legacy HTTP+SSE"): the
//! `url` is an absolute HTTP or HTTPS URL with no user information and no fragment, HTTPS unless the
//! host is exactly `localhost` or a loopback IP literal; `headers` are valid HTTP fields, no name
//! twice under different casing. Neither is expanded: `${...}` in them is literal text.
//!
//! Judgments (spec/judgments.md): the loopback literals are `127.0.0.0/8` and `[::1]`; a header name
//! is an RFC 9110 token, a value holds visible ASCII, obs-text, spaces and tabs, but not at either end.

use std::sync::LazyLock;

use crate::js::JsValue;
use indexmap::IndexSet;
use regex::Regex;

use crate::js::url::{Url, parse};
use crate::js::{Pattern, regex_test};

/// `127.x.y.z`, digits ASCII (RS6).
static IPV4_LOOPBACK: Pattern =
    LazyLock::new(|| Regex::new(r"^127\.[0-9]{1,3}\.[0-9]{1,3}\.[0-9]{1,3}$").ok());

fn is_loopback(hostname: &str) -> bool {
    hostname == "localhost" || hostname == "[::1]" || regex_test(&IPV4_LOOPBACK, hostname)
}

/// The schemes of a remote server and whether each is secure.
const SCHEMES: [(&str, bool); 2] = [("https:", true), ("http:", false)];

fn parsed(text: &str) -> Option<Url> {
    parse(text)
}

/// `^[a-z][a-z0-9+.-]*:\/\/[^/?#]*@` without regard to case: the ASCII letters fold only (RS6).
static USER_INFO: Pattern =
    LazyLock::new(|| Regex::new(r"^(?i-u:[a-z][a-z0-9+.-]*)://[^/?#]*@").ok());

/// User information, even an empty one (`https://@host`), which `URL` would not show.
fn holds_user_info(url: &Url, text: &str) -> bool {
    !url.username.is_empty() || !url.password.is_empty() || regex_test(&USER_INFO, text)
}

/// Why a `url` breaks core §7.2.1, or `None` when it is valid.
#[must_use]
pub fn url_problem(value: &JsValue) -> Option<String> {
    let JsValue::String(text) = value else {
        return Some("must be a non-empty string".to_owned());
    };
    if text.is_empty() {
        return Some("must be a non-empty string".to_owned());
    }
    let Some(url) = parsed(text) else {
        return Some("must be an absolute URL".to_owned());
    };
    let Some((_, secure)) = SCHEMES
        .iter()
        .find(|(protocol, _)| *protocol == url.protocol)
    else {
        return Some("must be an http or https URL".to_owned());
    };
    if holds_user_info(&url, text) {
        return Some("must not hold user information".to_owned());
    }
    if text.contains('#') {
        return Some("must not hold a fragment".to_owned());
    }
    if !secure && !is_loopback(&url.hostname) {
        return Some(
            "must use https unless the host is localhost or a loopback address".to_owned(),
        );
    }
    None
}

/// An RFC 9110 token: the characters a field name holds.
static TOKEN: Pattern = LazyLock::new(|| Regex::new(r"^[!#$%&'*+.^_`|~0-9A-Za-z-]+$").ok());
/// A field value: visible ASCII and obs-text, spaces and tabs between them, none at either end.
static FIELD_VALUE: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:[\x21-\x7e\x80-\xff](?:[\t\x20-\x7e\x80-\xff]*[\x21-\x7e\x80-\xff])?)?$").ok()
});

fn header_problem(name: &str, value: &JsValue) -> Option<String> {
    if !regex_test(&TOKEN, name) {
        return Some(format!(
            "header name \"{name}\" is not a valid HTTP field name"
        ));
    }
    let JsValue::String(text) = value else {
        return Some(format!("header \"{name}\" must be a string"));
    };
    if regex_test(&FIELD_VALUE, text) {
        None
    } else {
        Some(format!(
            "header \"{name}\" has a value that is not a valid HTTP field value"
        ))
    }
}

/// Why `headers` breaks core §7.2.1, or `None` when every entry is a valid HTTP field.
#[must_use]
pub fn headers_problem(value: &JsValue) -> Option<String> {
    let JsValue::Object(members) = value else {
        return Some("must be an object of strings".to_owned());
    };
    let mut seen: IndexSet<String> = IndexSet::new();
    for (name, member) in members {
        if let Some(problem) = header_problem(name, member) {
            return Some(problem);
        }
        let folded = name.to_lowercase();
        if seen.contains(&folded) {
            return Some(format!(
                "header \"{name}\" is given twice under different casing"
            ));
        }
        seen.insert(folded);
    }
    None
}

crate::registry::exports! {
    "format/core/remote" {
        "urlProblem" => ok |value: JsValue| url_problem(&value);
        "headersProblem" => ok |value: JsValue| headers_problem(&value);
    }
}
