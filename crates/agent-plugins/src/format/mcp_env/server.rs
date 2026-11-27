//! `src/format/mcp-env.ts`, its second half: one server's fields expanded, and what the surfaces
//! show of a server.

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

use super::{EnvLookup, ReadsAsEmpty, expand_env_text, missing_variable_message};
use crate::format::credential_env::{credential_debug_line, reads_as_empty_toward_remote};
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::mcp_config::is_remote_mcp_server;
use crate::js::{JsObject, JsValue, array_join, string_of, url};
use crate::ports::BoxFuture;
use crate::registry::{Callback, Port};

/// One server expanded.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpExpansion {
    /// Undefined when the server cannot start: its `url` is not a URL after expansion.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<JsValue>,
    /// A warning per field with an unset variable; an error when the server is dropped.
    pub issues: Vec<PluginIssue>,
    /// The unset variables without a default, once each (the `claude mcp list` warning).
    pub missing: Vec<String>,
    /// Debug-log lines for covered credentials the user set and a remote `url` or `headers` named.
    pub debug: Vec<String>,
    /// A remote server whose `url` is written empty: `not configured`, not connected and not an error
    /// (mcp.md: "Claude Code doesn't attempt to connect to it"; spec §8.8 item 6). A `url` that becomes
    /// empty or unusable only after expansion is still `'url' is not a valid URL`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_configured: Option<bool>,
}

/// What a host adds to the expansion of one server: the record holds `readsAsEmpty`, a function,
/// so it crosses as the host's (`Host`), or as plain data when it holds none.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum McpExpansionOptions {
    /// The host's record: `readsAsEmpty` is read from it.
    Host(Port),
    /// No function in it.
    Data(JsValue),
}

impl Default for McpExpansionOptions {
    fn default() -> Self {
        Self::Data(JsValue::Undefined)
    }
}

impl McpExpansionOptions {
    /// More variable names that read as empty toward a remote server, on top of the kit's own list
    /// (mcp.md, "Credential variables that read as empty"); it widens the list and never narrows it.
    /// Judgment (spec/judgments.md): a host stricter than Claude Code withholds more of its environment.
    async fn reads_as_empty(&self) -> Option<Callback> {
        match self {
            Self::Host(options) => options.member::<Option<Callback>>("readsAsEmpty").await,
            Self::Data(_) => None,
        }
    }
}

/// The kit's credential list, widened by the host's.
struct Covered(Option<Callback>);

impl ReadsAsEmpty for Covered {
    fn reads_as_empty<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            if reads_as_empty_toward_remote(name) {
                return true;
            }
            match &self.0 {
                Some(wider) => wider.reads_as_empty(name).await,
                None => false,
            }
        })
    }
}

/// Collects what the fields of one server raise, in the order they are expanded.
struct Fields<'a> {
    name: &'a str,
    lookup: &'a dyn EnvLookup,
    reads_as_empty: Option<Covered>,
    issues: Vec<PluginIssue>,
    missing: IndexSet<String>,
    credentials: IndexSet<String>,
}

impl Fields<'_> {
    async fn one(&mut self, field: &str, value: &str) -> String {
        let reads = self.reads_as_empty.as_ref().map(|c| c as &dyn ReadsAsEmpty);
        let done = expand_env_text(value, self.lookup, reads).await;
        for each in done.missing {
            let path = format!("mcpServers.{}.{field}", self.name);
            self.issues
                .push(warning(&path, &missing_variable_message(&each)));
            self.missing.insert(each);
        }
        self.credentials.extend(done.credentials);
        done.text
    }

    /// Each text of a list expanded (a member that is no text kept as it is).
    async fn list(&mut self, field: &str, values: &JsValue) -> JsValue {
        let mut items = Vec::new();
        for value in values.as_array().into_iter().flatten() {
            items.push(match value.as_str() {
                Some(text) => self.one(field, text).await.into(),
                None => value.clone(),
            });
        }
        JsValue::Array(items)
    }

    /// Each value of a table expanded.
    async fn table(&mut self, field: &str, entries: &JsValue) -> JsValue {
        let mut table = JsObject::new();
        for (key, value) in entries.as_object().into_iter().flatten() {
            let expanded = match value.as_str() {
                Some(text) => self.one(&format!("{field}.{key}"), text).await.into(),
                None => value.clone(),
            };
            table.insert(key.clone(), expanded);
        }
        JsValue::Object(table)
    }

    async fn field(&mut self, server: &mut JsObject, key: &str) {
        if let Some(JsValue::String(text)) = server.get(key).cloned() {
            let expanded = self.one(key, &text).await;
            server.insert(key.to_owned(), expanded.into());
        }
    }

    async fn nested(&mut self, server: &mut JsObject, key: &str, table: bool) {
        let Some(value) = server.get(key).filter(|v| !v.is_undefined()).cloned() else {
            return;
        };
        let expanded = if table {
            self.table(key, &value).await
        } else {
            self.list(key, &value).await
        };
        server.insert(key.to_owned(), expanded);
    }
}

async fn expand_stdio(server: &JsObject, fields: &mut Fields<'_>) -> JsObject {
    let mut expanded = server.clone();
    fields.field(&mut expanded, "command").await;
    fields.nested(&mut expanded, "args", false).await;
    fields.nested(&mut expanded, "env", true).await;
    expanded
}

async fn expand_remote(server: &JsObject, fields: &mut Fields<'_>) -> JsObject {
    let mut expanded = server.clone();
    fields.field(&mut expanded, "url").await;
    fields.nested(&mut expanded, "headers", true).await;
    expanded
}

