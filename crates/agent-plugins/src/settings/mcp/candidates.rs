//! Port of `src/settings/mcp/candidates.ts`, switched.
//!
//! The MCP servers of the files Claude Code itself keeps, as candidates of their scope (`McpCandidate`
//! of `scope.ts`, `{ scope, name, server }`): `~/.claude.json` and a project's `.mcp.json`.

use serde::Serialize;

use crate::format::mcp_config::read_mcp_config;
use crate::format::mcp_reserved::without_reserved_servers;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, property_order};
use crate::protocol::is_record::is_record;

/// The candidates of one read and what it dropped.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct McpCandidatesRead {
    /// The candidates, each `{ scope, name, server }`.
    pub candidates: Vec<JsValue>,
    /// The entries Claude Code drops (`PluginIssue`s).
    pub issues: Vec<JsValue>,
}

/// `{ scope, name, server }`.
fn candidate(scope: &str, name: &str, server: &JsValue) -> JsValue {
    JsValue::Object(JsObject::from([
        ("scope".to_owned(), JsValue::String(scope.to_owned())),
        ("name".to_owned(), JsValue::String(name.to_owned())),
        ("server".to_owned(), server.clone()),
    ]))
}

/// `withoutReservedServers(readMcpConfig(servers, origin), origin)` as candidates of `scope`.
fn read_as(scope: &str, servers: &JsValue, origin: &str) -> McpCandidatesRead {
    let read = to_js_value(&read_mcp_config(servers, origin)).unwrap_or_default();
    let read = without_reserved_servers(&read, origin);
    let candidates = read
        .get("servers")
        .and_then(JsValue::as_object)
        .map(|servers| {
            property_order(servers)
                .into_iter()
                .map(|(name, server)| candidate(scope, name, server))
                .collect()
        })
        .unwrap_or_default();
    let issues = read
        .get("issues")
        .and_then(JsValue::as_array)
        .cloned()
        .unwrap_or_default();
    McpCandidatesRead { candidates, issues }
}

/// The servers of one `mcpServers` map as candidates of `scope`; an entry Claude Code drops is an issue.
fn candidates_of(scope: &str, servers: &JsValue, origin: &str) -> McpCandidatesRead {
    if !is_record(servers) {
        return McpCandidatesRead::default();
    }
    read_as(scope, servers, origin)
}

/// The `~/.claude.json` entry of one project (`projects["<path>"]`), as it was written.
#[must_use]
pub fn claude_json_project(state: &JsValue, project_path: &str) -> JsValue {
    state
        .get("projects")
        .filter(|projects| is_record(projects))
        .and_then(|projects| projects.get(project_path))
        .cloned()
        .unwrap_or_default()
}

fn member_of_record(value: &JsValue, key: &str) -> JsValue {
    if is_record(value) {
        value.get(key).cloned().unwrap_or_default()
    } else {
        JsValue::Undefined
    }
}

/// The user and local servers of Claude Code's state file `~/.claude.json`, already parsed by the host
/// (mcp.md, "MCP installation scopes"): the top-level `mcpServers` are user scope, "available across
/// all projects"; those under `projects["<path>"]` are local scope and load "only in the project where
/// you added it". The host names the project path the way Claude Code keys it.
#[must_use]
pub fn claude_json_servers(state: &JsValue, project_path: &str) -> McpCandidatesRead {
    let origin = "~/.claude.json";
    let local = claude_json_project(state, project_path);
    let reads = [
        candidates_of(
            "local",
            &member_of_record(&local, "mcpServers"),
            &format!("{origin} ({project_path})"),
        ),
        candidates_of("user", &member_of_record(state, "mcpServers"), origin),
    ];
    McpCandidatesRead {
        candidates: reads
            .iter()
            .flat_map(|read| read.candidates.iter().cloned())
            .collect(),
        issues: reads
            .iter()
            .flat_map(|read| read.issues.iter().cloned())
            .collect(),
    }
}

/// The servers of a project's `.mcp.json`, already parsed (mcp.md, "Project scope"): project scope.
/// An absent file (`undefined`) holds none. They load only once approved (`projectServerApproval`); this reads, it does not decide.
#[must_use]
pub fn project_file_servers(mcp_json: &JsValue) -> McpCandidatesRead {
    if mcp_json.is_undefined() {
        return McpCandidatesRead::default();
    }
    read_as("project", mcp_json, ".mcp.json")
}

crate::registry::exports! {
    "settings/mcp/candidates" {
        "claudeJsonProject" => ok |state: JsValue, project_path: String| claude_json_project(&state, &project_path);
        "claudeJsonServers" => ok |state: JsValue, project_path: String| claude_json_servers(&state, &project_path);
        "projectFileServers" => ok |mcp_json: JsValue| project_file_servers(&mcp_json);
    }
}
