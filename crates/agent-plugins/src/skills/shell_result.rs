//! Ported from `src/skills/shell-result.ts`; only the core uses it now, so it has no wrapper.
//!
//! What one finished injected command becomes (skills.md, "How injected commands run": "Either tool
//! runs the commands the same way it runs Claude's own shell commands. They share the working
//! directory, timeout, and output handling"; "When an injected command fails"). Every rule of the
//! Bash tool is the rules module's (`tools/semantics`); this file only applies them to a result.
//!
//! `SkillCommandResult` and `BackgroundedCommand` are of `src/skills/shell-ports.ts`, a module of
//! types only: they are declared here, where the TypeScript first needs them.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::utf8::prefix_within_bytes;
use crate::js::{JsValue, code_unit_length, number_to_string, trim_end};
use crate::ports::command_runner::ShellKind;
use crate::ports::environment::Environment;
use crate::ports::scratch_files::ScratchFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::catch_host;
use crate::rules::tools::semantics::bash_background::{
    MoveSession, background_cwd_note, moved_by_timeout_notice, should_move_to_background,
};
use crate::rules::tools::semantics::bash_exit::{classify_bash_exit, classify_power_shell_exit};
use crate::rules::tools::semantics::bash_limits::effective_bash_timeout;
use crate::rules::tools::semantics::bash_output::{bash_output_limits, window_bash_output};

/// What the result rules read of the invocation.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultContext {
    /// The skill's shell.
    pub shell: ShellKind,
    /// The command's environment.
    pub env: IndexMap<String, String>,
    /// The working directory.
    pub cwd: String,
    /// The `bashOutputMaxChars` setting.
    #[serde(default)]
    pub bash_output_max_chars: Option<f64>,
    /// Bare mode turns auto-backgrounding off ("`--bare` ... disables auto-backgrounding").
    pub bare: bool,
}

/// A result to inject, or the failure that aborts the invocation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum Step {
    /// The text that takes the command's place.
    Output {
        /// The text.
        output: String,
    },
    /// Why the invocation aborts.
    Failure {
        /// The reason.
        failure: String,
    },
}

/// A foreground command the Bash tool moved to the background at its timeout, still running.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackgroundedCommand {
    /// The background task's ID, which the injected text names.
    #[serde(default)]
    pub task_id: Option<String>,
    /// The file the command's output is being written to, which the injected text names.
    #[serde(default)]
    pub output_path: Option<String>,
}

/// A `CommandResult` with the move to the background a skill's runner may report.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillCommandResult {
    /// The exit code; `None` when the process was stopped by a signal or never started.
    #[serde(default)]
    pub exit_code: Option<f64>,
    /// What it wrote to standard output.
    pub stdout: String,
    /// What it wrote to standard error.
    pub stderr: String,
    /// Whether the host stopped it at its timeout.
    pub timed_out: bool,
    /// Why it could not start, when it could not.
    #[serde(default)]
    pub start_error: Option<String>,
    /// Set when the run reached its timeout and the command was moved to the background.
    #[serde(default)]
    pub background: Option<BackgroundedCommand>,
}

/// The command a result is of: its placeholder and its text.
#[derive(Clone, Debug, Deserialize)]
pub struct Planned {
    /// The placeholder, for messages.
    pub pattern: String,
    /// The command with its paths written in.
    pub command: String,
}

/// `environmentOf(env)`: the command's environment as the `Environment` the rules read.
struct EnvironmentOf<'a>(&'a IndexMap<String, String>);

impl Environment for EnvironmentOf<'_> {
    fn get<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(std::future::ready(self.0.get(name).cloned()))
    }

    fn snapshot(&self) -> BoxFuture<'_, IndexMap<String, String>> {
        Box::pin(std::future::ready(self.0.clone()))
    }
}

/// Judgment (spec/judgments.md): a value that is not a positive whole number of milliseconds is
/// ignored.
pub async fn timeout_of(env: &IndexMap<String, String>) -> f64 {
    effective_bash_timeout(&JsValue::Undefined, &EnvironmentOf(env)).await
}

/// Whether the command may move to the background at its timeout instead of being killed.
pub async fn may_move_to_background(command: &str, context: &ResultContext) -> bool {
    let session = MoveSession { bare: context.bare };
    should_move_to_background(command, &EnvironmentOf(&context.env), session).await
}

/// Exit 0, and exit 1 of the commands the tool lists, are results: "any non-zero exit code counts
/// as a failure".
fn is_normal_result(command: &str, shell: ShellKind, exit_code: Option<f64>) -> bool {
    exit_code.is_some_and(|code| match shell {
        ShellKind::Bash => classify_bash_exit(command, code),
        ShellKind::Powershell => classify_power_shell_exit(command, code),
    })
}

const OUTPUT_FILE: &str = "skill-command-output.txt";

/// "the path of a file saved to the session directory and truncated past 64 MiB"
/// (tools-reference.md).
const SAVED_OUTPUT_MAX_BYTES: usize = 64 * 1024 * 1024;

