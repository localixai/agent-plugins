//! Port of `src/format/marketplace/relevance.ts`, switched.
//!
//! `relevance` of a plugin entry (plugins/relevance.md, "Field reference" and "Validate your
//! marketplace"): the typed model and its limits. Unknown fields under `relevance` and
//! `relevance.signals` are ignored with a warning, "because ... older clients still load a
//! marketplace that uses `relevance` fields they don't recognize"; "a recognized field whose value
//! exceeds its limit ... invalidates the whole plugin entry". Limits count UTF-16 code units, the
//! length of a JavaScript string (spec/judgments.md). No signal matching happens here.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::marketplace_schema::{type_name, unknown_field};
use super::schema_issues::zod_issues;
use super::types::{Relevance, RelevanceSignals};
use crate::format::issues::{PluginIssue, warning};
use crate::js::utf16::upper_case_first;
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, fields};

const TOPIC_KEYS: [&str; 2] = ["topic", "signals"];
const SIGNAL_KEYS: [&str; 5] = ["cwd", "cli", "hosts", "filesRead", "manifestDeps"];

/// A list of `count` strings of at most `length` characters.
fn list(count: f64, length: f64) -> Schema {
    zod::array(zod::string().max(length)).max(count)
}

/// "Bare lowercase hostname only: no scheme, port, or path". Oracle 2.1.325 accepts only lower-case
/// letters, digits, `.` and `-`, so an upper-case letter, a space, an underscore or a non-ASCII
/// letter invalidates the entry like a scheme (spec J15.10; spec/judgments.md).
fn host() -> Schema {
    zod::string().max(128.0).refine(
        // `/^[a-z0-9.-]*$/.test(value)`
        |value| {
            value.as_str().is_some_and(|text| {
                text.chars()
                    .all(|character| matches!(character, 'a'..='z' | '0'..='9' | '.' | '-'))
            })
        },
        "must be a lowercase bare hostname (no scheme, port, or path)",
    )
}

static SIGNALS_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "cwd" => list(10.0, 256.0).optional(),
        "cli" => list(10.0, 64.0).optional(),
        "hosts" => zod::array(host()).max(20.0).optional(),
        "filesRead" => list(10.0, 256.0).optional(),
        "manifestDeps" => zod::array(zod::loose_object(fields! {
            "file" => zod::string().max(256.0),
            "pattern" => zod::string().max(256.0),
        }))
        .max(10.0)
        .optional(),
    })
});

static RELEVANCE_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "topic" => zod::string().max(64.0).optional(),
        "signals" => SIGNALS_SCHEMA.optional(),
    })
});

fn not_an_object(kind: &str) -> String {
    format!(
        "'relevance' must be an object containing topic and signals; got {kind}. It will be ignored at load time."
    )
}

fn unknown_keys(value: &JsObject, keys: &[&str], base: &str) -> Vec<PluginIssue> {
    property_order(value)
        .into_iter()
        .map(|(key, _)| key)
        .filter(|key| !keys.contains(&key.as_str()))
        .map(|key| unknown_field(key, &format!("{base}.{key}"), keys))
        .collect()
}

/// The signals as the model holds them: the recognized keys only (`manifestDeps` items keep
/// `file` and `pattern`), as reading the parsed object into `RelevanceSignals` leaves them.
fn signals_of(parsed: &JsValue) -> RelevanceSignals {
    RelevanceSignals::deserialize(parsed.clone()).unwrap_or_default()
}

/// What reading `relevance` gave.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelevanceReading {
    /// Undefined when the value is dropped (not an object) or invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance: Option<Relevance>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

/// Reads `plugins[i].relevance` at `base` (`plugins[i].relevance`); a value that is not an object is a warning and is dropped.
#[must_use]
pub fn read_relevance(value: &JsValue, base: &str) -> RelevanceReading {
    if value.is_undefined() {
        return RelevanceReading {
            relevance: None,
            issues: Vec::new(),
        };
    }
    let Some(members) = value.as_object().filter(|_| is_record(value)) else {
        return RelevanceReading {
            relevance: None,
            issues: vec![warning(base, &not_an_object(type_name(value)))],
        };
    };
    let issues = unknown_keys(members, &TOPIC_KEYS, base);
    let signals = match members.get("signals") {
        Some(found) if is_record(found) => found.as_object().map_or_else(Vec::new, |inner| {
            unknown_keys(inner, &SIGNAL_KEYS, &format!("{base}.signals"))
        }),
        Some(_) | None => Vec::new(),
    };
    let parsed = match RELEVANCE_SCHEMA.safe_parse(value) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return RelevanceReading {
                relevance: None,
                issues: zod_issues(&failure, base)
                    .into_iter()
                    .chain(issues)
                    .chain(signals)
                    .collect(),
            };
        }
    };
    let topic = parsed
        .get("topic")
        .and_then(JsValue::as_str)
        .map(str::to_owned);
    let relevance = Relevance {
        topic,
        signals: parsed
            .get("signals")
            .filter(|found| !found.is_undefined())
            .map(signals_of),
    };
    RelevanceReading {
        relevance: Some(relevance),
        issues: issues.into_iter().chain(signals).collect(),
    }
}

/// The plugin an entry names and its `relevance`: what `effectiveTopic` reads of an entry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TopicSource {
    /// The plugin's name.
    pub name: String,
    /// Its relevance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relevance: Option<Relevance>,
}

/// The phrase of "Working with <topic>?": `relevance.topic`, which "defaults to the plugin name with
/// each hyphen segment capitalized" (`terraform-helpers` gives `Terraform Helpers`).
#[must_use]
pub fn effective_topic(entry: &TopicSource) -> String {
    entry
        .relevance
        .as_ref()
        .and_then(|relevance| relevance.topic.clone())
        .unwrap_or_else(|| {
            entry
                .name
                .split('-')
                .map(upper_case_first)
                .collect::<Vec<String>>()
                .join(" ")
        })
}

crate::registry::exports! {
    "format/marketplace/relevance" {
        "readRelevance" => ok |value: JsValue, base: String| read_relevance(&value, &base);
        "effectiveTopic" => ok |entry: TopicSource| effective_topic(&entry);
    }
}
