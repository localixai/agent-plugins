//! The core's counterpart of `src/hooks/verdict/exit2.ts` at sha256:69bfa3e2f1eb, which stays TypeScript:
//! its table holds functions (`missingFileIsError`, `cannotBlockWhen`), which no value crossing the
//! boundary can carry (docs/RUST.md, "What stays TypeScript").
//!
//! What exit code 2 does on each event (hooks.md, "Exit code 2 behavior per event"): the table is
//! data, one row per event of the official union.

use crate::hooks::contracts::HookSource;
use crate::hooks::input::fact_read::fact_of;
use crate::hooks::source_kind::is_plugin_source;
use crate::js::JsValue;

/// Where the stderr of an exit-2 hook goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StderrChannel {
    /// Claude sees it: the denial reason, the tool's feedback, the next instruction.
    Claude,
    /// The user sees it: a block message, a `<hook name> hook error` notice.
    User,
    /// Both.
    UserAndClaude,
    /// Only the debug log ("Stderr goes to the debug log"; "A blocked change surfaces no message").
    Debug,
    /// "Exit code and stderr are ignored", "its stderr is discarded".
    Discarded,
}

/// One row of the table (`Exit2Rule`). `effect` is its `Exit2Effect` text.
#[derive(Clone, Copy, Debug)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one row of the package's table: independent yes-or-no facts of an event"
)]
pub struct Exit2Rule {
    /// "Can block?" of the table: exit 2 stops what the event guards.
    pub can_block: bool,
    /// What exit 2 does to the thing the event guards.
    pub effect: &'static str,
    /// Where the stderr goes.
    pub stderr: StderrChannel,
    /// `WorktreeCreate` and `WorktreeRemove`: "Any non-zero exit code" fails, not only 2.
    pub any_non_zero_blocks: bool,
    /// `onFailure` has no effect: exit 2 "sends Claude back to keep working".
    pub ignores_on_failure: bool,
    /// `PreModelSwitch`: a hook that doesn't respond before its timeout blocks the switch.
    pub timeout_blocks: bool,
    /// `PermissionRequest`: a failure with `onFailure: "block"` denies, though exit 2 is not honored.
    pub failure_denies: bool,
    /// The `hookSpecificOutput` applies only when the hook exits 0.
    pub output_only_on_exit0: bool,
    /// `WorktreeRemove`: the failure counts only if the directory is still there afterwards.
    pub blocks_unless_directory_gone: bool,
    /// Exit 2 with nothing on stdout and a missing-file stderr is a non-blocking error for these sources.
    pub missing_file_is_error: Option<fn(&HookSource) -> bool>,
    /// A hook that ends the turn stops a teammate instead of holding an action back.
    pub stops_teammate: bool,
    /// The event's own exception to blocking (`ConfigChange`: "`policy_settings` changes can't be blocked").
    pub cannot_block_when: Option<fn(&JsValue) -> bool>,
}

const fn row(can_block: bool, effect: &'static str, stderr: StderrChannel) -> Exit2Rule {
    Exit2Rule {
        can_block,
        effect,
        stderr,
        any_non_zero_blocks: false,
        ignores_on_failure: false,
        timeout_blocks: false,
        failure_denies: false,
        output_only_on_exit0: false,
        blocks_unless_directory_gone: false,
        missing_file_is_error: None,
        stops_teammate: false,
        cannot_block_when: None,
    }
}

const fn blocks(effect: &'static str, stderr: StderrChannel) -> Exit2Rule {
    row(true, effect, stderr)
}

const fn ignored(stderr: StderrChannel) -> Exit2Rule {
    row(false, "none", stderr)
}

/// "Stop, SubagentStop, TaskCompleted, and a plugin's UserPromptSubmit hook": a missing file is not a block.
const fn any_source(_: &HookSource) -> bool {
    true
}

const POLICY_SETTINGS: &str = "policy_settings";

fn is_policy_settings(facts: &JsValue) -> bool {
    fact_of(facts, "source").as_str() == Some(POLICY_SETTINGS)
}

/// The `onFailure`-exempt events: exit 2 there sends Claude back to work.
const fn keeps_working(rule: Exit2Rule) -> Exit2Rule {
    Exit2Rule {
        ignores_on_failure: true,
        missing_file_is_error: Some(any_source),
        ..rule
    }
}

/// The row of `event` (`EXIT2_RULES[event]`); `None` for a name the official union does not hold.
#[must_use]
pub fn exit2_rule(event: &str) -> Option<Exit2Rule> {
    use StderrChannel::{Claude, Debug, Discarded, User, UserAndClaude};
    Some(match event {
        "PreToolUse" => blocks("blocks-tool-call", Claude),
        "PermissionRequest" => Exit2Rule {
            failure_denies: true,
            ..row(false, "none", Discarded)
        },
        "UserPromptSubmit" => Exit2Rule {
            missing_file_is_error: Some(is_plugin_source),
            ..blocks("blocks-prompt", User)
        },
        "UserPromptExpansion" => blocks("blocks-expansion", User),
        "Stop" => keeps_working(blocks("prevents-stop", Claude)),
        "SubagentStop" => keeps_working(blocks("prevents-subagent-stop", Claude)),
        "TeammateIdle" => Exit2Rule {
            ignores_on_failure: true,
            stops_teammate: true,
            ..blocks("prevents-teammate-idle", Claude)
        },
        "TaskCreated" => blocks("rolls-back-task-creation", Claude),
        "TaskCompleted" => Exit2Rule {
            stops_teammate: true,
            ..keeps_working(blocks("prevents-task-completion", Claude))
        },
        "ConfigChange" => Exit2Rule {
            cannot_block_when: Some(is_policy_settings),
            ..blocks("blocks-config-change", Debug)
        },
        "StopFailure" => ignored(Discarded),
        "PostToolUse" => ignored(Claude),
        "PostToolUseFailure" => ignored(Claude),
        "PostToolBatch" => blocks("stops-agentic-loop", UserAndClaude),
        "PermissionDenied" => ignored(Discarded),
        "Notification" => ignored(Discarded),
        "SubagentStart" => ignored(User),
        "SessionStart" => ignored(User),
        "Setup" => ignored(Discarded),
        "SessionEnd" => ignored(User),
        "CwdChanged" => ignored(User),
        "DirectoryAdded" => ignored(Debug),
        "FileChanged" => ignored(User),
        "PreCompact" => blocks("blocks-compaction", User),
        "PostCompact" => ignored(User),
        "PreModelSwitch" => Exit2Rule {
            timeout_blocks: true,
            ..blocks("blocks-model-switch", User)
        },
        "PostModelSwitch" => ignored(User),
        "Elicitation" => Exit2Rule {
            output_only_on_exit0: true,
            ..blocks("declines-elicitation", Discarded)
        },
        "ElicitationResult" => Exit2Rule {
            output_only_on_exit0: true,
            ..blocks("blocks-elicitation-response", Discarded)
        },
        // Judgment (spec/judgments.md): the docs give the exit code of WorktreeCreate no stderr route.
        "WorktreeCreate" => Exit2Rule {
            any_non_zero_blocks: true,
            ..blocks("fails-worktree-creation", Debug)
        },
        "WorktreeRemove" => Exit2Rule {
            any_non_zero_blocks: true,
            blocks_unless_directory_gone: true,
            ..blocks("fails-worktree-removal", Debug)
        },
        "InstructionsLoaded" => ignored(Discarded),
        // Judgment (spec/judgments.md): "The original text is displayed" says nothing of stderr.
        "MessageDisplay" => ignored(Discarded),
        _ => return None,
    })
}
