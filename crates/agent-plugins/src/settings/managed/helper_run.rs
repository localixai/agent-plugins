//! Port of `src/settings/managed/helper-run.ts`, switched.
//!
//! One run of a policy helper and what it comes to (settings-reference.md, `policyHelper`, "Write
//! the helper output", "Helper failures"). The process is the shared `CommandRunner`; the file check
//! is this module's `HelperFileProbe`; time is the `Clock`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::value::to_js_value;
use crate::js::{JsValue, number_to_string, trim};
use crate::ports::clock::Clock;
use crate::ports::command_runner::{CommandResult, CommandRun, CommandRunner, CommandSpec};
use crate::ports::signal::Signal;
use crate::registry::Port;
use crate::settings::managed::policy_helper::{
    HelperOutput, PathFlavor, PolicyHelper, helper_path_fault, read_helper_output,
};

/// When a helper runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HelperPhase {
    /// At launch.
    Startup,
    /// When a settings fetch reports the server-managed settings removed.
    FetchRemoved,
    /// A background refresh.
    Refresh,
}

/// The phases.
pub const HELPER_PHASES: [HelperPhase; 3] = [
    HelperPhase::Startup,
    HelperPhase::FetchRemoved,
    HelperPhase::Refresh,
];

/// What a failed run costs in each phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseOutcome {
    /// "When the startup run fails, Claude Code prints the reason and refuses to start"; a run that
    /// starts when "a later settings fetch reports the server-managed settings removed" "ends the
    /// session with the same message as a failed startup run".
    pub stops_claude_code: bool,
    /// "When a background refresh fails, Claude Code keeps the last successful policy in effect."
    pub keeps_last_policy: bool,
    /// "`/status` shows the failing refresh with its reason until a refresh succeeds."
    pub shown_in_status: bool,
}

/// What a failed run costs in `phase`.
#[must_use]
pub const fn phase_outcome(phase: HelperPhase) -> PhaseOutcome {
    let (stops_claude_code, keeps_last_policy, shown_in_status) = match phase {
        HelperPhase::Startup | HelperPhase::FetchRemoved => (true, false, false),
        HelperPhase::Refresh => (false, true, true),
    };
    PhaseOutcome {
        stops_claude_code,
        keeps_last_policy,
        shown_in_status,
    }
}

/// What a run came to.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HelperRun {
    /// It succeeded.
    Ok {
        /// `true`.
        ok: bool,
        /// The emitted `managedSettings`; `undefined` when the envelope omits it (the helper contributes nothing).
        settings: JsValue,
        /// What the helper wrote to stderr: "With `--debug`, Claude Code writes the helper's stderr from every run to the debug log."
        stderr: String,
    },
    /// It failed.
    Failed {
        /// `false`.
        ok: bool,
        /// Why.
        reason: String,
        /// What the failure costs.
        outcome: PhaseOutcome,
        /// What the helper wrote to stderr.
        stderr: String,
    },
}

fn failure(phase: HelperPhase, detail: &str, stderr: &str) -> HelperRun {
    HelperRun::Failed {
        ok: false,
        reason: format!("policyHelper failed: {detail}"),
        outcome: phase_outcome(phase),
        stderr: stderr.to_owned(),
    }
}

/// What stderr said, else stdout: "the reason includes the helper's stderr, or its stdout when stderr is empty".
fn output_of(result: &CommandResult) -> &str {
    if trim(&result.stderr).is_empty() {
        trim(&result.stdout)
    } else {
        trim(&result.stderr)
    }
}

/// The reason a finished process failed, or `None` when it exited `0` in time.
fn process_fault(result: &CommandResult, helper: &PolicyHelper) -> Option<String> {
    if let Some(start_error) = &result.start_error {
        return Some(format!("the helper did not start ({start_error})"));
    }
    // "After a timeout, the reason names the `timeoutMs` limit and includes none of the helper's output."
    if result.timed_out {
        return Some(format!(
            "the helper was still running after timeoutMs ({} ms)",
            number_to_string(helper.timeout_ms)
        ));
    }
    if result.exit_code == Some(0) {
        return None;
    }
    let code = result
        .exit_code
        .map_or_else(|| "none".to_owned(), |code| code.to_string());
    Some(format!(
        "the helper exited with code {code}: {}",
        output_of(result)
    ))
}

