//! Port of `src/agents/handback-denial.ts`, switched.
//!
//! A hook that denies `SubagentHandback` (hooks.md, "PreToolUse `ask` decisions in auto mode"): "Claude
//! Code then tells the subagent, up to three times, that its report wasn't delivered and to call the
//! tool again. If your hook keeps denying, the subagent ends without delivering a report through this
//! tool, and the conversation waiting on it gets a note saying so." The tool call, the message to the
//! subagent and the note are the host's; this counts.

use serde::{Deserialize, Serialize};

/// How many denials the subagent is told about.
pub const HANDBACK_DENIALS_TOLD: f64 = 3.0;

/// What follows a denial of a handback.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HandbackAfterDenial {
    /// Tell the subagent its report was not delivered and to call the tool again.
    TellSubagent,
    /// End the subagent, and tell the conversation waiting on it that no report came through the tool.
    EndWithoutReport,
}

/// What follows the `denials`-th denial of one subagent's handback (1 for the first).
#[must_use]
pub fn after_handback_denial(denials: f64) -> HandbackAfterDenial {
    if denials <= HANDBACK_DENIALS_TOLD {
        HandbackAfterDenial::TellSubagent
    } else {
        HandbackAfterDenial::EndWithoutReport
    }
}

crate::registry::exports! {
    "agents/handback-denial" {
        "HANDBACK_DENIALS_TOLD" => ok || HANDBACK_DENIALS_TOLD;
        "afterHandbackDenial" => ok |denials: f64| after_handback_denial(denials);
    }
}
