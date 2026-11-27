//! Port of `src/loading/resolve/mcp.ts`, switched.
//!
//! A plugin's MCP server config resolved for this host (plugins/manifest-reference.md, "Where each
//! variable resolves"; mcp.md, "Plugin-provided MCP servers"). The kit never starts anything: it hands
//! the host the substituted config, the variables the process receives and, for a server with a
//! `headersHelper`, the environment that helper runs with.

use std::collections::HashSet;

use indexmap::IndexMap;
use serde::Serialize;

use super::exported_environment::exported_environment;
use super::field_map::VariableComponent;
use super::field_runner::{FieldRunner, Part, field_runner, merge_unresolved};
use super::headers_helper::HelperServer;
use super::shell_exports::{ExportScope, shell_exports};
use super::shell_guard::{HEADERS_HELPER_GUARD, HeadersHelperContext, reject_user_config_in_shell};
use super::shell_prefix::{StdioStart, mcp_stdio_start};
use super::substitute::{SubstitutionScope, Unresolved, environment_names};
use super::substituter::{VariableSource, substituter};
use super::surfaces::SubstitutionSurface;
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::ids::plugin_name_of;
use crate::format::mcp_config::is_remote_mcp_server;
use crate::format::mcp_env::missing_variable_message;
use crate::js::{JsObject, JsValue, promise_all_pair};
use crate::ports::command_runner::CommandSpec;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// A plugin's MCP server config resolved for this host.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedMcpServer {
    /// The key as the plugin wrote it.
    pub name: String,
    /// What Claude Code registers it under: `plugin:<plugin>:<server>`.
    pub key: String,
    /// The plugin it belongs to, by the id the host uses for it (`SubstitutionScope.pluginId`): what a
    /// host keys a registration by. The same field on every resolved process (judgment, spec/judgments.md).
    pub plugin_id: String,
    /// The config with `${...}` substituted.
    pub server: JsValue,
    /// Variables the process receives besides its `env`: the three path variables for stdio, none for a remote server.
    pub environment: JsObject,
    /// The server as `planHeadersHelper` takes it (with origin `plugin`): the registered key, the URL as
    /// expanded, the environment variables the expansion substituted and the substituted command. The
    /// plan gives the helper's environment and directory. Absent when the server has no helper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers_helper: Option<HelperServer>,
    /// How a stdio server's process starts (`mcpStdioStart`): an exec of `command` and `args`, or under
    /// `CLAUDE_CODE_SHELL_PREFIX` the prefix with the quoted command line (spec §8.2.1, J8.26). Absent
    /// for a remote server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<CommandSpec>,
}

/// What resolving one server gave.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpResolution {
    /// `None` when the server does not start.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub resolved: Option<ResolvedMcpServer>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
    /// A remote server whose `url` is written empty: a placeholder for a connector the user configures
    /// later. It is listed as `not configured`, not connected, and not an error (mcp.md, "not
    /// configured"; spec §8.8 item 6). Its detail view reads `No URL configured for this server`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub not_configured: Option<bool>,
}

/// A remote entry with an empty `url`, as written: a placeholder, before any substitution.
const PLACEHOLDER: McpResolution = McpResolution {
    resolved: None,
    issues: Vec::new(),
    not_configured: Some(true),
};

/// `plugin:<plugin>:<server>` (components.md, "Server names, tool names, and reloads").
#[must_use]
pub fn mcp_server_key(plugin: &str, server: &str) -> String {
    format!("plugin:{plugin}:{server}")
}

/// A plugin identifier `name@marketplace` registers under its name; a bare name is its own.
fn plugin_name(id: &str) -> String {
    plugin_name_of(id)
}

fn invalid(name: &str, path: &str, why: &str) -> PluginIssue {
    error(
        path,
        &format!("Invalid MCP server config for \"{name}\": {why}"),
    )
}

fn refused(issues: Vec<PluginIssue>) -> McpResolution {
    McpResolution {
        resolved: None,
        issues,
        not_configured: None,
    }
}

