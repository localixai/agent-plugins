//! Port of `src/host/core-servers.ts`, switched.
//!
//! A core package's MCP servers in the session's one tool namespace (Agent Plugins Extended §18.1.4,
//! J18.10, J18.11): registered under `plugin:<name>:<server>`, so their tools are
//! `mcp__plugin_<name>_<server>__<tool>` for the model, the permission rules, the hook matchers and
//! the policy lists. The loader validated each entry by core §7.2; here a valid one becomes a Claude
//! Code plugin server of the same transport with no optional field set: `stdio` as is, with its
//! `./` command resolved, `args`, `env` and `cwd` expanded (core §9.2) and the `cwd` contained;
//! `streamable-http` as `http`; `sse` as `sse`; `url` and `headers` never expanded. The process gets
//! `PLUGIN_ROOT`, `PLUGIN_DATA` and `CLAUDE_PROJECT_DIR` (§18.1.3, J18.12), no `CLAUDE_PLUGIN_*`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::core::mcp_server::{CoreMcpServer, CoreStdioServer, RemoteType};
use crate::format::core::placeholders::CorePaths;
use crate::format::core::subprocess::{CoreProcessInput, CoreServerProcess, core_server_process};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::ids::plugin_name_of;
use crate::js::{JsObject, JsValue};
use crate::loading::resolve::mcp::{ResolvedMcpServer, mcp_server_key};
use crate::loading::resolve::shell_prefix::{StdioStart, mcp_stdio_start};
use crate::ports::Signal;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// Where a core package's processes run: the session's locations and environment.
#[derive(Debug, Deserialize)]
pub struct CorePlace {
    /// The session's locations.
    pub locations: Port,
    /// The session's environment.
    pub environment: Port,
    /// Aborts the data directory's creation.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The two directories of core §9.1: the install's root and its data directory (J18.13).
pub async fn core_paths_of(plugin_id: &str, place: &CorePlace) -> CorePaths {
    CorePaths {
        root: place.locations.plugin_root(plugin_id).await,
        data: place
            .locations
            .plugin_data(plugin_id, place.signal.as_ref())
            .await,
    }
}

const RESERVED: [&str; 2] = ["PLUGIN_ROOT", "PLUGIN_DATA"];

/// A core server as a Claude Code plugin server, with the `core` facts the session starts it with.
#[derive(Clone, Debug, Serialize)]
pub struct CoreResolvedServer {
    /// The server as a plugin's.
    #[serde(flatten)]
    pub resolved: ResolvedMcpServer,
    /// A stdio server's working directory (`cwd`); empty for a remote one.
    pub core: JsObject,
}

/// One server, or why it is skipped.
type Made = Result<CoreResolvedServer, PluginIssue>;

/// The fields every server of the package shares.
struct Base {
    name: String,
    key: String,
    plugin_id: String,
}

fn resolved_of(base: Base, server: JsValue, environment: JsObject) -> ResolvedMcpServer {
    ResolvedMcpServer {
        name: base.name,
        key: base.key,
        plugin_id: base.plugin_id,
        server,
        environment,
        headers_helper: None,
        start: None,
    }
}

async fn stdio_server(
    base: Base,
    server: &CoreStdioServer,
    paths: &CorePaths,
    place: &CorePlace,
) -> Made {
    let input = CoreProcessInput {
        paths: paths.clone(),
        base: None,
        platform: None,
    };
    let launch = match core_server_process(server, &input) {
        CoreServerProcess::Problem(problem) => {
            let path = format!("mcpServers.{}", base.name);
            let message = format!("Server \"{}\" is skipped: {problem}", base.name);
            return Err(error(&path, &message));
        }
        CoreServerProcess::Process(launch) => launch,
    };
    // `env` is the server's own, expanded; the two reserved variables come in `environment`.
    let env: JsObject = launch
        .env
        .iter()
        .filter(|(name, _)| !RESERVED.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), JsValue::String(value.clone())))
        .collect();
    let args: Vec<JsValue> = launch.args.iter().cloned().map(JsValue::String).collect();
    let config = JsValue::Object(JsObject::from([
        ("type".to_owned(), JsValue::String("stdio".to_owned())),
        (
            "command".to_owned(),
            JsValue::String(launch.command.clone()),
        ),
        ("args".to_owned(), JsValue::Array(args)),
        ("env".to_owned(), JsValue::Object(env)),
    ]));
    let environment = JsObject::from([
        (
            "PLUGIN_ROOT".to_owned(),
            JsValue::String(paths.root.clone()),
        ),
        (
            "PLUGIN_DATA".to_owned(),
            JsValue::String(paths.data.clone()),
        ),
        (
            "CLAUDE_PROJECT_DIR".to_owned(),
            JsValue::String(place.locations.project_dir().await),
        ),
    ]);
    let start = StdioStart {
        command: launch.command.clone(),
        args: Some(launch.args.clone()),
    };
    let mut resolved = resolved_of(base, config, environment);
    resolved.start = Some(mcp_stdio_start(&start, &place.environment, None).await);
    let core = JsObject::from([("cwd".to_owned(), JsValue::String(launch.cwd))]);
    Ok(CoreResolvedServer { resolved, core })
}

