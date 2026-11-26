//! Port of `src/hooks/output/defer.ts`, switched.
//!
//! Whether a `PreToolUse` `defer` pauses the turn (hooks.md, "Defer a tool call for later"): "Claude
//! Code honors `defer` only in non-interactive mode with the `-p` flag, and only when Claude makes a
//! single tool call in the turn". "In interactive sessions: it logs a warning and ignores the hook
//! result"; with "several tool calls at once: `defer` is ignored with a warning and the tool proceeds
//! through the normal permission flow". The merge says `defer` won; only the host knows the turn.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};

/// What of the merged effects the resolution reads (`Pick<EventEffects, 'permissionDecision'>`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferEffects {
    /// The merged decision.
    #[serde(default)]
    pub permission_decision: Option<String>,
}

/// What the host knows of the turn the tool call belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeferTurn {
    /// An interactive terminal session, as `HookSession.interactive`.
    pub interactive: bool,
    /// How many tool calls Claude made in this turn.
    pub tool_calls: f64,
}

/// What a merged `defer` comes to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum DeferResolution {
    /// The merged output holds no `defer`: nothing to resolve.
    None,
    /// The turn ends with `stop_reason: "tool_deferred"` and the pending call stays in the transcript.
    Deferred,
    /// `defer` does not apply: log `warning`, run the normal permission flow with no hook decision.
    Ignored {
        /// What to log.
        warning: PluginIssue,
    },
}

const PATH: &str = "hooks.PreToolUse.permissionDecision";

/// The resolution of the merged `effects` of a `PreToolUse` dispatch in `turn`.
#[must_use]
pub fn resolve_defer(effects: &DeferEffects, turn: DeferTurn) -> DeferResolution {
    if effects.permission_decision.as_deref() != Some("defer") {
        return DeferResolution::None;
    }
    if turn.interactive {
        return DeferResolution::Ignored {
            warning: warning(
                PATH,
                "\"defer\" is honored only in non-interactive mode with the -p flag; the hook result is ignored",
            ),
        };
    }
    if (turn.tool_calls - 1.0).abs() > 0.0 || turn.tool_calls.is_nan() {
        return DeferResolution::Ignored {
            warning: warning(
                PATH,
                "\"defer\" is honored only when Claude makes a single tool call in the turn; it is ignored and the tool proceeds through the normal permission flow",
            ),
        };
    }
    DeferResolution::Deferred
}

crate::registry::exports! {
    "hooks/output/defer" {
        "resolveDefer" => ok |effects: DeferEffects, turn: DeferTurn| resolve_defer(&effects, turn);
    }
}