/// The names of the `${VAR}` references no variable answered, once each, in the order they appear.
fn missing_variables(parts: &[Option<&Unresolved>]) -> Vec<String> {
    merge_unresolved(parts.iter().flatten().copied()).variables
}

/// An unset `${VAR}` with no default keeps a plugin's server from starting (plugins/troubleshooting.md,
/// "`Missing environment variables: <names>`": "set those variables in the shell you start Claude Code
/// from"). The per-variable warnings of the fields it names are dropped for the one error. A server of
/// the settings scopes loads with a warning instead (`expandMcpServer`, mcp.md, "Unset variables
/// without a default"). Judgment (spec/judgments.md): names are joined with a comma and a space.
fn unless_missing(
    name: &str,
    parts: &[Option<&Unresolved>],
    issues: &[PluginIssue],
) -> Option<Vec<PluginIssue>> {
    let missing = missing_variables(parts);
    if missing.is_empty() {
        return None;
    }
    let dropped: HashSet<String> = missing
        .iter()
        .map(|each| missing_variable_message(each))
        .collect();
    let mut rest: Vec<PluginIssue> = issues
        .iter()
        .filter(|found| !dropped.contains(&found.message))
        .cloned()
        .collect();
    let why = format!("Missing environment variables: {}", missing.join(", "));
    rest.push(invalid(name, &format!("mcpServers.{name}"), &why));
    Some(rest)
}

/// A string member of the config.
fn text_of(server: &JsValue, key: &str) -> Option<String> {
    server.get(key).and_then(JsValue::as_str).map(str::to_owned)
}

/// A list member of the config (`args`).
fn list_of(server: &JsValue, key: &str) -> Option<Vec<String>> {
    let items = server.get(key)?.as_array()?;
    Some(
        items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_owned))
            .collect(),
    )
}

/// A table member of the config (`env`, `headers`).
fn table_of(server: &JsValue, key: &str) -> Option<IndexMap<String, String>> {
    let members = server.get(key)?.as_object()?;
    Some(
        members
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|text| (key.clone(), text.to_owned())))
            .collect(),
    )
}

/// `{ ...server, [field]: part.value }` for each part that is there.
fn with_fields(server: &JsValue, fields: Vec<(&str, JsValue)>) -> JsValue {
    let mut members = server.as_object().cloned().unwrap_or_default();
    for (field, value) in fields {
        members.insert(field.to_owned(), value);
    }
    JsValue::Object(members)
}

fn list_value(values: &[String]) -> JsValue {
    JsValue::Array(values.iter().cloned().map(JsValue::String).collect())
}

fn table_value(values: &IndexMap<String, String>) -> JsValue {
    JsValue::Object(
        values
            .iter()
            .map(|(key, value)| (key.clone(), JsValue::String(value.clone())))
            .collect(),
    )
}

async fn maybe_list<L: Locations + Clone, E: Environment>(
    run: &FieldRunner<'_, '_, L, E>,
    surface: SubstitutionSurface,
    each: Option<Vec<String>>,
    field: &str,
) -> Option<Part<Vec<String>>> {
    match each {
        Some(each) => Some(run.list(surface, &each, field).await),
        None => None,
    }
}

async fn maybe_table<L: Locations + Clone, E: Environment>(
    run: &FieldRunner<'_, '_, L, E>,
    surface: SubstitutionSurface,
    each: Option<IndexMap<String, String>>,
    field: &str,
) -> Option<Part<IndexMap<String, String>>> {
    match each {
        Some(each) => Some(run.table(surface, &each, field).await),
        None => None,
    }
}

async fn maybe_one<L: Locations + Clone, E: Environment>(
    run: &FieldRunner<'_, '_, L, E>,
    surface: SubstitutionSurface,
    each: Option<String>,
    field: &str,
) -> Option<Part<String>> {
    match each {
        Some(each) => Some(run.one(surface, &each, field).await),
        None => None,
    }
}

fn issues_of<T>(part: Option<&Part<T>>) -> Vec<PluginIssue> {
    part.map(|part| part.issues.clone()).unwrap_or_default()
}

