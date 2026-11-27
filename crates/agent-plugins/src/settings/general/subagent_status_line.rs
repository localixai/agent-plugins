//! Port of `src/settings/general/subagent-status-line.ts` at sha256:bd48170f4297.
//!
//! Running a `subagentStatusLine` (statusline.md, "Subagent status lines"): what the command reads
//! on stdin and how its lines rewrite the subagent rows. Which line runs, under which gates, is
//! `readSubagentStatusLine`; when a refresh tick fires, the environment the command gets and how a
//! row body is drawn stay the host's. statusline.md is outside the requirement set, so each rule
//! here is a judgment (spec/judgments.md).
//!
//! A task (one entry of `tasks`, statusline.md, "Task fields") and the base hook input are
//! [`JsValue`] objects, kept whole as the spreads of the package keep them: `id`, `name`, `type`
//! (`local_agent`), `agentType`, `status`, `description`, `label`, `startTime`, `model`, `effort`,
//! `contextWindowSize`, `tokenCount`, `tokenSamples` ("up to the last 16 `tokenCount` readings, one
//! per refresh tick, oldest first") and `cwd`. `runSubagentStatusLine` takes the host's
//! `CommandRunner` and stays TypeScript in the package (the module comment of `crate::settings`),
//! so only the two data functions are exported.

use indexmap::IndexMap;

use crate::js::json_parse::json_parse;
use crate::js::{JsObject, JsValue, json_stringify, trim};
use crate::ports::command_runner::{CommandRun, CommandRunner, CommandSpec, ShellKind};
use crate::ports::{BoxFuture, Signal};

const MAX_TOKEN_SAMPLES: usize = 16;

/// The stdin object of one tick, each row's `tokenSamples` cut to its last 16 readings.
#[must_use]
pub fn subagent_status_line_input(base: &JsObject, columns: f64, tasks: &[JsValue]) -> JsValue {
    let mut input = base.clone();
    input.insert("columns".to_owned(), JsValue::Number(columns));
    let cut: Vec<JsValue> = tasks.iter().map(with_last_samples).collect();
    input.insert("tasks".to_owned(), JsValue::Array(cut));
    JsValue::Object(input)
}

/// `{ ...task, tokenSamples: task.tokenSamples.slice(-16) }`.
fn with_last_samples(task: &JsValue) -> JsValue {
    let mut copy = task.as_object().cloned().unwrap_or_default();
    let samples = task
        .get("tokenSamples")
        .and_then(JsValue::as_array)
        .cloned()
        .unwrap_or_default();
    let start = samples.len().saturating_sub(MAX_TOKEN_SAMPLES);
    copy.insert(
        "tokenSamples".to_owned(),
        JsValue::Array(samples.get(start..).unwrap_or_default().to_vec()),
    );
    JsValue::Object(copy)
}

/// One printed line as a row, or `None` for a line that is not `{"id","content"}` JSON.
fn row_of(line: &str) -> Option<(String, String)> {
    let parsed = json_parse(line).ok()?;
    let id = parsed.get("id")?.as_str()?;
    let content = parsed.get("content")?.as_str()?;
    Some((id.to_owned(), content.to_owned()))
}

/// The rows the command rewrote, by task id: each body exactly as printed (ANSI and OSC 8 kept),
/// an empty one meaning the row is hidden. A row it does not name keeps the default and is absent.
/// Judgment: a line that is not such a JSON object, or names no visible task, is skipped, and a
/// later line for the same id wins.
#[must_use]
pub fn read_subagent_status_rows(stdout: &str, tasks: &[JsValue]) -> IndexMap<String, String> {
    let visible: Vec<&str> = tasks
        .iter()
        .filter_map(|task| task.get("id").and_then(JsValue::as_str))
        .collect();
    let mut rows = IndexMap::new();
    for line in stdout.split('\n') {
        if let Some((id, content)) = row_of(trim(line))
            && visible.contains(&id.as_str())
        {
            rows.insert(id, content);
        }
    }
    rows
}

/// What one run needs beyond the line: the stdin object and the whole environment of the process.
#[derive(Clone, Debug)]
pub struct SubagentStatusLineRun {
    /// The stdin object (`subagent_status_line_input`'s answer).
    pub input: JsValue,
    /// The whole environment of the process.
    pub env: IndexMap<String, String>,
}

/// Runs the line once over `run.input` through the `CommandRunner` and reads its rows. Judgment:
/// the command is a `bash` line (a host on Windows maps it to Git Bash or PowerShell, statusline.md
/// "Windows configuration"), run from `input.cwd` with no time limit of its own (the host cancels a
/// run a new tick replaces through `signal`); like the main status line ("displays your script's
/// output only after the script exits with code 0"), a run that does not exit 0 rewrites no row.
pub fn run_subagent_status_line<'a>(
    command: &'a str,
    run: &'a SubagentStatusLineRun,
    runner: &'a dyn CommandRunner,
    signal: Signal,
) -> BoxFuture<'a, IndexMap<String, String>> {
    Box::pin(async move {
        let cwd = run
            .input
            .get("cwd")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned();
        let result = runner
            .run(CommandRun {
                spec: CommandSpec::Shell {
                    command: command.to_owned(),
                    shell: ShellKind::Bash,
                },
                cwd,
                env: run.env.clone(),
                stdin: json_stringify(&run.input),
                timeout_ms: None,
                merge_stderr: false,
                signal,
            })
            .await;
        if result.exit_code == Some(0) {
            let tasks = run
                .input
                .get("tasks")
                .and_then(JsValue::as_array)
                .cloned()
                .unwrap_or_default();
            read_subagent_status_rows(&result.stdout, &tasks)
        } else {
            IndexMap::new()
        }
    })
}

crate::registry::exports! {
    "settings/general/subagent-status-line" {
        "subagentStatusLineInput" => ok |base: JsObject, columns: f64, tasks: Vec<JsValue>| subagent_status_line_input(&base, columns, &tasks);
        "readSubagentStatusRows" => ok |stdout: String, tasks: Vec<JsValue>| crate::registry::Map { entries: read_subagent_status_rows(&stdout, &tasks).into_iter().collect() };
    }
}
