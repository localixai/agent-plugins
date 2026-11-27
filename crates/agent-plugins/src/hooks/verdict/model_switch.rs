//! Port of `src/hooks/verdict/model-switch.ts`, switched.
//!
//! The reasons a blocked model switch is reported with when no hook wrote one, and the report line
//! (errors.md, "Model switch was blocked by a PreModelSwitch hook"; Agent Plugins Extended §7.5.33).
//! The timeout reason is `timedOutBlocksSwitch` (notices.ts), carried by the hook's block.

use serde::Serialize;

/// The reasons a blocked switch is reported with when no hook wrote one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ModelSwitchReasons {
    /// The hook run ended without a verdict because a hook failed: never taken as approval.
    pub failed: &'static str,
    /// The run ended without a verdict because the control stream closed.
    pub cancelled: &'static str,
}

/// The reasons a blocked switch is reported with when no hook wrote one.
pub const MODEL_SWITCH_REASONS: ModelSwitchReasons = ModelSwitchReasons {
    failed: "a PreModelSwitch hook failed before answering",
    cancelled: "PreModelSwitch hooks were cancelled (the control stream closed) before answering",
};

/// `confirmation required, and this session cannot ask`: an `ask` without a reason where no
/// confirmation can be shown; `/model` in a `-p` run adds `(run /model interactively to confirm)`.
#[must_use]
pub fn cannot_ask_reason(model_command_in_print: bool) -> String {
    let hint = if model_command_in_print {
        " (run /model interactively to confirm)"
    } else {
        ""
    };
    format!("confirmation required, and this session cannot ask{hint}")
}

/// The client cannot tell which `PreModelSwitch` hooks managed plugins deliver (a managed plugin
/// failed to load): the reason begins with what failed and ends with the fixed clause.
#[must_use]
pub fn managed_hooks_unchecked_reason(what_failed: &str) -> String {
    format!("{what_failed}, so organization-managed PreModelSwitch hooks could not be checked")
}

/// The report: `Model switch to <model> was blocked by a PreModelSwitch hook: <reason>` for a switch
/// the user typed or picked; `Model switch blocked by a PreModelSwitch hook: <reason>`, not naming
/// the model, for one an Agent SDK host or Remote Control requested (`model` left out).
#[must_use]
pub fn model_switch_blocked(reason: &str, model: Option<&str>) -> String {
    match model {
        None => format!("Model switch blocked by a PreModelSwitch hook: {reason}"),
        Some(model) => {
            format!("Model switch to {model} was blocked by a PreModelSwitch hook: {reason}")
        }
    }
}

crate::registry::exports! {
    "hooks/verdict/model-switch" {
        "MODEL_SWITCH_REASONS" => ok || MODEL_SWITCH_REASONS;
        "cannotAskReason" => ok |model_command_in_print: Option<bool>| cannot_ask_reason(model_command_in_print.unwrap_or(false));
        "managedHooksUncheckedReason" => ok |what_failed: String| managed_hooks_unchecked_reason(&what_failed);
        "modelSwitchBlocked" => ok |reason: String, model: Option<String>| model_switch_blocked(&reason, model.as_deref());
    }
}