/// A valid result past the inline ceiling "arrives as a file path plus a short preview, not
/// truncated text" (tools-reference.md, "Output limits"). Judgment (spec/judgments.md): the docs
/// give the content, not the wording; without a `ScratchFiles` port, or when it fails, the preview
/// says the rest was not saved.
async fn inline_or_file(
    output: &str,
    context: &ResultContext,
    scratch: Option<&dyn ScratchFiles>,
    signal: &Signal,
) -> String {
    let limits =
        bash_output_limits(&EnvironmentOf(&context.env), context.bash_output_max_chars).await;
    let window = window_bash_output(output, true, limits);
    if !window.saved_to_file {
        return window.text;
    }
    let saved = prefix_within_bytes(output, SAVED_OUTPUT_MAX_BYTES);
    let path = match scratch {
        Some(scratch) => catch_host(scratch.write(OUTPUT_FILE, saved, Some(signal)))
            .await
            .ok(),
        None => None,
    };
    let extent = if saved == output {
        "the full output"
    } else {
        "the first 64 MiB of the output"
    };
    let place = path.map_or_else(
        || "the rest could not be saved".to_owned(),
        |path| format!("{extent} was saved to {path}; read or search that file for the rest"),
    );
    format!(
        "[The command printed {} characters, more than fits inline: {place}. Preview of the first {} characters follows.]\n{}",
        code_unit_length(output),
        code_unit_length(&window.text),
        window.text
    )
}

/// A failure's output "Inline up to roughly 10,000 characters; past that, a head-and-tail
/// excerpt".
async fn failure_output(output: &str, context: &ResultContext) -> String {
    let limits =
        bash_output_limits(&EnvironmentOf(&context.env), context.bash_output_max_chars).await;
    window_bash_output(output, false, limits).text
}

fn failed(pattern: &str, reason: &str) -> String {
    if reason.is_empty() {
        format!("Shell command failed for pattern \"{pattern}\"")
    } else {
        format!("Shell command failed for pattern \"{pattern}\": {reason}")
    }
}

/// The text of a command the Bash tool moved to the background: "The injected text reports the
/// move and names the background task and the file collecting the command's output", and a `cd`
/// inside it "never carries over". Judgment (spec/judgments.md): the sentence around the task and
/// the file.
async fn moved_text(command: &str, result: &SkillCommandResult, context: &ResultContext) -> String {
    let background = result.background.clone().unwrap_or_default();
    let task_id = background.task_id.unwrap_or_default();
    let output_path = background.output_path.unwrap_or_default();
    let notice = format!(
        "{}. Background task {task_id}, output file {output_path}.",
        moved_by_timeout_notice(timeout_of(&context.env).await)
    );
    match background_cwd_note(command, &context.cwd) {
        None => notice,
        Some(cwd_note) => format!("{notice} {cwd_note}"),
    }
}

/// The output of a run that did not end in a result: standard output then error, trimmed.
async fn combined_output(result: &SkillCommandResult, context: &ResultContext) -> String {
    let combined = format!("{}{}", result.stdout, result.stderr);
    failure_output(trim_end(&combined), context).await
}

/// What a run that reached its timeout becomes: a rendered skill when it moved, else the failure.
async fn timed_out(
    planned: &Planned,
    result: &SkillCommandResult,
    context: &ResultContext,
    move_allowed: bool,
) -> Step {
    if move_allowed && result.background.is_some() {
        return Step::Output {
            output: moved_text(&planned.command, result, context).await,
        };
    }
    let output = combined_output(result, context).await;
    let timeout = timeout_of(&context.env).await;
    let minutes = timeout / 60_000.0;
    let limit = if minutes.is_finite() && minutes.fract() == 0.0 {
        let unit = if minutes == 1.0 { "minute" } else { "minutes" };
        format!("{} {unit}", number_to_string(minutes))
    } else {
        format!("{} seconds", number_to_string(timeout / 1000.0))
    };
    let tail = if output.is_empty() {
        String::new()
    } else {
        format!("\n{output}")
    };
    Step::Failure {
        failure: failed(&planned.pattern, &format!("timed out after {limit}{tail}")),
    }
}

/// One finished command: a result to inject, or the failure that aborts the invocation.
pub async fn render_result(
    planned: &Planned,
    result: &SkillCommandResult,
    context: &ResultContext,
    (scratch, signal): (Option<&dyn ScratchFiles>, &Signal),
) -> Step {
    if let Some(start_error) = &result.start_error {
        return Step::Failure {
            failure: failed(&planned.pattern, start_error),
        };
    }
    if result.timed_out {
        let move_allowed = may_move_to_background(&planned.command, context).await;
        return timed_out(planned, result, context, move_allowed).await;
    }
    if is_normal_result(&planned.command, context.shell, result.exit_code) {
        let output = inline_or_file(trim_end(&result.stdout), context, scratch, signal).await;
        return Step::Output { output };
    }
    let output = combined_output(result, context).await;
    let reason = if output.is_empty() {
        let code = result
            .exit_code
            .map_or_else(|| "none".to_owned(), number_to_string);
        format!("exit code {code}")
    } else {
        output
    };
    Step::Failure {
        failure: failed(&planned.pattern, &reason),
    }
}