/// A server of the local, project or user scope with its references expanded (mcp.md, "Environment
/// variable expansion in `.mcp.json`"): `command`, `args` and `env` of a stdio server; `url` and
/// `headers` of a remote one (`headersHelper` is a command the process runs, so it is left as
/// written). An unset variable without a default keeps its text and is a warning; a covered
/// credential reads as empty in a remote server only.
///
/// Judgment (spec/judgments.md): a `url` that does not parse after expansion drops the server, whatever
/// the cause, with the docs' `'url' is not a valid URL` (the docs show it for one that expands to
/// `/mcp`); a `url` written empty is a placeholder (`notConfigured`), not an error.
pub async fn expand_mcp_server(
    name: &str,
    server: &JsValue,
    lookup: &dyn EnvLookup,
    options: McpExpansionOptions,
) -> McpExpansion {
    let remote = is_remote_mcp_server(server);
    if remote && server.get("url") == Some(&JsValue::from("")) {
        return McpExpansion {
            server: None,
            issues: Vec::new(),
            missing: Vec::new(),
            debug: Vec::new(),
            not_configured: Some(true),
        };
    }
    let wider = if remote {
        options.reads_as_empty().await
    } else {
        None
    };
    let mut fields = Fields {
        name,
        lookup,
        reads_as_empty: remote.then_some(Covered(wider)),
        issues: Vec::new(),
        missing: IndexSet::new(),
        credentials: IndexSet::new(),
    };
    let members = server.as_object().cloned().unwrap_or_default();
    let expanded = if remote {
        expand_remote(&members, &mut fields).await
    } else {
        expand_stdio(&members, &mut fields).await
    };
    let expanded = JsValue::Object(expanded);
    let unusable = is_remote_mcp_server(&expanded)
        && url::parse(&string_of(
            expanded.get("url").unwrap_or(&JsValue::Undefined),
        ))
        .is_none();
    let mut issues = fields.issues;
    if unusable {
        issues.push(error(
            &format!("mcpServers.{name}.url"),
            &format!("Invalid MCP server config for \"{name}\": 'url' is not a valid URL"),
        ));
    }
    McpExpansion {
        server: (!unusable).then_some(expanded),
        issues,
        missing: fields.missing.into_iter().collect(),
        debug: fields
            .credentials
            .iter()
            .map(|one| credential_debug_line(one))
            .collect(),
        not_configured: None,
    }
}

/// Where a configured server comes from, as far as what the surfaces show is concerned.
pub const MCP_DISPLAY_SCOPES: [McpDisplayScope; 4] = [
    McpDisplayScope::Local,
    McpDisplayScope::Project,
    McpDisplayScope::User,
    McpDisplayScope::Managed,
];

/// One of `MCP_DISPLAY_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpDisplayScope {
    /// `local`.
    Local,
    /// `project`.
    Project,
    /// `user`.
    User,
    /// `managed`.
    Managed,
}

/// managed-mcp.md, "What users can see and change": "`https://mcp.example.com/…`".
fn host_only(url: &str) -> String {
    match url::parse(url) {
        None => "…".to_owned(),
        Some(parsed) => format!("{}//{}/…", parsed.protocol, parsed.host),
    }
}

/// `REMOTE_DISPLAY`: the URL as written, or its host only.
fn remote_display(scope: McpDisplayScope, url: &str) -> String {
    match scope {
        McpDisplayScope::Local | McpDisplayScope::Project | McpDisplayScope::User => url.to_owned(),
        McpDisplayScope::Managed => host_only(url),
    }
}

/// What `/mcp`, `claude mcp list` and `claude mcp get` show of a server's URL or command line (mcp.md,
/// "How references appear in `/mcp` and CLI output"): for a server of the local, project or user
/// scope the `${VAR}` reference by name, never its resolved value; for a server the organization
/// provides, the URL's host only. Judgment: the host is the URL's host component (with its port) and
/// a URL that does not parse shows nothing of itself. A managed entry has no `command` (managed-mcp.md,
/// "What an entry can contain"), so a command line is shown as written in every scope.
///
/// `server` is the entry as written, never the result of `expandMcpServer`: the function shows what it
/// is given, so an expanded server would show the resolved value.
#[must_use]
pub fn display_reference(server: &JsValue, scope: McpDisplayScope) -> String {
    let member = |key: &str| server.get(key).cloned().unwrap_or_default();
    if is_remote_mcp_server(server) {
        return remote_display(scope, &string_of(&member("url")));
    }
    let mut line = vec![member("command")];
    if let Some(JsValue::Array(args)) = server.get("args") {
        line.extend(args.iter().cloned());
    }
    array_join(&line, " ")
}

/// One header a surface shows: its name, and its value when the scope shows it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DisplayedHeader {
    /// Its name.
    pub name: String,
    /// Its value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<JsValue>,
}

/// What `claude mcp get` shows of a remote server's headers. For a server the organization provides:
/// "its header names without their values" (managed-mcp.md, "What users can see and change"). Judgment:
/// the docs give no rule for the other scopes, so a header there is shown as written (a `${VAR}`
/// reference by name, as for the URL; pass the entry as written, as for `displayReference`). A stdio
/// server has no headers.
#[must_use]
pub fn display_headers(server: &JsValue, scope: McpDisplayScope) -> Vec<DisplayedHeader> {
    if !is_remote_mcp_server(server) {
        return Vec::new();
    }
    let headers = server.get("headers").and_then(JsValue::as_object);
    headers
        .into_iter()
        .flatten()
        .map(|(name, value)| DisplayedHeader {
            name: name.clone(),
            value: (scope != McpDisplayScope::Managed).then(|| value.clone()),
        })
        .collect()
}