/// What a host hands to run a helper.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperHost {
    /// The shared command runner.
    pub runner: Port,
    /// The `HelperFileProbe`: `check(path, budgetMs, signal)`.
    pub probe: Port,
    /// The clock.
    pub clock: Port,
    /// The environment the helper inherits; `CLAUDE_CODE_VERSION` is added.
    pub env: IndexMap<String, String>,
    /// The folder it runs in.
    pub cwd: String,
    /// The Claude Code version the helper is told in `CLAUDE_CODE_VERSION`.
    pub version: String,
}

/// The probe's answer.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Probed {
    regular_file: bool,
    timed_out: bool,
}

/// One run.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HelperRunInput {
    /// The helper.
    pub helper: PolicyHelper,
    /// The host's ports and facts.
    pub host: HelperHost,
    /// The OS family the path is judged for.
    pub flavor: PathFlavor,
    /// When it runs.
    pub phase: HelperPhase,
    /// Ends the run.
    pub signal: Signal,
}

/// Runs the helper with no arguments and `CLAUDE_CODE_VERSION` set, within `timeoutMs` counted from
/// the file check (the check and the run share one budget).
pub async fn run_policy_helper(input: HelperRunInput) -> HelperRun {
    let HelperRunInput {
        helper,
        host,
        phase,
        signal,
        flavor,
    } = input;
    if let Some(path_fault) = helper_path_fault(&helper.path, flavor) {
        return failure(phase, &format!("the path {path_fault}"), "");
    }
    let timeout = number_to_string(helper.timeout_ms);
    let started = host.clock.now().await;
    let args = vec![
        JsValue::from(helper.path.as_str()),
        JsValue::from(helper.timeout_ms),
        to_js_value(&signal).unwrap_or_default(),
    ];
    let probed: Probed = host.probe.method("check", args).await;
    if probed.timed_out {
        let detail = format!("checking the helper file took longer than timeoutMs ({timeout} ms)");
        return failure(phase, &detail, "");
    }
    if !probed.regular_file {
        return failure(phase, &format!("no regular file is at {}", helper.path), "");
    }
    let remaining = helper.timeout_ms - (host.clock.now().await - started);
    if remaining <= 0.0 {
        let detail = format!("the file check used up timeoutMs ({timeout} ms)");
        return failure(phase, &detail, "");
    }
    let mut env = host.env.clone();
    env.insert("CLAUDE_CODE_VERSION".to_owned(), host.version.clone());
    let run = CommandRun {
        spec: CommandSpec::Exec {
            command: helper.path.clone(),
            args: Vec::new(),
        },
        cwd: host.cwd.clone(),
        env,
        stdin: None,
        timeout_ms: Some(remaining),
        merge_stderr: None,
        signal,
    };
    let result = host.runner.run(run).await;
    if let Some(fault) = process_fault(&result, &helper) {
        return failure(phase, &fault, &result.stderr);
    }
    match read_helper_output(&result.stdout, &result.stderr) {
        HelperOutput::Failure { failure: why } => failure(phase, &why, &result.stderr),
        HelperOutput::Settings { settings } => HelperRun::Ok {
            ok: true,
            settings,
            stderr: result.stderr,
        },
    }
}

crate::registry::exports! {
    "settings/managed/helper-run" {
        "HELPER_PHASES" => ok || HELPER_PHASES;
        "PHASE_OUTCOMES" => ok || HELPER_PHASES
            .into_iter()
            .map(|phase| (phase, phase_outcome(phase)))
            .collect::<IndexMap<HelperPhase, PhaseOutcome>>();
        "runPolicyHelper" => async ok |input: HelperRunInput| run_policy_helper(input).await;
    }
}
