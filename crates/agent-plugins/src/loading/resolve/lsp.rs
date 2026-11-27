//! Port of `src/loading/resolve/lsp.ts`, switched.
//!
//! An LSP server config resolved for this host.

use indexmap::IndexMap;
use serde::Serialize;

use super::exported_environment::exported_environment;
use super::field_map::VariableComponent;
use super::field_runner::{Part, field_runner};
use super::substitute::SubstitutionScope;
use super::substituter::{VariableSource, substituter};
use super::surfaces::SubstitutionSurface;
use crate::format::issues::PluginIssue;
use crate::format::lsp_config::LspServer;
use crate::js::{JsObject, JsValue};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// What Claude Code does when a field is unset (plugins/manifest-reference.md, "lspServers"):
/// `requestTimeout` 60000 milliseconds, `restartOnCrash` true, `diagnostics` true. The docs name no
/// default for `startupTimeout` or `maxRestarts`, and "when unset, no timeout applies" for
/// `shutdownTimeout`: those stay unset in the resolved server.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LspDefaults {
    /// `requestTimeout`.
    pub request_timeout_ms: f64,
    /// `restartOnCrash`.
    pub restart_on_crash: bool,
    /// `diagnostics`.
    pub diagnostics: bool,
}

/// The defaults.
pub const LSP_DEFAULTS: LspDefaults = LspDefaults {
    request_timeout_ms: 60000.0,
    restart_on_crash: true,
    diagnostics: true,
};

/// An LSP server config resolved for this host. The kit never starts a language server: the host's
/// LSP client reads this (command line, environment, timeouts, restart policy, the two payloads)
/// and runs it. The decisions the kit owns are in `lsp-runtime.ts`.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedLspServer {
    /// The key as the plugin wrote it.
    pub name: String,
    /// The plugin it belongs to, by the id the host uses for it (`SubstitutionScope.pluginId`):
    /// what a host keys a registration by. The same field on every resolved process (judgment,
    /// spec/judgments.md).
    pub plugin_id: String,
    /// `command` with `${...}` substituted.
    pub command: String,
    /// The arguments.
    pub args: Vec<String>,
    /// The server's own `env`, substituted; empty when it declares none.
    pub env: IndexMap<String, String>,
    /// The variables the process receives besides `env`: the three path variables.
    pub environment: JsObject,
    /// The workspace folder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_folder: Option<String>,
    /// Always `stdio`: "Claude Code accepts `socket` but runs every server over stdio".
    pub transport: &'static str,
    /// The file extensions' languages.
    pub extension_to_language: IndexMap<String, String>,
    /// Not substituted: `${...}` does not resolve there (the table of fields lists four).
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub initialization_options: JsValue,
    /// The settings.
    #[serde(skip_serializing_if = "JsValue::is_undefined")]
    pub settings: JsValue,
    /// The startup timeout.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_timeout_ms: Option<f64>,
    /// Undefined means no timeout: the host waits for a graceful shutdown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shutdown_timeout_ms: Option<f64>,
    /// The request timeout.
    pub request_timeout_ms: f64,
    /// Whether it restarts on a crash.
    pub restart_on_crash: bool,
    /// Undefined means unbounded (judgment J-L2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_restarts: Option<f64>,
    /// Whether it reports diagnostics.
    pub diagnostics: bool,
}

/// One server resolved.
#[derive(Clone, Debug, Serialize)]
pub struct LspResolution {
    /// The server.
    pub resolved: ResolvedLspServer,
    /// The issues.
    pub issues: Vec<PluginIssue>,
}

/// Resolves one server of a plugin (plugins/manifest-reference.md, "Where each variable
/// resolves"): `command`, `args`, `env` and `workspaceFolder` are substituted, the process gets the
/// three path variables (and the data folder is created for it), every other field is handed over
/// as written with the documented defaults applied.
///
/// Judgment: `${user_config.KEY}` is read only in those four fields, although the page also says
/// "substituted in ... LSP server config": its table of fields where `${...}` resolves is the finer
/// statement, and `initializationOptions` and `settings` are data for the server, not the kit's
/// text.
pub async fn resolve_lsp_server<L: Locations + Clone, E: Environment>(
    name: &str,
    server: &LspServer,
    scope: &SubstitutionScope<L, E>,
) -> LspResolution {
    let substitution = substituter(scope);
    let run = field_runner(&substitution, &format!("lspServers.{name}"));
    let lsp = SubstitutionSurface::Lsp;
    let source: &dyn VariableSource = &substitution;
    let (command, args, env, workspace_folder, environment) = join5(
        run.one(lsp, &server.command, "command"),
        async {
            match &server.args {
                Some(each) => Some(run.list(lsp, each, "args").await),
                None => None,
            }
        },
        async {
            match &server.env {
                Some(each) => Some(run.table(lsp, each, "env").await),
                None => None,
            }
        },
        async {
            match &server.workspace_folder {
                Some(each) => Some(run.one(lsp, each, "workspaceFolder").await),
                None => None,
            }
        },
        exported_environment(VariableComponent::Lsp, scope, Some(source)),
    )
    .await;
    let mut issues = command.issues.clone();
    issues.extend(args.iter().flat_map(|part| part.issues.clone()));
    issues.extend(env.iter().flat_map(|part| part.issues.clone()));
    issues.extend(workspace_folder.iter().flat_map(|part| part.issues.clone()));
    LspResolution {
        resolved: ResolvedLspServer {
            name: name.to_owned(),
            plugin_id: scope.plugin_id.clone(),
            command: command.value,
            args: args.map(|part| part.value).unwrap_or_default(),
            env: env.map(|part| part.value).unwrap_or_default(),
            environment,
            workspace_folder: workspace_folder.map(|part: Part<String>| part.value),
            transport: "stdio",
            extension_to_language: server.extension_to_language.clone(),
            initialization_options: server.initialization_options.clone(),
            settings: server.settings.clone(),
            startup_timeout_ms: server.startup_timeout,
            shutdown_timeout_ms: server.shutdown_timeout,
            request_timeout_ms: server
                .request_timeout
                .unwrap_or(LSP_DEFAULTS.request_timeout_ms),
            restart_on_crash: server
                .restart_on_crash
                .unwrap_or(LSP_DEFAULTS.restart_on_crash),
            max_restarts: server.max_restarts,
            diagnostics: server.diagnostics.unwrap_or(LSP_DEFAULTS.diagnostics),
        },
        issues,
    }
}

/// `Promise.all` of five futures of different types: they run together.
async fn join5<A, B, C, D, F>(
    a: impl Future<Output = A> + Send,
    b: impl Future<Output = B> + Send,
    c: impl Future<Output = C> + Send,
    d: impl Future<Output = D> + Send,
    f: impl Future<Output = F> + Send,
) -> (A, B, C, D, F)
where
    A: Send,
    B: Send,
    C: Send,
    D: Send,
    F: Send,
{
    use crate::js::promise_all_pair;
    let ((a, b), ((c, d), f)) = promise_all_pair(
        Box::pin(promise_all_pair(Box::pin(a), Box::pin(b))),
        Box::pin(promise_all_pair(
            Box::pin(promise_all_pair(Box::pin(c), Box::pin(d))),
            Box::pin(f),
        )),
    )
    .await;
    (a, b, c, d, f)
}

crate::registry::exports! {
    "loading/resolve/lsp" {
        "LSP_DEFAULTS" => ok || LSP_DEFAULTS;
        "resolveLspServer" => async ok |name: String, server: LspServer, scope: SubstitutionScope|
            resolve_lsp_server(&name, &server, &scope).await;
    }
}
