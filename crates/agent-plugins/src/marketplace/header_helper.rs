//! Port of `src/marketplace/header-helper.ts`, switched.
//!
//! Running a `headersHelper` command (plugins/host-marketplace.md, "Write the headersHelper
//! command", "When Claude Code skips a headersHelper command or drops its output"): through the
//! `CommandRunner` port, from the configuration directory, under 10 seconds, printing one JSON
//! object of header names and string values. A command that fails means the fetch or download it
//! was run for does not happen: a `PluginIssue`, never a throw.
//!
//! `helperCache` stays at the edge, in the wrapper: it keeps the host's promises of runs still in
//! flight, so concurrent downloads on one origin share one run, and a promise is the host's. The
//! core asks a cache through [`HelperCache`], the host's by its `remember` method.

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::command_source::{WithSignal, shells};
use super::header_map::normalize_headers;
use super::header_policy::{HeaderLayer, strips_credentials};
use super::types::{CommandShell, ResolvedHeaders};
use crate::format::config_directory::{CONFIG_DIR_VARIABLE, JoinPath, config_directory_of};
use crate::format::credential_env::without_credentials;
use crate::format::helper_headers::{HelperHeadersProblem, HelperHeadersRead, read_helper_headers};
use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::plugin_source_kinds::command_text_issues;
use crate::format::path_text::PathPlatform;
use crate::js::number_to_string;
use crate::js::value::to_js_value;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Function, Port};

/// The host's ports a run needs (`HeadersHelperPorts`).
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct HeadersHelperPorts {
    /// Runs the command with `cwd` the configuration directory. Claude Code has that directory by
    /// the time a command runs, a missing one made at startup (oracle 2.1.322, spec/judgments.md),
    /// so a host whose configuration directory may not exist yet creates it before it runs the
    /// command.
    pub runner: Port,
    /// The environment the command inherits, less the credential variables for a command that was not the user's own.
    pub environment: Port,
    /// `homeDir()` and `joinPath` find the configuration directory (`~/.claude`) when `CLAUDE_CONFIG_DIR` is unset.
    pub locations: Port,
}

/// `Pick<HeadersSetting, 'platform'>`.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct HelperSetting {
    /// The host's platform.
    pub platform: PathPlatform,
}

/// The headers a command printed, or why there are none (`ResolvedHeaders | PluginIssue`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HelperOutcome {
    /// Why the fetch or download does not happen.
    Issue(PluginIssue),
    /// The headers.
    Resolved(ResolvedHeaders),
}

const HELPER_TIMEOUT_MS: f64 = 10_000.0;

fn problem(message: &str) -> PluginIssue {
    error("headersHelper", message)
}

/// "The working directory is the configuration directory, which is `~/.claude` or `CLAUDE_CONFIG_DIR`."
async fn config_directory(ports: &HeadersHelperPorts) -> String {
    let home = ports.locations.home_dir().await;
    let config_dir = ports.environment.get(CONFIG_DIR_VARIABLE).await;
    config_directory_of(&home, config_dir.as_deref(), &JoinPath(&ports.locations)).await
}

fn run_issue(result: &CommandResult) -> Option<PluginIssue> {
    if let Some(start_error) = &result.start_error {
        return Some(problem(&format!(
            "The headersHelper command could not be run: {start_error}"
        )));
    }
    if result.timed_out {
        return Some(problem(&format!(
            "The headersHelper command ran longer than {} seconds",
            number_to_string(HELPER_TIMEOUT_MS / 1000.0)
        )));
    }
    if result.exit_code != Some(0) {
        return Some(problem("The headersHelper command did not exit 0"));
    }
    None
}

