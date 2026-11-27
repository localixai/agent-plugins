//! Port of `src/settings/mcp/allow-deny.ts`, switched.
//!
//! The allowlist and the denylist over one server (managed-mcp.md, "How a server is evaluated").

use serde::{Deserialize, Serialize};

use crate::format::mcp_config::is_remote_mcp_server;
use crate::format::mcp_env::{McpExpansionOptions, expand_mcp_server};
use crate::js::JsValue;
use crate::js::value::JsObject;
use crate::registry::{Handle, Port};
use crate::settings::mcp::command_match::command_matches;
use crate::settings::mcp::entries::McpEntry;
use crate::settings::mcp::entry_env::{
    EntryExpansion, PolicyEnv, entry_expansions, lookup_in, policy_env_of,
};
use crate::settings::mcp::lists::{McpEntriesByKind, McpLists, read_mcp_lists};
use crate::settings::mcp::origins::ORIGIN_RULES;
use crate::settings::mcp::url_pattern::{reference_in_url_scope, url_pattern_matches};
use crate::settings::view::PolicyView;

/// The environments a policy reads.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct McpPolicyEnv {
    /// The environment Claude Code started with: the base of what a policy entry expands in.
    pub startup: JsObject,
    /// The live process environment: what the server's own `${VAR}` expands in.
    pub live: JsObject,
}

/// One server to check.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct McpPolicySubject {
    /// The label the server goes by: the key in its file, or a claude.ai connector's display name.
    pub name: String,
    /// The server.
    pub server: JsValue,
    /// Defaults to `configured`: a server a user, a plugin or claude.ai adds.
    #[serde(default)]
    pub origin: Option<String>,
}

/// Whether the server may load, and why not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpServerVerdict {
    /// It may load.
    pub allowed: bool,
    /// `denylist` or `allowlist`, when blocked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<&'static str>,
    /// The status of the server in `claude mcp list` and `claude mcp get`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<&'static str>,
    /// What `claude mcp add` says (managed-mcp.md, "How restrictions appear to users").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_error: Option<String>,
    /// What a reconnect says.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reconnect_error: Option<String>,
}

const STATUS: &str = "Blocked by enterprise managed policy";

fn add_reason(blocked_by: &str) -> &'static str {
    if blocked_by == "denylist" {
        "server is explicitly blocked by enterprise policy"
    } else {
        "not allowed by enterprise policy"
    }
}

const ALLOWED: McpServerVerdict = McpServerVerdict {
    allowed: true,
    blocked_by: None,
    status: None,
    add_error: None,
    reconnect_error: None,
};

fn blocked(blocked_by: &'static str, name: &str) -> McpServerVerdict {
    McpServerVerdict {
        allowed: false,
        blocked_by: Some(blocked_by),
        status: Some(STATUS),
        add_error: Some(format!(
            "Cannot add MCP server \"{name}\": {}",
            add_reason(blocked_by)
        )),
        reconnect_error: Some(format!(
            "MCP server {name} is blocked by enterprise managed policy"
        )),
    }
}

/// The endpoint after the server's own expansion.
enum Endpoint {
    Url(String),
    Command(Vec<String>),
}

/// What an entry is matched against: the label, and the endpoint after the server's own expansion.
struct Target {
    name: String,
    endpoint: Option<Endpoint>,
}

struct ListRules<'a> {
    expansion: &'a EntryExpansion,
    /// An entry with a `${VAR}` in its URL's scheme, host or path never matches (allowlist only).
    ignores_scope_references: bool,
}

/// `MATCHERS[entry.kind]`.
async fn matches(entry: &McpEntry, target: &Target, list: &ListRules<'_>) -> bool {
    match entry {
        // "Exact match only; wildcards are not expanded", and names "never expand".
        McpEntry::ServerName(value) => *value == target.name,
        McpEntry::ServerCommand(value) => match &target.endpoint {
            Some(Endpoint::Command(command)) => {
                command_matches(value, command, list.expansion).await
            }
            _ => false,
        },
        McpEntry::ServerUrl(value) => match &target.endpoint {
            Some(Endpoint::Url(url)) => {
                !(list.ignores_scope_references && reference_in_url_scope(value))
                    && url_pattern_matches(&list.expansion.expand(value), url)
            }
            _ => false,
        },
    }
}

