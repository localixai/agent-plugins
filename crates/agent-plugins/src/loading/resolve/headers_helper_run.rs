//! Port of `src/loading/resolve/headers-helper-run.ts`, switched.
//!
//! Running a server's `headersHelper` and using what it prints (mcp.md, "Use dynamic headers for
//! custom authentication"). The kit starts nothing itself: the command goes through `CommandRunner`,
//! and the connection is the host's, handed in as a function.
//!
//! The response `send` answers is the host's: one that holds a function crosses as a value the host
//! keeps and is given back as that very value; a plain record is read and given back as data. The
//! core only reads its `status` and compares nothing by identity.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::headers_helper::HEADERS_HELPER_TIMEOUT_MS;
use crate::format::helper_headers::{
    HelperHeadersProblem, HelperHeadersRead, has_authorization_header, read_helper_headers,
};
use crate::format::issues::{PluginIssue, error};
use crate::js::{JsValue, in_key_order, number_to_string};
use crate::ports::Signal;
use crate::ports::command_runner::{
    CommandResult, CommandRun, CommandRunner, CommandSpec, ShellKind,
};
use crate::registry::{Callback, Port};

fn problem(server: &str, message: &str) -> PluginIssue {
    error(&format!("mcpServers.{server}.headersHelper"), message)
}

/// Why the run produced no usable output, or `None` when it ran and exited 0.
fn failure(result: &CommandResult) -> Option<String> {
    if let Some(start_error) = &result.start_error {
        return Some(format!(
            "The headersHelper command could not be run: {start_error}"
        ));
    }
    if result.timed_out {
        return Some(format!(
            "The headersHelper command ran longer than {} seconds",
            HEADERS_HELPER_TIMEOUT_MS / 1000
        ));
    }
    (result.exit_code != Some(0)).then(|| "The headersHelper command did not exit 0".to_owned())
}

const fn output_problem(problem: HelperHeadersProblem) -> &'static str {
    match problem {
        HelperHeadersProblem::NotAnObject => {
            "The headersHelper command did not print a JSON object"
        }
        HelperHeadersProblem::NonStringValue => {
            "The headersHelper command printed a header whose value is not a string"
        }
    }
}

/// The members of a `HelperRun` the run reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperRunGiven {
    /// The command.
    pub command: String,
    /// That directory.
    pub cwd: String,
    /// Its environment.
    pub environment: IndexMap<String, String>,
    /// How long it may take.
    pub timeout_ms: f64,
}

/// `{ signal }`.
#[derive(Clone, Debug, Deserialize)]
pub struct RunSignal {
    /// Ends the run.
    pub signal: Signal,
}

/// What one run of the helper gives: the headers to send, or why there are none.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HelperOutput {
    /// Why there are none (`'severity' in output`).
    Issue(PluginIssue),
    /// The headers to send.
    Headers {
        /// Name to value.
        headers: IndexMap<String, String>,
    },
}

/// Runs the planned command in a shell (`bash`; a host on Windows maps it to the shell it has, as
/// for a hook) from the planned directory and environment, under 10 seconds, and reads the JSON
/// object of string pairs it prints. Every call runs the command: nothing is cached. A command that
/// fails is a `PluginIssue`, never a throw; judgment (spec/judgments.md): the issue's server is the
/// `CLAUDE_CODE_MCP_SERVER_NAME` the helper got.
pub async fn run_mcp_headers_helper(
    plan: HelperRunGiven,
    runner: &dyn CommandRunner,
    RunSignal { signal }: RunSignal,
) -> HelperOutput {
    let server = plan
        .environment
        .get("CLAUDE_CODE_MCP_SERVER_NAME")
        .cloned()
        .unwrap_or_default();
    let result = runner
        .run(CommandRun {
            spec: CommandSpec::Shell {
                command: plan.command,
                shell: ShellKind::Bash,
            },
            cwd: plan.cwd,
            env: plan.environment,
            stdin: None,
            timeout_ms: Some(plan.timeout_ms),
            merge_stderr: None,
            signal,
        })
        .await;
    if let Some(failed) = failure(&result) {
        return HelperOutput::Issue(problem(&server, &failed));
    }
    match read_helper_headers(&result.stdout) {
        HelperHeadersRead::Problem { problem: read } => {
            HelperOutput::Issue(problem(&server, output_problem(read)))
        }
        HelperHeadersRead::Headers { headers } => HelperOutput::Headers { headers },
    }
}

/// The headers to connect with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionHeaders {
    /// The headers.
    pub headers: IndexMap<String, String>,
    /// Whether OAuth is still the fallback: "If you configure an `Authorization` header, Claude
    /// Code uses it as the server's credential and doesn't fall back to OAuth", in `headers` or
    /// from the helper.
    pub oauth_fallback: bool,
}