async fn resolve_stdio<L: Locations + Clone, E: Environment>(
    name: &str,
    server: &JsValue,
    scope: &SubstitutionScope<L, E>,
) -> McpResolution {
    let substitution = substituter(scope);
    let run = field_runner(&substitution, &format!("mcpServers.{name}"));
    let stdio = SubstitutionSurface::McpStdio;
    let source: &dyn VariableSource = &substitution;
    let command_text = text_of(server, "command").unwrap_or_default();
    let ((command, args), (env, environment)) = promise_all_pair(
        Box::pin(promise_all_pair(
            Box::pin(run.one(stdio, &command_text, "command")),
            Box::pin(maybe_list(&run, stdio, list_of(server, "args"), "args")),
        )),
        Box::pin(promise_all_pair(
            Box::pin(maybe_table(&run, stdio, table_of(server, "env"), "env")),
            Box::pin(exported_environment(
                VariableComponent::McpStdio,
                scope,
                Some(source),
            )),
        )),
    )
    .await;
    let mut issues = command.issues.clone();
    issues.extend(issues_of(args.as_ref()));
    issues.extend(issues_of(env.as_ref()));
    let parts = [
        Some(&command.unresolved),
        args.as_ref().map(|part| &part.unresolved),
        env.as_ref().map(|part| &part.unresolved),
    ];
    if let Some(failed) = unless_missing(name, &parts, &issues) {
        return refused(failed);
    }
    let mut fields = vec![("command", JsValue::String(command.value.clone()))];
    fields.extend(args.as_ref().map(|part| ("args", list_value(&part.value))));
    fields.extend(env.as_ref().map(|part| ("env", table_value(&part.value))));
    let start = StdioStart {
        command: command.value.clone(),
        args: Some(args.map(|part| part.value).unwrap_or_default()),
    };
    McpResolution {
        resolved: Some(ResolvedMcpServer {
            name: name.to_owned(),
            key: mcp_server_key(&plugin_name(&scope.plugin_id), name),
            plugin_id: scope.plugin_id.clone(),
            server: with_fields(server, fields),
            environment,
            headers_helper: None,
            start: Some(mcp_stdio_start(&start, &scope.environment, None).await),
        }),
        issues,
        not_configured: None,
    }
}

/// The helper's guard error, checked on the template before anything is substituted.
async fn helper_refusal(name: &str, server: &JsValue) -> Option<PluginIssue> {
    let path = format!("mcpServers.{name}.headersHelper");
    let template = text_of(server, "headersHelper")?;
    let context = HeadersHelperContext {
        server: name.to_owned(),
    };
    let refused =
        reject_user_config_in_shell(&HEADERS_HELPER_GUARD, &template, &context, &path).await?;
    Some(invalid(name, &path, &refused.message))
}

/// What `planHeadersHelper` needs of a plugin server (mcp.md, "dynamic headers"): the helper sees the
/// registered `plugin:<plugin>:<server>` key (judgment: the one `/mcp` shows), the URL as expanded and,
/// for redaction, the environment variables that expansion substituted.
fn helper_server(
    key: &str,
    url: &Part<String>,
    command: String,
    paths: IndexMap<String, String>,
) -> HelperServer {
    HelperServer {
        name: key.to_owned(),
        url: url.value.clone(),
        expanded_variables: environment_names(&url.referenced)
            .into_iter()
            .filter(|name| !url.unresolved.variables.contains(name))
            .collect(),
        headers_helper: Some(command),
        paths: Some(paths),
    }
}

