//! Port of `src/rules/tools/semantics/bash-background.ts`, switched.
//!
//! A foreground command that reaches its timeout, moved to the background, and the notices of a
//! background command (tools-reference.md, "Foreground commands that move to the background").

use serde::Deserialize;

use crate::format::env_flag::is_truthy_env;
use crate::js::{number_to_string, trim_start};
use crate::ports::environment::Environment;
use crate::registry::Port;
use crate::rules::bash::split::split_command;
use crate::rules::bash::words::words_of;

/// What the session is, as far as moving a timed-out command goes.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct MoveSession {
    /// Running in bare mode (headless.md, "Start faster with bare mode").
    pub bare: bool,
}

/// Whether a foreground command that reaches its timeout moves to the background instead of
/// stopping (tools-reference.md, "Foreground commands that move to the background"): it does,
/// unless the command starts with `sleep`, `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS=1` is set
/// (env-vars.md: "Set to `1`") or the session runs in bare mode. A line "starts with `sleep`" when
/// its first word is `sleep` (spec/judgments.md).
pub async fn should_move_to_background(
    command: &str,
    env: &dyn Environment,
    session: MoveSession,
) -> bool {
    if session.bare {
        return false;
    }
    let disabled = env.get("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS").await;
    if is_truthy_env(disabled.as_deref()) {
        return false;
    }
    words_of(trim_start(command), None)
        .first()
        .is_none_or(|word| word.text != "sleep")
}

/// "`Command did not complete within its 120s timeout and was moved to the background`", seconds
/// of the timeout.
#[must_use]
pub fn moved_by_timeout_notice(timeout_ms: f64) -> String {
    format!(
        "Command did not complete within its {}s timeout and was moved to the background",
        number_to_string(timeout_ms / 1000.0)
    )
}

const DIRECTORY_COMMANDS: [&str; 4] = ["cd", "pushd", "popd", "chdir"];

/// The note of a moved command that changes directory: "A `cd`, `pushd`, `popd`, or `chdir`
/// inside a command that is moved to the background never carries over: the result states
/// `Session cwd remains <dir>; directory changes made by the backgrounded command do not apply to
/// subsequent commands.`" `None` when no subcommand starts with one of the four.
#[must_use]
pub fn background_cwd_note(command: &str, cwd: &str) -> Option<String> {
    let changes = split_command(command, None)
        .subcommands
        .iter()
        .any(|subcommand| {
            words_of(subcommand, None)
                .first()
                .is_some_and(|word| DIRECTORY_COMMANDS.contains(&word.text.as_str()))
        });
    changes.then(|| {
        format!(
            "Session cwd remains {cwd}; directory changes made by the backgrounded command do not apply to subsequent commands."
        )
    })
}

/// "`Background command "<description>" was stopped after reaching its background time limit`".
#[must_use]
pub fn background_stop_notice(description: &str) -> String {
    format!(
        "Background command \"{description}\" was stopped after reaching its background time limit"
    )
}

crate::registry::exports! {
    "rules/tools/semantics/bash-background" {
        "shouldMoveToBackground" => async ok |command: String, env: Port, session: MoveSession| should_move_to_background(&command, &env, session).await;
        "movedByTimeoutNotice" => ok |timeout_ms: f64| moved_by_timeout_notice(timeout_ms);
        "backgroundCwdNote" => ok |command: String, cwd: String| background_cwd_note(&command, &cwd);
        "backgroundStopNotice" => ok |description: String| background_stop_notice(&description);
    }
}
