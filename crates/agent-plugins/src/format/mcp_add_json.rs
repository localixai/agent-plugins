//! Port of `src/format/mcp-add-json.ts`, switched.
//!
//! `claude mcp add-json <name> '<json>'` and the part of `claude mcp add` that both share (mcp.md, "Add
//! MCP servers from JSON configuration", "Use pre-configured OAuth credentials", "Add a server from
//! setup instructions written for another client"): the input becomes a server entry, the OAuth
//! credentials the add carries, and what the host must ask for. The stored configuration, the keychain
//! and the prompt are the host's.

use serde::{Deserialize, Serialize};

use super::issues::{PluginIssue, Severity, error, warning};
use super::mcp_config::{read_mcp_server, signs_in_with_oauth};
use super::mcp_env::EnvLookup;
use super::mcp_oauth::{McpOAuthRead, read_mcp_oauth};
use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, number};
use crate::protocol::is_record::is_record;
use crate::registry::{Callback, Set};

/// The scopes an add writes to.
pub const MCP_ADD_SCOPES: [McpAddScope; 3] =
    [McpAddScope::Local, McpAddScope::Project, McpAddScope::User];

/// One of `MCP_ADD_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpAddScope {
    /// `local`.
    Local,
    /// `project`.
    Project,
    /// `user`.
    User,
}

impl McpAddScope {
    /// Its name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Project => "project",
            Self::User => "user",
        }
    }
}

/// Where the client secret comes from: "The `--client-secret` flag prompts", or `MCP_CLIENT_SECRET`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "source", rename_all = "lowercase")]
pub enum McpSecretRequest {
    /// The prompt.
    Prompt,
    /// The environment.
    Environment {
        /// The secret.
        value: String,
    },
}

/// What an add is given beside its entry.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAddOptions {
    /// `--scope`; local when absent.
    #[serde(default)]
    pub scope: Option<String>,
    /// `--client-secret`.
    #[serde(default)]
    pub client_secret: Option<bool>,
    /// The servers already stored, as `mcpAddKey(scope, name)`: an add of one of them fails.
    #[serde(default)]
    pub existing: Option<Set<String>>,
}

/// What an add gives.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct McpAddRead {
    /// The server's name.
    pub name: String,
    /// Its scope.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<McpAddScope>,
    /// Undefined when the add is refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<JsValue>,
    /// The OAuth setup that applies: the entry's `oauth` block, with `MCP_OAUTH_CALLBACK_PORT` filled in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth: Option<JsObject>,
    /// Present only with `--client-secret` on an HTTP or SSE server. Never stored in `server`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<McpSecretRequest>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

const SECRET_VARIABLE: &str = "MCP_CLIENT_SECRET";
const PORT_VARIABLE: &str = "MCP_OAUTH_CALLBACK_PORT";

/// Judgment (spec/judgments.md): "letters, numbers, hyphens, and underscores", the docs' own list.
fn is_name(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn refusal(name: &str, scope: Option<McpAddScope>, issue: PluginIssue) -> McpAddRead {
    McpAddRead {
        name: name.to_owned(),
        scope,
        server: None,
        oauth: None,
        secret: None,
        issues: vec![issue],
    }
}

/// The path of an issue about an entry being added.
fn path_of(name: &str) -> String {
    format!("mcpServers.{name}")
}

/// A scope, or why the text is none.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ScopeRead {
    /// The scope.
    Scope(McpAddScope),
    /// The refusal.
    Issue(PluginIssue),
}

/// The scope an add writes to: "Each command writes to local scope unless you add `--scope project` or `--scope user`".
#[must_use]
pub fn read_add_scope(name: &str, scope: Option<&str>) -> ScopeRead {
    let Some(scope) = scope else {
        return ScopeRead::Scope(McpAddScope::Local);
    };
    if let Some(found) = MCP_ADD_SCOPES.iter().find(|each| each.as_str() == scope) {
        return ScopeRead::Scope(*found);
    }
    let names: Vec<String> = MCP_ADD_SCOPES
        .iter()
        .map(|each| format!("\"{}\"", each.as_str()))
        .collect();
    let rest = names
        .get(..names.len().saturating_sub(1))
        .unwrap_or_default();
    let last = names.last().map_or("undefined", String::as_str);
    ScopeRead::Issue(error(
        &path_of(name),
        &format!(
            "Invalid scope \"{scope}\": use {} or {last}",
            rest.join(", ")
        ),
    ))
}