/// The entry kind that admits a server with this endpoint (`KIND_OF_ENDPOINT`), or the names when
/// the list holds none of it.
fn entries_for<'a>(allowed: &'a McpEntriesByKind, target: &Target) -> &'a [McpEntry] {
    let own = match &target.endpoint {
        None => return &allowed.server_name,
        Some(Endpoint::Url(_)) => &allowed.server_url,
        Some(Endpoint::Command(_)) => &allowed.server_command,
    };
    if own.is_empty() {
        &allowed.server_name
    } else {
        own
    }
}

fn texts(value: Option<&JsValue>) -> Vec<String> {
    match value {
        Some(JsValue::Array(items)) => items.iter().map(crate::js::array_join::string_of).collect(),
        _ => Vec::new(),
    }
}

/// The target of a server: its command or URL with its own `${VAR}` expanded in the live
/// environment.
async fn target_of(subject: &McpPolicySubject, live: &PolicyEnv) -> Target {
    let lookup = |name: &str| lookup_in(live, name);
    let expanded = expand_mcp_server(
        &subject.name,
        &subject.server,
        &lookup,
        McpExpansionOptions::default(),
    )
    .await
    .server;
    let Some(expanded) = expanded else {
        return Target {
            name: subject.name.clone(),
            endpoint: None,
        };
    };
    let endpoint = if is_remote_mcp_server(&expanded) {
        Endpoint::Url(crate::js::array_join::string_of(
            expanded.get("url").unwrap_or(&JsValue::Undefined),
        ))
    } else {
        let mut command = vec![crate::js::array_join::string_of(
            expanded.get("command").unwrap_or(&JsValue::Undefined),
        )];
        command.extend(texts(expanded.get("args")));
        Endpoint::Command(command)
    };
    Target {
        name: subject.name.clone(),
        endpoint: Some(endpoint),
    }
}

/// The lists, read once, when the policy is created, so one policy gives the same verdict for a
/// server until it is dropped: a host that sees settings change before a user reconnects a server
/// (the docs run the checks again then) builds a new policy, as `McpSettingsPolicy.serverPolicy()`
/// does.
struct McpServerPolicy {
    lists: McpLists,
    allowed: EntryExpansion,
    denied: EntryExpansion,
    live: PolicyEnv,
}

impl McpServerPolicy {
    /// Whether the server may load, and why not.
    async fn check(&self, subject: &McpPolicySubject) -> McpServerVerdict {
        let origin = subject.origin.as_deref().unwrap_or("configured");
        let Some((_, rule)) = ORIGIN_RULES.iter().find(|(name, _)| *name == origin) else {
            return ALLOWED;
        };
        if !rule.denylist {
            return ALLOWED;
        }
        let target = target_of(subject, &self.live).await;
        let denied = ListRules {
            expansion: &self.denied,
            ignores_scope_references: false,
        };
        for entry in &self.lists.denied {
            if matches(entry, &target, &denied).await {
                return blocked("denylist", &subject.name);
            }
        }
        let Some(allowed) = &self.lists.allowed else {
            return ALLOWED;
        };
        if rule.skips_allowlist.skips(&subject.server) {
            return ALLOWED;
        }
        let rules = ListRules {
            expansion: &self.allowed,
            ignores_scope_references: true,
        };
        for entry in entries_for(allowed, &target) {
            if matches(entry, &target, &rules).await {
                return ALLOWED;
            }
        }
        blocked("allowlist", &subject.name)
    }
}

/// The allowlist and the denylist over one server, in the order the page gives: merge the lists
/// (`lists.ts`), "check the denylist" ("Nothing overrides a denylist match"), "check the
/// allowlist" ("If `allowedMcpServers` isn't set anywhere, every server that passed the denylist
/// loads"). A remote server is allowed by a `serverUrl` entry, a stdio server by a `serverCommand`
/// entry, and a `serverName` match counts only when the list holds no entry of the server's kind.
/// The origins that skip a check are `origins.ts`.
async fn create_mcp_server_policy(view: &dyn PolicyView, env: &McpPolicyEnv) -> McpServerPolicy {
    let lists = read_mcp_lists(view).await;
    let expansions = entry_expansions(view, &policy_env_of(&env.startup)).await;
    McpServerPolicy {
        lists,
        allowed: expansions.allowed,
        denied: expansions.denied,
        live: policy_env_of(&env.live),
    }
}

crate::registry::methods! {
    McpServerPolicy {
        "check" => sync async ok |this, subject: McpPolicySubject| this.check(&subject).await;
    }
}

crate::registry::exports! {
    "settings/mcp/allow-deny" {
        "createMcpServerPolicy" => async ok |view: Port, env: McpPolicyEnv|
            Handle::new(create_mcp_server_policy(&view, &env).await);
    }
}
