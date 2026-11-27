//! Port of `src/agents/prompt-surface.ts`, switched.
//!
//! Where a subagent's permission prompts appear (tools-reference.md, "Agent tool behavior").

use serde::{Deserialize, Serialize};

/// Where the prompts of a subagent show: `call`, the same prompts as the main conversation, "at
/// the moment each tool call happens"; `main-session`, surfaced in the main session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppearsIn {
    /// The same prompts as the main conversation, "at the moment each tool call happens".
    Call,
    /// Surfaced in the main session.
    MainSession,
}

/// Where a subagent's permission prompts appear (tools-reference.md, "Agent tool behavior").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubagentPromptSurface {
    /// Where the prompt shows.
    pub appears_in: AppearsIn,
    /// "The prompt names which subagent is asking."
    pub names_subagent: bool,
    /// "Pressing Esc denies that one tool call without stopping the subagent."
    pub esc_denies_one_call: bool,
}

const FOREGROUND: SubagentPromptSurface = SubagentPromptSurface {
    appears_in: AppearsIn::Call,
    names_subagent: false,
    esc_denies_one_call: false,
};

const BACKGROUND: SubagentPromptSurface = SubagentPromptSurface {
    appears_in: AppearsIn::MainSession,
    names_subagent: true,
    esc_denies_one_call: true,
};

/// "Launching the subagent doesn't itself prompt for permission": its own tool calls are checked
/// against the permission rules as it runs, and where the prompt shows depends on whether it runs in
/// the foreground or the background. Judgment (spec/judgments.md): the foreground prompt does not name
/// the subagent, because the docs say it is the one the main conversation shows.
#[must_use]
pub const fn permission_subagent_prompt_surface(background: bool) -> SubagentPromptSurface {
    if background { BACKGROUND } else { FOREGROUND }
}

crate::registry::exports! {
    "agents/prompt-surface" {
        "permissionSubagentPromptSurface" => ok |background: bool| permission_subagent_prompt_surface(background);
    }
}