/// "Claude Code adds no server named like that": the message of `claude mcp add` for a name taken at that scope.
#[must_use]
pub fn mcp_server_exists_message(name: &str, scope: McpAddScope) -> String {
    format!(
        "MCP server {name} already exists in {} config",
        scope.as_str()
    )
}

/// How the host names a stored server for `McpAddOptions.existing`: the scope and the name, which
/// together make "the same server name at the same scope". A name has no `:` (see `NAME`).
#[must_use]
pub fn mcp_add_key(scope: McpAddScope, name: &str) -> String {
    format!("{}:{name}", scope.as_str())
}

/// A port written the way a redirect URI carries it. Judgment: anything else is no port.
fn port_of(text: Option<&str>) -> Option<f64> {
    let text = text.filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()))?;
    let port = number(text);
    (1.0..=65535.0).contains(&port).then_some(port)
}

fn without_oauth(entry: &JsObject) -> JsObject {
    entry
        .iter()
        .filter(|(key, _)| *key != "oauth")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

const NO_OAUTH: &str = "OAuth works with HTTP and SSE servers only: the oauth block and the OAuth flags have no effect on this server";

/// `--client-secret` asks for the secret; `MCP_CLIENT_SECRET` answers the prompt when it is set.
async fn secret_of(options: &McpAddOptions, lookup: &dyn EnvLookup) -> Option<McpSecretRequest> {
    if options.client_secret != Some(true) {
        return None;
    }
    Some(match lookup.lookup(SECRET_VARIABLE).await {
        None => McpSecretRequest::Prompt,
        Some(value) if value.is_empty() => McpSecretRequest::Prompt,
        Some(value) => McpSecretRequest::Environment { value },
    })
}

/// The block, with `MCP_OAUTH_CALLBACK_PORT` filled in when the block names no port.
async fn with_environment_port(block: &McpOAuthRead, lookup: &dyn EnvLookup) -> Option<JsObject> {
    let env_port = port_of(lookup.lookup(PORT_VARIABLE).await.as_deref());
    let unusable = block.issues.iter().any(|f| f.severity == Severity::Error);
    let named = block
        .oauth
        .as_ref()
        .and_then(|oauth| oauth.get("callbackPort"))
        .is_some_and(|port| !port.is_undefined());
    let port = if named { None } else { env_port };
    match port {
        Some(port) if !unusable => {
            let mut oauth = block.oauth.clone().unwrap_or_default();
            oauth.insert("callbackPort".to_owned(), JsValue::Number(port));
            Some(oauth)
        }
        _ => block.oauth.clone(),
    }
}

/// What an OAuth-bearing add means for one server.
struct OAuthSettled {
    oauth: Option<JsObject>,
    secret: Option<McpSecretRequest>,
    issues: Vec<PluginIssue>,
}

/// What an OAuth-bearing add means for one server, once its entry has been read.
async fn oauth_of(
    name: &str,
    entry: &JsObject,
    server: &JsValue,
    options: &McpAddOptions,
    lookup: &dyn EnvLookup,
) -> OAuthSettled {
    let path = path_of(name);
    let written = entry.get("oauth").unwrap_or(&JsValue::Undefined);
    if !signs_in_with_oauth(server) {
        let asked = !written.is_undefined() || options.client_secret == Some(true);
        let issues = if asked {
            vec![warning(
                &path,
                &format!("MCP server \"{name}\": {NO_OAUTH}"),
            )]
        } else {
            Vec::new()
        };
        return OAuthSettled {
            oauth: None,
            secret: None,
            issues,
        };
    }
    let block = read_mcp_oauth(name, written, &path);
    OAuthSettled {
        oauth: with_environment_port(&block, lookup).await,
        secret: secret_of(options, lookup).await,
        issues: block.issues,
    }
}

/// Where an add goes.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct McpAddTarget {
    /// The server's name.
    pub name: String,
    /// Its scope.
    pub scope: McpAddScope,
}

