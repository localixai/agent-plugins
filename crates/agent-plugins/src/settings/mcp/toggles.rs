//! Port of `src/settings/mcp/toggles.ts` at sha256:8556a6de588f.
//!
//! The per-project lists a toggle in `/mcp` writes to `~/.claude.json` (mcp.md, "Disable a server
//! without removing it"). They are unrelated to `enabledMcpjsonServers` and `disabledMcpjsonServers`.
//!
//! A candidate is the caller's own value (`McpCandidate` of `scope.ts`), answered with `enabled` set.
//! A subject none of the eight, which the TypeScript's table has no rule for (reading its list
//! throws), consults the opt-out list here: every candidate carries one of the six scopes.

use serde::{Deserialize, Serialize};

use super::candidates::claude_json_project;
use crate::js::JsValue;
use crate::protocol::is_record::is_record;

/// The two lists of one project.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectToggles {
    /// The servers turned off.
    pub disabled_mcp_servers: Vec<String>,
    /// The servers turned on.
    pub enabled_mcp_servers: Vec<String>,
}

fn texts_of(value: Option<&JsValue>) -> Vec<String> {
    value
        .and_then(JsValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(JsValue::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The two lists of one project's entry in the state file `~/.claude.json`, already parsed.
#[must_use]
pub fn read_project_toggles(state: &JsValue, project_path: &str) -> ProjectToggles {
    let entry = claude_json_project(state, project_path);
    let lists = if is_record(&entry) {
        entry
    } else {
        JsValue::Object(crate::js::JsObject::new())
    };
    ProjectToggles {
        disabled_mcp_servers: texts_of(lists.get("disabledMcpServers")),
        enabled_mcp_servers: texts_of(lists.get("enabledMcpServers")),
    }
}

#[derive(Clone, Copy)]
enum ToggleList {
    DisabledMcpServers,
    EnabledMcpServers,
}

struct ToggleRule {
    /// The one list this kind of server consults: "exactly one of the two lists for each server".
    list: ToggleList,
    /// Whether a server its list names is on: no for the opt-out list, yes for the opt-in list.
    on_when_listed: bool,
}

const OPT_OUT: ToggleRule = ToggleRule {
    list: ToggleList::DisabledMcpServers,
    on_when_listed: false,
};
const OPT_IN: ToggleRule = ToggleRule {
    list: ToggleList::EnabledMcpServers,
    on_when_listed: true,
};

const TOGGLE_RULES: [(&str, ToggleRule); 8] = [
    ("managed", OPT_OUT),
    ("local", OPT_OUT),
    ("project", OPT_OUT),
    ("user", OPT_OUT),
    ("plugin", OPT_OUT),
    ("connector", OPT_OUT),
    ("builtin", OPT_OUT),
    ("builtinDefaultOff", OPT_IN),
];

/// Whether the project's lists leave the server on. A name in the list the server does not consult is ignored.
#[must_use]
pub fn server_enabled(toggles: &ProjectToggles, subject: &str, name: &str) -> bool {
    let rule = TOGGLE_RULES
        .iter()
        .find(|(each, _)| *each == subject)
        .map_or(&OPT_OUT, |(_, rule)| rule);
    let list = match rule.list {
        ToggleList::DisabledMcpServers => &toggles.disabled_mcp_servers,
        ToggleList::EnabledMcpServers => &toggles.enabled_mcp_servers,
    };
    list.iter().any(|each| each == name) == rule.on_when_listed
}

/// How a connector is written in the lists: "under its display name, for example `claude.ai Slack`"
/// (`/mcp` lists the connector Anthropic provides as `claude.ai Claude Docs`).
#[must_use]
pub fn connector_display_name(connector: &str) -> String {
    format!("claude.ai {connector}")
}

/// The candidates with `enabled` set from the project's lists; a disabled one stays, marked.
#[must_use]
pub fn apply_toggles(candidates: Vec<JsValue>, toggles: &ProjectToggles) -> Vec<JsValue> {
    candidates
        .into_iter()
        .map(|each| {
            let text = |key: &str| {
                each.get(key)
                    .and_then(JsValue::as_str)
                    .unwrap_or("")
                    .to_owned()
            };
            let enabled = server_enabled(toggles, &text("scope"), &text("name"));
            match each {
                JsValue::Object(mut members) => {
                    members.insert("enabled".to_owned(), JsValue::Bool(enabled));
                    JsValue::Object(members)
                }
                other @ (JsValue::Undefined
                | JsValue::Null
                | JsValue::Bool(_)
                | JsValue::Number(_)
                | JsValue::String(_)
                | JsValue::Array(_)) => other,
            }
        })
        .collect()
}

crate::registry::exports! {
    "settings/mcp/toggles" {
        "readProjectToggles" => ok |state: JsValue, project_path: String| read_project_toggles(&state, &project_path);
        "serverEnabled" => ok |toggles: ProjectToggles, subject: String, name: String| server_enabled(&toggles, &subject, &name);
        "connectorDisplayName" => ok |connector: String| connector_display_name(&connector);
        "applyToggles" => ok |candidates: Vec<JsValue>, toggles: ProjectToggles| apply_toggles(candidates, &toggles);
    }
}