/// The Claude Code transport each remote core transport runs as (§18.1.4 item 3).
fn remote_type(transport: RemoteType) -> &'static str {
    match transport {
        RemoteType::StreamableHttp => "http",
        RemoteType::Sse => "sse",
    }
}

async fn server_of(
    plugin_id: &str,
    name: &str,
    server: &CoreMcpServer,
    context: (&CorePaths, &CorePlace),
) -> Made {
    let base = Base {
        name: name.to_owned(),
        key: mcp_server_key(&plugin_name_of(plugin_id), name),
        plugin_id: plugin_id.to_owned(),
    };
    let remote = match server {
        CoreMcpServer::Stdio(stdio) => {
            return stdio_server(base, stdio, context.0, context.1).await;
        }
        CoreMcpServer::Remote(remote) => remote,
    };
    let mut config = JsObject::from([
        (
            "type".to_owned(),
            JsValue::String(remote_type(remote.transport).to_owned()),
        ),
        ("url".to_owned(), JsValue::String(remote.url.clone())),
    ]);
    if let Some(headers) = &remote.headers {
        let headers = headers
            .iter()
            .map(|(key, value)| (key.clone(), JsValue::String(value.clone())));
        config.insert("headers".to_owned(), JsValue::Object(headers.collect()));
    }
    let resolved = resolved_of(base, JsValue::Object(config), JsObject::new());
    Ok(CoreResolvedServer {
        resolved,
        core: JsObject::new(),
    })
}

/// A plugin as the servers read it: its id.
#[derive(Debug, Deserialize)]
pub struct CorePluginId {
    /// The id it runs under.
    pub id: String,
}

/// A core package as the servers read it: its servers, in the order `mcp.json` names them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreServersOf {
    /// The validated servers.
    #[serde(default)]
    pub mcp_servers: IndexMap<String, CoreMcpServer>,
}

/// What a package's servers resolve to.
#[derive(Debug, Default, Serialize)]
pub struct CoreServers {
    /// The servers that start.
    pub resolved: Vec<CoreResolvedServer>,
    /// The servers skipped.
    pub issues: Vec<PluginIssue>,
}

/// The servers of one core package, in the order `mcp.json` names them.
pub async fn core_servers(plugin_id: &str, core: &CoreServersOf, place: &CorePlace) -> CoreServers {
    if core.mcp_servers.is_empty() {
        return CoreServers::default();
    }
    let paths = core_paths_of(plugin_id, place).await;
    let mut made = CoreServers::default();
    for (name, server) in &core.mcp_servers {
        match server_of(plugin_id, name, server, (&paths, place)).await {
            Ok(resolved) => made.resolved.push(resolved),
            Err(issue) => made.issues.push(issue),
        }
    }
    made
}

crate::registry::exports! {
    "host/core-servers" {
        "coreServers" => async ok |plugin: CorePluginId, core: CoreServersOf, place: CorePlace|
            core_servers(&plugin.id, &core, &place).await;
    }
}
