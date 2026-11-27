//! Port of `src/hooks/interrupt.ts`, switched.
//!
//! What a user's interrupt does to the hooks of an event (hooks.md, "PostToolUse after an
//! interrupt" and "`Esc` during a prompt check").
//!
//! - `PostToolUse`: `Ctrl+Enter` that ends the turn while a tool runs gives the hooks "10 seconds to
//!   finish" and kills a command hook still running then, "whatever its `timeout`"; `Enter` only
//!   queues a message, so "the hooks keep their usual `timeout`"; `Esc` cancels the hooks at once.
//!   `Ctrl+Enter` "acts as `Esc` if you queued a `!` command ahead of your messages or background
//!   tasks are disabled, for example with `CLAUDE_CODE_DISABLE_BACKGROUND_TASKS`".
//! - `UserPromptSubmit`: an interrupt while the hooks check a prompt stops the check, the prompt
//!   does not reach Claude, and the text goes back to the input, unless a hook blocked the prompt
//!   at the same moment: then it "stays blocked" and the text does not come back.
//!
//! Judgments (spec/judgments.md): the first rule is read for `PostToolUse` only, since no other
//! event's section says so; the second is read from the request's abort signal (an interrupted
//! session aborts it) together with a hook that was cancelled by it.
//!
//! An outcome is read as a [`JsValue`]: only `event`, `runs[].status` and `block` are read, and
//! the rest, its `background` promises of the host's among it (`{"$promise": n}`), is given back
//! as it came.

use serde::{Deserialize, Serialize};

use crate::format::env_flag::is_truthy_env;
use crate::js::JsValue;
use crate::ports::environment::Environment;

/// The keys an interrupt is pressed with.
pub const INTERRUPT_KEYS: [&str; 3] = ["esc", "enter", "ctrl-enter"];

/// A key the user pressed (`INTERRUPT_KEYS`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InterruptKey {
    /// `Esc`.
    Esc,
    /// `Enter`.
    Enter,
    /// `Ctrl+Enter`.
    CtrlEnter,
}

/// The key the user pressed while a tool ran, and what the host knows of what they queued.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserInterrupt {
    /// The key.
    pub key: InterruptKey,
    /// A `!` shell command was queued ahead of the messages.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub queued_shell_command: Option<bool>,
}

/// What the interrupt does to the hooks that have not run.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterruptPlan {
    /// The hooks are cancelled at once, none of them runs.
    pub cancel: bool,
    /// The most a hook may run, in milliseconds, whatever its `timeout`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cap_ms: Option<f64>,
}

const CTRL_ENTER_BUDGET_MS: f64 = 10_000.0;
const KEEP: InterruptPlan = InterruptPlan {
    cancel: false,
    cap_ms: None,
};
const CANCEL: InterruptPlan = InterruptPlan {
    cancel: true,
    cap_ms: None,
};

async fn background_disabled(environment: &(impl Environment + ?Sized)) -> bool {
    is_truthy_env(
        environment
            .get("CLAUDE_CODE_DISABLE_BACKGROUND_TASKS")
            .await
            .as_deref(),
    )
}

/// `PLANS`: each key's plan.
async fn plans(
    interrupt: &UserInterrupt,
    environment: &(impl Environment + ?Sized),
) -> InterruptPlan {
    match interrupt.key {
        InterruptKey::Esc => CANCEL,
        InterruptKey::Enter => KEEP,
        InterruptKey::CtrlEnter => {
            if interrupt.queued_shell_command == Some(true)
                || background_disabled(environment).await
            {
                CANCEL
            } else {
                InterruptPlan {
                    cancel: false,
                    cap_ms: Some(CTRL_ENTER_BUDGET_MS),
                }
            }
        }
    }
}

/// The events an interrupt during a tool call changes the hooks of.
const AFTER_TOOL: [&str; 1] = ["PostToolUse"];

/// What the interrupt does to the hooks of `event` that have not run.
pub async fn interrupt_plan(
    event: &str,
    interrupt: Option<&UserInterrupt>,
    environment: &(impl Environment + ?Sized),
) -> InterruptPlan {
    match interrupt {
        Some(interrupt) if AFTER_TOOL.contains(&event) => plans(interrupt, environment).await,
        _ => KEEP,
    }
}

/// The events whose prompt check an interrupt stops.
const PROMPT_CHECKS: [&str; 1] = ["UserPromptSubmit"];

/// An interrupt that cancelled a prompt check: the prompt does not go ahead, and its text returns
/// to the input unless a hook blocked it, in which case the block message is what the user sees.
#[must_use]
pub fn with_interrupt(outcome: JsValue, aborted: bool) -> JsValue {
    let cancelled = outcome
        .get("runs")
        .and_then(JsValue::as_array)
        .is_some_and(|runs| {
            runs.iter()
                .any(|run| run.get("status").and_then(JsValue::as_str) == Some("cancelled"))
        });
    let event = outcome
        .get("event")
        .and_then(JsValue::as_str)
        .unwrap_or_default();
    if !aborted || !cancelled || !PROMPT_CHECKS.contains(&event) {
        return outcome;
    }
    let restore_input = outcome.get("block").is_none_or(JsValue::is_undefined);
    let JsValue::Object(mut members) = outcome else {
        return outcome;
    };
    members.insert("proceed".to_owned(), JsValue::Bool(false));
    members.insert(
        "interrupted".to_owned(),
        JsValue::from([("restoreInput", JsValue::Bool(restore_input))]),
    );
    JsValue::Object(members)
}

crate::registry::exports! {
    "hooks/interrupt" {
        "INTERRUPT_KEYS" => ok || INTERRUPT_KEYS;
    }
}

#[cfg(test)]
mod tests {
    use super::with_interrupt;
    use crate::js::json_parse::json_parse;

    #[test]
    fn a_cancelled_prompt_check_stops_and_gives_the_hosts_promises_back_as_they_came() {
        let outcome = json_parse(
            r#"{"event": "UserPromptSubmit", "proceed": true, "runs": [{"status": "cancelled"}],
            "background": [{"$promise": 3}]}"#,
        )
        .unwrap();
        let stopped = json_parse(
            r#"{"event": "UserPromptSubmit", "proceed": false, "runs": [{"status": "cancelled"}],
            "background": [{"$promise": 3}], "interrupted": {"restoreInput": true}}"#,
        )
        .unwrap();
        assert_eq!(with_interrupt(outcome.clone(), true), stopped);
        assert_eq!(with_interrupt(outcome.clone(), false), outcome);
    }
}
