//! Port of `src/settings/mcp/scope.ts`, switched.
//!
//! Where an MCP server comes from, highest precedence first (mcp.md, "Scope hierarchy and
//! precedence"), and the ranking that loads each server once.
//!
//! A candidate is the caller's own value (`McpCandidate` and any member it carries), answered as
//! given: the ranking only reads it. A scope none of the six, which the TypeScript's table has no
//! rule for (reading its rank throws), ranks below them all and holds nothing here; the callers
//! build every candidate with one of the six.

use serde::{Deserialize, Serialize};

use crate::format::mcp_config::is_remote_mcp_server;
use crate::format::mcp_endpoint::mcp_endpoint_key;
use crate::js::{self, JsValue};

/// Where an MCP server comes from, highest precedence first (mcp.md, "Scope hierarchy and
/// precedence"): the organization's `managedMcpServers` "ranks above all of these", then local,
/// project and user scope, plugin-provided servers and claude.ai connectors.
pub const MCP_SCOPES: [&str; 6] = ["managed", "local", "project", "user", "plugin", "connector"];

fn member<'a>(candidate: &'a JsValue, key: &str) -> &'a JsValue {
    candidate.get(key).unwrap_or(&JsValue::Undefined)
}

fn scope_of(candidate: &JsValue) -> &str {
    member(candidate, "scope").as_str().unwrap_or("")
}

/// What makes two candidates one server: the one thing each scope's rule picks.
type Identify = fn(&JsValue) -> String;

fn by_name(candidate: &JsValue) -> String {
    format!(
        "name:{}",
        js::json_stringify(member(candidate, "name")).unwrap_or_else(|| "undefined".to_owned())
    )
}

fn by_endpoint(candidate: &JsValue) -> String {
    mcp_endpoint_key(member(candidate, "server"))
}

struct ScopeRule {
    /// A higher number wins.
    rank: f64,
    /// How a candidate of this scope finds the server above it that it duplicates.
    identify: Identify,
    /// Whether servers of this scope hold their name against the scopes that match by name.
    holds_name: bool,
}

/// "Claude Code matches duplicates across the three scopes by name. It matches plugins and connectors
/// by endpoint." The managed servers are matched by name against the three scopes and, through the
/// endpoint every enabled server holds, hide a plugin or connector at their URL (managed-mcp.md,
/// "How provided servers load").
const SCOPE_RULES: [(&str, ScopeRule); 6] = [
    (
        "managed",
        ScopeRule {
            rank: 6.0,
            identify: by_name,
            holds_name: true,
        },
    ),
    (
        "local",
        ScopeRule {
            rank: 5.0,
            identify: by_name,
            holds_name: true,
        },
    ),
    (
        "project",
        ScopeRule {
            rank: 4.0,
            identify: by_name,
            holds_name: true,
        },
    ),
    (
        "user",
        ScopeRule {
            rank: 3.0,
            identify: by_name,
            holds_name: true,
        },
    ),
    (
        "plugin",
        ScopeRule {
            rank: 2.0,
            identify: by_endpoint,
            holds_name: false,
        },
    ),
    (
        "connector",
        ScopeRule {
            rank: 1.0,
            identify: by_endpoint,
            holds_name: false,
        },
    ),
];

/// The rule of a scope none of the six (the module comment).
const NO_RULE: ScopeRule = ScopeRule {
    rank: 0.0,
    identify: by_name,
    holds_name: false,
};

fn rule_of(candidate: &JsValue) -> &'static ScopeRule {
    let scope = scope_of(candidate);
    SCOPE_RULES
        .iter()
        .find(|(name, _)| *name == scope)
        .map_or(&NO_RULE, |(_, rule)| rule)
}

/// How the ranking reads a session.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankOptions {
    /// The session is a local one in the Desktop app's Code tab: "the Code tab uses the `~/.claude.json`
    /// definition" of a stdio server written in both user scope and `.mcp.json`.
    #[serde(default)]
    pub desktop_code_tab: Option<bool>,
}

/// A candidate a higher one of the same identity displaced: `/mcp` lists a hidden connector so.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HiddenMcpServer {
    /// The candidate left out.
    pub candidate: JsValue,
    /// The one loaded in its place.
    pub duplicate_of: JsValue,
}

/// What loads and what was displaced.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpRanking {
    /// One server per identity, highest scope first; a disabled one is kept (hosts skip `enabled: false`).
    pub loaded: Vec<JsValue>,
    /// The candidates a higher one displaced.
    pub hidden: Vec<HiddenMcpServer>,
}

/// Between project (4) and local (5): the user definition beats `.mcp.json` and nothing above it.
const DESKTOP_USER_RANK: f64 = 4.5;

fn is_stdio(candidate: &JsValue) -> bool {
    !is_remote_mcp_server(member(candidate, "server"))
}

fn written_in_project_too(user: &JsValue, all: &[JsValue]) -> bool {
    all.iter().any(|other| {
        scope_of(other) == "project"
            && js::strict_equals(member(other, "name"), member(user, "name"))
            && is_stdio(other)
            && is_stdio(user)
    })
}

fn rank_of(candidate: &JsValue, all: &[JsValue], options: &RankOptions) -> f64 {
    let own = rule_of(candidate).rank;
    let desktop_user = options.desktop_code_tab == Some(true)
        && scope_of(candidate) == "user"
        && written_in_project_too(candidate, all);
    if desktop_user { DESKTOP_USER_RANK } else { own }
}

/// Loads each server once, from the highest-precedence source, using its whole entry (mcp.md, "Scope
/// hierarchy and precedence"). A server holds its endpoint against plugins and connectors only while
/// it is enabled ("one that points at the same URL or command as an enabled server above").
#[must_use]
pub fn rank_mcp_servers(candidates: &[JsValue], options: &RankOptions) -> McpRanking {
    let mut ordered: Vec<(f64, &JsValue)> = candidates
        .iter()
        .map(|candidate| (rank_of(candidate, candidates, options), candidate))
        .collect();
    ordered.sort_by(|(a, _), (b, _)| b.total_cmp(a));
    let mut held: Vec<(String, &JsValue)> = Vec::new();
    let mut loaded = Vec::new();
    let mut hidden = Vec::new();
    for (_, candidate) in ordered {
        let rule = rule_of(candidate);
        let identity = (rule.identify)(candidate);
        if let Some((_, above)) = held.iter().find(|(key, _)| *key == identity) {
            hidden.push(HiddenMcpServer {
                candidate: candidate.clone(),
                duplicate_of: (*above).clone(),
            });
            continue;
        }
        loaded.push(candidate.clone());
        if rule.holds_name {
            set(&mut held, by_name(candidate), candidate);
        }
        if member(candidate, "enabled") != &JsValue::Bool(false) {
            let endpoint = by_endpoint(candidate);
            if !held.iter().any(|(key, _)| *key == endpoint) {
                held.push((endpoint, candidate));
            }
        }
    }
    McpRanking { loaded, hidden }
}

/// `Map.set`: replaces the value of a key already held, else adds it.
fn set<'a>(held: &mut Vec<(String, &'a JsValue)>, key: String, candidate: &'a JsValue) {
    match held.iter_mut().find(|(existing, _)| *existing == key) {
        Some(entry) => entry.1 = candidate,
        None => held.push((key, candidate)),
    }
}

crate::registry::exports! {
    "settings/mcp/scope" {
        "MCP_SCOPES" => ok || MCP_SCOPES;
        "rankMcpServers" => ok |candidates: Vec<JsValue>, options: Option<RankOptions>|
            rank_mcp_servers(&candidates, &options.unwrap_or_default());
    }
}
