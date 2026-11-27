//! Port of `src/format/monitors.ts`, switched.
//!
//! Monitor entries of a plugin (plugins/manifest-reference.md, "monitors"; plugins/components.md,
//! "Monitors"). The SDK has no monitor config type (no match in sdk.d.ts), so `Monitor` is the kit's,
//! transcribed from the field table: "Each entry is a strict object with these fields", and "An
//! unknown key inside one is an error, and the plugin doesn't load" (manifest-reference, "Unrecognized
//! fields").

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::monitor_schema::{
    ALWAYS, MONITOR_ENTRY_SCHEMA, UNIQUE_NAMES, WHEN_SKILL, skill_of,
};
use crate::js::{JsValue, array_join};
use crate::protocol::is_record::is_record;
use crate::zod::ZodError;

/// `always`, or the first run of one skill.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MonitorTrigger {
    /// With the session.
    #[serde(rename = "always")]
    Always,
    /// The first time the skill runs.
    #[serde(rename = "on-skill-invoke")]
    OnSkillInvoke {
        /// The skill.
        skill: String,
    },
}

/// One monitor.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Monitor {
    /// "Identifier unique within the plugin."
    pub name: String,
    /// "Shell command Claude Code runs as a persistent background process in the session working directory."
    pub command: String,
    /// "Short summary shown in the task panel and notification summaries."
    pub description: String,
    /// The `when` field; absent means `always`, "the default".
    pub when: MonitorTrigger,
}

/// `"always"` or `"on-skill-invoke:<skill>"`. The skill after the colon must not be empty (oracle
/// 2.1.325: `on-skill-invoke: must specify a skill name`), and the value is read as written, with no
/// trimming or case folding (spec J9.8).
#[must_use]
pub fn parse_monitor_trigger(when: &str) -> Option<MonitorTrigger> {
    if when == ALWAYS {
        return Some(MonitorTrigger::Always);
    }
    let skill = skill_of(when);
    (!skill.is_empty()).then_some(MonitorTrigger::OnSkillInvoke { skill })
}

fn reason(failure: &ZodError) -> String {
    failure
        .issues
        .iter()
        .map(|found| {
            if found.path().is_empty() {
                found.message().to_owned()
            } else {
                format!("{}: {}", array_join(found.path(), "."), found.message())
            }
        })
        .collect::<Vec<String>>()
        .join("; ")
}

fn invalid(origin: &str, index: usize, why: &str) -> PluginIssue {
    error(
        &format!("{origin}[{index}]"),
        &format!("Invalid monitor at {origin}[{index}]: {why}"),
    )
}

/// What reading one entry gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorRead {
    /// Undefined when Claude Code would not load the entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monitor: Option<Monitor>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
}

fn text_member(parsed: &JsValue, key: &str) -> String {
    parsed
        .get(key)
        .and_then(JsValue::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// One entry of the array; `index` is its position, for the issue.
#[must_use]
pub fn read_monitor(raw: &JsValue, origin: &str, index: usize) -> MonitorRead {
    if !is_record(raw) {
        return MonitorRead {
            monitor: None,
            issues: vec![invalid(origin, index, "expected an object")],
        };
    }
    let parsed = match MONITOR_ENTRY_SCHEMA.safe_parse(raw) {
        Ok(parsed) => parsed,
        Err(failure) => {
            return MonitorRead {
                monitor: None,
                issues: vec![invalid(origin, index, &reason(&failure))],
            };
        }
    };
    let when = parsed.get("when").and_then(JsValue::as_str);
    let trigger = when.map_or(Some(MonitorTrigger::Always), parse_monitor_trigger);
    // The schema refused every other `when`; this narrows the type.
    let Some(trigger) = trigger else {
        return MonitorRead {
            monitor: None,
            issues: vec![invalid(origin, index, WHEN_SKILL)],
        };
    };
    MonitorRead {
        monitor: Some(Monitor {
            name: text_member(&parsed, "name"),
            command: text_member(&parsed, "command"),
            description: text_member(&parsed, "description"),
            when: trigger,
        }),
        issues: Vec::new(),
    }
}

/// The monitors of one source, and what is wrong.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorsRead {
    /// The monitors kept, in order.
    pub monitors: Vec<Monitor>,
    /// What is wrong.
    pub issues: Vec<PluginIssue>,
}

/// The monitors of one source: `monitors/monitors.json`, a `.json` file `experimental.monitors` names,
/// or its inline array (components.md: "The `experimental.monitors` manifest key takes the same array
/// inline or a path to a JSON file"; manifest-reference: "a `.json` file holding the monitors array").
///
/// An inline array is part of the manifest's schema, so an invalid one never gets here: the plugin
/// does not load (spec §9.2.1, J9.7). A file with an invalid entry is skipped whole and reported, and
/// the plugin's other components load (J9.7). A later entry of a name already taken is left out, the
/// first keeping the name (J9.7, "at runtime a repeated name keeps the first entry").
#[must_use]
pub fn read_monitors(raw: &JsValue, origin: &str) -> MonitorsRead {
    let Some(entries) = raw.as_array() else {
        let why = "expected an array of monitors";
        return MonitorsRead {
            monitors: Vec::new(),
            issues: vec![error(origin, &format!("Invalid monitors in {origin}: {why}"))],
        };
    };
    let reads: Vec<MonitorRead> = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| read_monitor(entry, origin, index))
        .collect();
    let failures: Vec<PluginIssue> = reads
        .iter()
        .flat_map(|read| read.issues.iter().cloned())
        .collect();
    if !failures.is_empty() {
        return MonitorsRead {
            monitors: Vec::new(),
            issues: failures,
        };
    }
    let mut monitors: Vec<Monitor> = Vec::new();
    let mut issues: Vec<PluginIssue> = Vec::new();
    for (index, read) in reads.into_iter().enumerate() {
        let Some(monitor) = read.monitor else {
            continue;
        };
        if monitors.iter().any(|kept| kept.name == monitor.name) {
            issues.push(invalid(
                origin,
                index,
                &format!(
                    "{UNIQUE_NAMES}; \"{}\" is kept from the first entry",
                    monitor.name
                ),
            ));
            continue;
        }
        monitors.push(monitor);
    }
    MonitorsRead { monitors, issues }
}

crate::registry::exports! {
    "format/monitors" {
        "parseMonitorTrigger" => ok |when: String| parse_monitor_trigger(&when);
        "readMonitor" => ok |raw: JsValue, origin: String, index: usize|
            read_monitor(&raw, &origin, index);
        "readMonitors" => ok |raw: JsValue, origin: String| read_monitors(&raw, &origin);
    }
}
