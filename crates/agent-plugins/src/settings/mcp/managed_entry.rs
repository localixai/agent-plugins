//! Ported from `src/settings/mcp/managed-entry.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a `managedMcpServers` entry may contain (managed-mcp.md, "What an entry can contain"): "Claude
//! Code loads an entry only when it passes every check below". A check returns why the entry fails it,
//! or `None`; they run in the order of the page's list (type, url, program members, `${VAR}`, then the
//! name and the invisible characters), the first failure is the notice.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use super::origins::mentions_variable;
use crate::format::mcp_config::read_mcp_server;
use crate::js::{self, JsObject, JsValue, Pattern, property_order, regex_test};
use crate::protocol::is_record::is_record;

type Check = fn(&str, &JsObject) -> Option<String>;

/// "only letters, numbers, hyphens, and underscores".
static NAME: Pattern = LazyLock::new(|| Regex::new("^[A-Za-z0-9_-]+$").ok());

/// "control or invisible formatting characters": the Unicode control and format categories, and the
/// line and paragraph separators (spec/judgments.md).
static INVISIBLE: Pattern = LazyLock::new(|| Regex::new(r"[\p{Cc}\p{Cf}\p{Zl}\p{Zp}]").ok());

/// The two transports the page allows, and the alias `.mcp.json` accepts for `http`.
const TRANSPORTS: [&str; 3] = ["http", "sse", "streamable-http"];

/// "no `command`, `args`, `env`, or `headersHelper` member, so a managed settings document never names a program".
const PROGRAM_MEMBERS: [&str; 4] = ["command", "args", "env", "headersHelper"];

#[derive(Default)]
struct Texts {
    keys: Vec<String>,
    values: Vec<String>,
}

fn join(list: Vec<Texts>) -> Texts {
    let mut joined = Texts::default();
    for each in list {
        joined.keys.extend(each.keys);
        joined.values.extend(each.values);
    }
    joined
}

/// Every key and every string value in an entry, at any depth.
fn texts_of(value: &JsValue) -> Texts {
    match value {
        JsValue::String(text) => Texts {
            keys: Vec::new(),
            values: vec![text.clone()],
        },
        JsValue::Array(items) => join(items.iter().map(texts_of).collect()),
        JsValue::Object(members) => {
            let ordered = property_order(members);
            let nested = join(ordered.iter().map(|(_, one)| texts_of(one)).collect());
            let mut keys: Vec<String> = ordered.iter().map(|(key, _)| (*key).clone()).collect();
            keys.extend(nested.keys);
            Texts {
                keys,
                values: nested.values,
            }
        }
        JsValue::Undefined | JsValue::Null | JsValue::Bool(_) | JsValue::Number(_) => {
            Texts::default()
        }
    }
}

fn is_https(url: Option<&JsValue>) -> bool {
    url.and_then(JsValue::as_str)
        .and_then(js::url::parse)
        .is_some_and(|url| url.protocol == "https:")
}

fn check_type(_: &str, entry: &JsObject) -> Option<String> {
    let known = entry
        .get("type")
        .and_then(JsValue::as_str)
        .is_some_and(|kind| TRANSPORTS.contains(&kind));
    (!known).then(|| "its \"type\" must be \"http\" or \"sse\"".to_owned())
}

fn check_url(_: &str, entry: &JsObject) -> Option<String> {
    (!is_https(entry.get("url"))).then(|| "its \"url\" must be an https:// URL".to_owned())
}

fn check_program(_: &str, entry: &JsObject) -> Option<String> {
    let member = PROGRAM_MEMBERS
        .iter()
        .find(|each| entry.contains_key(**each))?;
    Some(format!(
        "it has a \"{member}\" member, and managed settings never name a program to run"
    ))
}

fn check_references(_: &str, entry: &JsObject) -> Option<String> {
    let texts = texts_of(&JsValue::Object(entry.clone()));
    texts.values.iter().any(|text| mentions_variable(text)).then(|| {
        "it contains a ${VAR} reference, and environment variables are not expanded in managed entries"
            .to_owned()
    })
}

fn check_name(name: &str, _: &JsObject) -> Option<String> {
    (!regex_test(&NAME, name))
        .then(|| "the name may hold only letters, numbers, hyphens and underscores".to_owned())
}

fn check_invisible(name: &str, entry: &JsObject) -> Option<String> {
    let Texts { keys, values } = texts_of(&JsValue::Object(entry.clone()));
    std::iter::once(name)
        .chain(keys.iter().map(String::as_str))
        .chain(values.iter().map(String::as_str))
        .any(|text| regex_test(&INVISIBLE, text))
        .then(|| "a key or value contains control or invisible formatting characters".to_owned())
}

const CHECKS: [Check; 6] = [
    check_type,
    check_url,
    check_program,
    check_references,
    check_name,
    check_invisible,
];

/// The server of an entry, or why Claude Code drops it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManagedEntryRead {
    /// The server (`undefined` when dropped).
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub server: JsValue,
    /// Why Claude Code drops the entry (`undefined` when it loads).
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub reason: JsValue,
}

fn dropped(reason: String) -> ManagedEntryRead {
    ManagedEntryRead {
        server: JsValue::Undefined,
        reason: JsValue::String(reason),
    }
}

/// One entry of `managedMcpServers`: every check, then the shape of a remote `.mcp.json` server.
#[must_use]
pub fn read_managed_entry(name: &str, raw: &JsValue) -> ManagedEntryRead {
    let Some(entry) = raw.as_object().filter(|_| is_record(raw)) else {
        return dropped("the entry must be an object".to_owned());
    };
    for check in CHECKS {
        if let Some(reason) = check(name, entry) {
            return dropped(reason);
        }
    }
    let read = read_mcp_server(name, raw, "managedMcpServers");
    match read.server {
        None => dropped(
            read.issues
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join("; "),
        ),
        Some(server) => ManagedEntryRead {
            server,
            reason: JsValue::Undefined,
        },
    }
}