async fn resolve_remote<L: Locations + Clone, E: Environment>(
    name: &str,
    server: &JsValue,
    scope: &SubstitutionScope<L, E>,
) -> McpResolution {
    let url_text = text_of(server, "url").unwrap_or_default();
    if url_text.is_empty() {
        return PLACEHOLDER;
    }
    if let Some(refusal) = helper_refusal(name, server).await {
        return refused(vec![refusal]);
    }
    let substitution = substituter(scope);
    let run = field_runner(&substitution, &format!("mcpServers.{name}"));
    let remote = SubstitutionSurface::McpRemote;
    let helper_surface = SubstitutionSurface::McpHeadersHelper;
    let ((url, headers), helper) = promise_all_pair(
        Box::pin(promise_all_pair(
            Box::pin(run.one(remote, &url_text, "url")),
            Box::pin(maybe_table(
                &run,
                remote,
                table_of(server, "headers"),
                "headers",
            )),
        )),
        Box::pin(maybe_one(
            &run,
            helper_surface,
            text_of(server, "headersHelper"),
            "headersHelper",
        )),
    )
    .await;
    let mut issues = url.issues.clone();
    issues.extend(issues_of(headers.as_ref()));
    issues.extend(issues_of(helper.as_ref()));
    let parts = [
        Some(&url.unresolved),
        headers.as_ref().map(|part| &part.unresolved),
        helper.as_ref().map(|part| &part.unresolved),
    ];
    if let Some(failed) = unless_missing(name, &parts, &issues) {
        return refused(failed);
    }
    // The `url` uses a `${user_config.*}` option that has no value (or no declaration).
    let unset = !url.unresolved.options.is_empty();
    if unset || crate::js::url::parse(&url.value).is_none() {
        let why = if unset {
            "URL is unset or invalid"
        } else {
            "has an invalid MCP url"
        };
        issues.push(invalid(name, &format!("mcpServers.{name}.url"), why));
        return refused(issues);
    }
    let key = mcp_server_key(&plugin_name(&scope.plugin_id), name);
    let paths = match &helper {
        Some(helper) => {
            let exports = ExportScope::from(scope);
            Some(shell_exports(helper_surface, &helper.referenced, &exports, &substitution).await)
        }
        None => None,
    };
    let mut fields = vec![("url", JsValue::String(url.value.clone()))];
    fields.extend(
        headers
            .as_ref()
            .map(|part| ("headers", table_value(&part.value))),
    );
    fields.extend(
        helper
            .as_ref()
            .map(|part| ("headersHelper", JsValue::String(part.value.clone()))),
    );
    let headers_helper =
        helper.map(|command| helper_server(&key, &url, command.value, paths.unwrap_or_default()));
    McpResolution {
        resolved: Some(ResolvedMcpServer {
            name: name.to_owned(),
            key,
            plugin_id: scope.plugin_id.clone(),
            server: with_fields(server, fields),
            environment: JsObject::new(),
            headers_helper,
            start: None,
        }),
        issues,
        not_configured: None,
    }
}

/// Resolves one server of a plugin. stdio: `command`, `args` and `env` are substituted and the process
/// gets the three path variables. http, sse and ws: `url`, `headers` and `headersHelper` are
/// substituted, nothing is exported, and a `headersHelper` that references `${user_config.*}` keeps the
/// server from starting. So does a `url` that does not parse after substitution (plugins/
/// troubleshooting.md): `URL is unset or invalid` when it uses a `${user_config.*}` option with no value,
/// `has an invalid MCP url` when the plugin's own `url` is at fault (judgment: the docs name both causes,
/// not how they are told apart). An unset `${VAR}` with no default keeps it from starting too, as
/// `Missing environment variables: <names>`, in whichever field it is named.
pub async fn resolve_mcp_server<L: Locations + Clone, E: Environment>(
    name: &str,
    server: &JsValue,
    scope: &SubstitutionScope<L, E>,
) -> McpResolution {
    if is_remote_mcp_server(server) {
        resolve_remote(name, server, scope).await
    } else {
        resolve_stdio(name, server, scope).await
    }
}

crate::registry::exports! {
    "loading/resolve/mcp" {
        "mcpServerKey" => ok |plugin: String, server: String| mcp_server_key(&plugin, &server);
        "resolveMcpServer" => async ok |name: String, server: JsValue, scope: SubstitutionScope|
            resolve_mcp_server(&name, &server, &scope).await;
    }
}