/// Finishes an add whose entry is known: refuses a name already stored at the scope, reads the entry as
/// a server and settles the OAuth setup and the secret. Judgments (spec/judgments.md): `add-json`
/// refuses a taken name as `add` does; `MCP_OAUTH_CALLBACK_PORT` fills the port only when the entry
/// names none; the secret is requested only by `--client-secret`, never by the environment variable
/// alone ("Avoids the interactive prompt when adding a server with `--client-secret`"); an invalid
/// `oauth` block is an error and leaves the entry itself read.
pub async fn complete_mcp_add(
    target: &McpAddTarget,
    entry: &JsObject,
    options: &McpAddOptions,
    lookup: &dyn EnvLookup,
) -> McpAddRead {
    let (name, scope) = (target.name.as_str(), target.scope);
    let key = mcp_add_key(scope, name);
    if options
        .existing
        .as_ref()
        .is_some_and(|set| set.members.contains(&key))
    {
        let issue = error(&path_of(name), &mcp_server_exists_message(name, scope));
        return refusal(name, Some(scope), issue);
    }
    let value = JsValue::Object(entry.clone());
    let read = read_mcp_server(name, &value, &path_of(name));
    let Some(server) = read.server else {
        return McpAddRead {
            name: name.to_owned(),
            scope: Some(scope),
            server: None,
            oauth: None,
            secret: None,
            issues: read.issues,
        };
    };
    let settled = oauth_of(name, entry, &server, options, lookup).await;
    // An `oauth` block that has no effect on this transport is not stored.
    let written = entry
        .get("oauth")
        .is_some_and(|oauth| !oauth.is_undefined());
    let stored = if written && settled.oauth.is_none() && !signs_in_with_oauth(&server) {
        let bare = JsValue::Object(without_oauth(entry));
        read_mcp_server(name, &bare, &path_of(name)).server
    } else {
        Some(server)
    };
    let mut issues = read.issues;
    issues.extend(settled.issues);
    McpAddRead {
        name: name.to_owned(),
        scope: Some(scope),
        server: stored,
        oauth: settled.oauth,
        secret: settled.secret,
        issues,
    }
}

fn wrapper(name: &str) -> String {
    format!(
        "MCP server \"{name}\": pass the object inside \"mcpServers\", not the wrapper that holds it"
    )
}

/// `claude mcp add-json <name> '<json>'`: "The JSON must conform to the MCP server configuration
/// schema" (`readMcpServer`). The wording of a refusal other than `Invalid MCP server config for
/// "<server>": <error>` and the two messages mcp.md quotes is the kit's (spec/judgments.md).
pub async fn read_mcp_add_json(
    name: &str,
    json: &str,
    options: &McpAddOptions,
    lookup: &dyn EnvLookup,
) -> McpAddRead {
    let scope = match read_add_scope(name, options.scope.as_deref()) {
        ScopeRead::Scope(scope) => scope,
        ScopeRead::Issue(issue) => return refusal(name, None, issue),
    };
    if !is_name(name) {
        let why = format!(
            "Invalid name \"{name}\": a server name uses only letters, numbers, hyphens and underscores"
        );
        return refusal(name, Some(scope), error(&path_of(name), &why));
    }
    let parsed = match json_parse(json) {
        Ok(parsed) => parsed,
        Err(why) => {
            let issue = error(&path_of(name), &format!("The JSON is not valid: {why}"));
            return refusal(name, Some(scope), issue);
        }
    };
    let Some(entry) = parsed.as_object().filter(|_| is_record(&parsed)) else {
        let issue = error(
            &path_of(name),
            "The JSON must be an object: the server entry",
        );
        return refusal(name, Some(scope), issue);
    };
    let absent = |key: &str| entry.get(key).is_none_or(JsValue::is_undefined);
    if entry.get("mcpServers").is_some_and(is_record) && absent("command") && absent("url") {
        return refusal(name, Some(scope), error(&path_of(name), &wrapper(name)));
    }
    let target = McpAddTarget {
        name: name.to_owned(),
        scope,
    };
    complete_mcp_add(&target, entry, options, lookup).await
}

crate::registry::exports! {
    "format/mcp-add-json" {
        "MCP_ADD_SCOPES" => ok || MCP_ADD_SCOPES;
        "mcpServerExistsMessage" => ok |name: String, scope: McpAddScope| mcp_server_exists_message(&name, scope);
        "mcpAddKey" => ok |scope: McpAddScope, name: String| mcp_add_key(scope, &name);
        "readMcpAddJson" => async ok |name: String, json: String, options: McpAddOptions, lookup: Callback|
            read_mcp_add_json(&name, &json, &options, &lookup).await;
    }
}