/// "Print one JSON object of header names and string values on stdout."
fn read_headers(stdout: &str) -> HelperOutcome {
    match read_helper_headers(stdout) {
        HelperHeadersRead::Problem { problem: why } => HelperOutcome::Issue(problem(match why {
            HelperHeadersProblem::NotAnObject => {
                "The headersHelper command did not print a JSON object"
            }
            HelperHeadersProblem::NonStringValue => {
                "The headersHelper command printed a header whose value is not a string"
            }
        })),
        HelperHeadersRead::Headers { headers } => HelperOutcome::Resolved(ResolvedHeaders {
            headers: normalize_headers(&headers),
        }),
    }
}

/// Runs the layer's command and reads what it printed; a layer without a command prints no headers.
pub async fn run_headers_helper(
    layer: &HeaderLayer,
    ports: &HeadersHelperPorts,
    setting: HelperSetting,
    signal: &Signal,
) -> HelperOutcome {
    let Some(command) = layer.headers_helper.as_deref() else {
        return HelperOutcome::Resolved(ResolvedHeaders::default());
    };
    if let Some(refused) = command_text_issues(command, "headersHelper")
        .into_iter()
        .next()
    {
        return HelperOutcome::Issue(refused);
    }
    let inherited = ports.environment.snapshot().await;
    let mut env = if strips_credentials(layer) {
        without_credentials(&inherited)
    } else {
        inherited
    };
    env.extend(layer.variables.clone());
    let shell = if setting.platform == PathPlatform::Windows {
        CommandShell::Cmd
    } else {
        CommandShell::Sh
    };
    let result = ports
        .runner
        .run(CommandRun {
            spec: shells(shell, command),
            cwd: config_directory(ports).await,
            env,
            stdin: None,
            timeout_ms: Some(HELPER_TIMEOUT_MS),
            merge_stderr: None,
            signal: signal.clone(),
        })
        .await;
    run_issue(&result).map_or_else(|| read_headers(&result.stdout), HelperOutcome::Issue)
}

/// A cache of command runs (`HelperCache`): the run of `produce` for this layer's command, reused
/// for up to 60 seconds when the place allows; a failure is never kept.
pub trait HelperCache: Send + Sync {
    /// The run for `layer`: a kept one, or `produce`'s.
    fn remember<'a>(
        &'a self,
        layer: &'a HeaderLayer,
        produce: Produce,
    ) -> BoxFuture<'a, HelperOutcome>;
}

/// The run a cache may make: `runHeadersHelper` of one layer with the ports it was asked with. It
/// crosses to the host's cache as a function that returns a promise.
pub struct Produce {
    layer: HeaderLayer,
    ports: HeadersHelperPorts,
    setting: HelperSetting,
    signal: Signal,
}

impl Produce {
    /// The run of `layer` with `ports`.
    #[must_use]
    pub const fn new(
        layer: HeaderLayer,
        ports: HeadersHelperPorts,
        setting: HelperSetting,
        signal: Signal,
    ) -> Self {
        Self {
            layer,
            ports,
            setting,
            signal,
        }
    }

    /// Runs the command.
    pub async fn run(&self) -> HelperOutcome {
        run_headers_helper(&self.layer, &self.ports, self.setting, &self.signal).await
    }
}

crate::registry::methods! {
    Produce {
        "call" => async ok |this| this.run().await;
    }
}

/// The host's cache (`helperCache` of the wrapper, or the host's own): its `remember` method,
/// given the layer and the run as a function.
impl HelperCache for Port {
    fn remember<'a>(
        &'a self,
        layer: &'a HeaderLayer,
        produce: Produce,
    ) -> BoxFuture<'a, HelperOutcome> {
        let layer = to_js_value(layer).unwrap_or_default();
        let produce = to_js_value(&Function::new_async(Arc::new(produce))).unwrap_or_default();
        Box::pin(self.method::<HelperOutcome>("remember", vec![layer, produce]))
    }
}

crate::registry::exports! {
    "marketplace/header-helper" {
        "runHeadersHelper" => async ok
            |layer: HeaderLayer, ports: HeadersHelperPorts, setting: HelperSetting, options: WithSignal|
            run_headers_helper(&layer, &ports, setting, &options.signal).await;
    }
}