/// "Dynamic headers override any static `headers` with the same name": names compare without
/// regard to case (judgment, spec/judgments.md).
#[must_use]
pub fn connection_headers(
    static_headers: &IndexMap<String, String>,
    dynamic_headers: &IndexMap<String, String>,
) -> ConnectionHeaders {
    let overridden: Vec<String> = dynamic_headers
        .keys()
        .map(|name| name.to_lowercase())
        .collect();
    let mut headers: IndexMap<String, String> = static_headers
        .iter()
        .filter(|(name, _)| !overridden.contains(&name.to_lowercase()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    headers.extend(
        dynamic_headers
            .iter()
            .map(|(name, value)| (name.clone(), value.clone())),
    );
    let headers = in_key_order(headers);
    let oauth_fallback = !has_authorization_header(&headers);
    ConnectionHeaders {
        headers,
        oauth_fallback,
    }
}

/// "A `401 Unauthorized` or `403 Forbidden`".
const REJECTED: [f64; 2] = [401.0, 403.0];

/// What `send` answered: the host's response, kept by the host when it holds a function.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Response {
    /// A response that holds a function: read through the host.
    Host(Port),
    /// A plain record.
    Data(JsValue),
}

impl Response {
    async fn status(&self) -> JsValue {
        match self {
            Self::Host(port) => port.member::<JsValue>("status").await,
            Self::Data(value) => value.get("status").cloned().unwrap_or_default(),
        }
    }

    async fn rejected(&self) -> Option<f64> {
        self.status()
            .await
            .as_f64()
            .filter(|status| REJECTED.contains(status))
    }
}

/// `ok`, `needs-authentication` or `failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// The call or connection went through.
    Ok,
    /// The retry after a rejected call was rejected too.
    NeedsAuthentication,
    /// The helper failed, or the server rejected the helper's credential while connecting.
    Failed,
}

/// How an attempt ended. `needs-authentication`: the retry after a rejected call was rejected too.
/// `failed`: the helper failed, or the server rejected the helper's credential while connecting;
/// the issue then carries the errors.md message when the helper supplied `Authorization`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct HelperAttempt {
    /// How it ended.
    pub outcome: Outcome,
    /// The response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>,
    /// How many times the helper was run again after a rejection: at most one.
    pub reruns: u8,
    /// Why it failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<PluginIssue>,
}

/// errors.md, "Server rejected the Authorization header minted by the configured headersHelper":
/// the line Claude Code reports for a connection a server answered with 401 or 403 while the helper
/// supplied `Authorization`.
fn rejected_authorization(server: &str, status: f64) -> PluginIssue {
    problem(
        server,
        &format!(
            "Server rejected the Authorization header minted by the configured headersHelper (HTTP {}). Check that the helper command returns a valid credential for this MCP endpoint — OAuth fallback is disabled when the helper supplies Authorization.",
            number_to_string(status)
        ),
    )
}

fn rejected_while_connecting(
    helper_headers: &IndexMap<String, String>,
    response: Response,
    status: f64,
    server: &str,
) -> HelperAttempt {
    let issue =
        has_authorization_header(helper_headers).then(|| rejected_authorization(server, status));
    HelperAttempt {
        outcome: Outcome::Failed,
        response: Some(response),
        reruns: 0,
        issue,
    }
}

/// `connect` or `call`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// A connection.
    Connect,
    /// A call.
    Call,
}

fn headers_value(headers: &IndexMap<String, String>) -> JsValue {
    JsValue::Object(
        headers
            .iter()
            .map(|(name, value)| (name.clone(), JsValue::from(value.as_str())))
            .collect(),
    )
}

fn failed(reruns: u8, issue: PluginIssue) -> HelperAttempt {
    HelperAttempt {
        outcome: Outcome::Failed,
        response: None,
        reruns,
        issue: Some(issue),
    }
}

/// One call or one connection with a helper (mcp.md): `produce` runs the helper fresh and gives the
/// headers it printed (`runMcpHeadersHelper`'s output, or none where the plan skipped it), and
/// `send` makes the request with them, merging the server's static headers (`connectionHeaders`).
/// `server` is the name Claude Code shows, for the issue.
///
/// - `call`: on a 401 or 403 the helper runs again once, the request is retried once with the fresh
///   headers, and "Claude Code marks the server as needing authentication in `/mcp` only if that
///   retry also fails".
/// - `connect`: "If the server rejects the helper's credential while connecting, Claude Code
///   reports the connection as failed rather than marking the server as needing authentication";
///   no re-run (a reconnect from `/mcp` runs the helper again).
pub async fn attempt_with_headers_helper(
    phase: Phase,
    produce: Callback,
    send: Callback,
    server: &str,
) -> HelperAttempt {
    let first = match produce.call::<HelperOutput>(vec![]).await {
        HelperOutput::Issue(issue) => return failed(0, issue),
        HelperOutput::Headers { headers } => headers,
    };
    let response = send.call::<Response>(vec![headers_value(&first)]).await;
    let Some(status) = response.rejected().await else {
        return HelperAttempt {
            outcome: Outcome::Ok,
            response: Some(response),
            reruns: 0,
            issue: None,
        };
    };
    if phase == Phase::Connect {
        return rejected_while_connecting(&first, response, status, server);
    }
    let again = match produce.call::<HelperOutput>(vec![]).await {
        HelperOutput::Issue(issue) => return failed(1, issue),
        HelperOutput::Headers { headers } => headers,
    };
    let retried = send.call::<Response>(vec![headers_value(&again)]).await;
    let outcome = if retried.rejected().await.is_some() {
        Outcome::NeedsAuthentication
    } else {
        Outcome::Ok
    };
    HelperAttempt {
        outcome,
        response: Some(retried),
        reruns: 1,
        issue: None,
    }
}

crate::registry::exports! {
    "loading/resolve/headers-helper-run" {
        "runMcpHeadersHelper" => async ok |plan: HelperRunGiven, runner: Port, options: RunSignal|
            run_mcp_headers_helper(plan, &runner, options).await;
        "connectionHeaders" => ok |static_headers: IndexMap<String, String>, dynamic_headers: IndexMap<String, String>|
            connection_headers(&static_headers, &dynamic_headers);
        "attemptWithHeadersHelper" => async ok |phase: Phase, produce: Callback, send: Callback, server: String|
            attempt_with_headers_helper(phase, produce, send, &server).await;
    }
}
