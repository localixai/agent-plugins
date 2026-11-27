//! Port of `src/settings/workflows/keyword.ts`, switched.
//!
//! Whether typing `ultracode` in a prompt starts a dynamic workflow (workflows.md, "Where the
//! keyword works"; settings-reference.md, `workflowKeywordTriggerEnabled`).

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{JsValue, Pattern, code_unit_length, trim_start};
use crate::registry::Port;
use crate::settings::view::PolicyView;
use crate::settings::workflows::enablement::{WorkflowsContext, workflows_decision};

/// How a prompt reaches the session (workflows.md, "Where the keyword works"): typed at the
/// interactive prompt, in an IDE extension panel or in a Remote Control client; sent by an Agent
/// SDK application; passed with `-p`; run as a scheduled task; or relayed from a webhook payload
/// or a pull request comment.
pub const PROMPT_SURFACES: [&str; 7] = [
    "interactive",
    "ideExtension",
    "remoteControl",
    "sdk",
    "print",
    "scheduledTask",
    "relayedEvent",
];

/// An SDK application's stamp on a prompt, by its kind.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct MessageOrigin {
    /// The kind of `SDKMessageOrigin`.
    pub kind: String,
}

/// One prompt.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PromptInput {
    /// One of `PROMPT_SURFACES`.
    pub surface: String,
    /// The text.
    pub text: String,
    /// An SDK application's stamp on the prompt; read only for the `sdk` surface.
    #[serde(default)]
    pub origin: Option<MessageOrigin>,
    /// The person dismissed the highlight for this prompt (`Option+W`, `Alt+W`, or backspace
    /// after it).
    #[serde(default)]
    pub dismissed: Option<bool>,
}

/// The keyword Claude Code highlights in the input, as a range of the text (UTF-16 units).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Highlight {
    /// Where it starts.
    pub start: usize,
    /// Where it ends.
    pub end: usize,
}

/// Whether a prompt triggers, and why not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KeywordVerdict {
    /// It triggers.
    pub triggers: bool,
    /// Why not, when it does not trigger (`KeywordMiss`).
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub reason: Option<&'static str>,
    /// The keyword's range.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub highlight: Option<Highlight>,
}

/// The origin kinds of `SDKMessageOrigin`: only the one for keyboard input is a prompt a person
/// typed.
const TYPED_BY_HUMAN: [(&str, bool); 9] = [
    ("human", true),
    ("channel", false),
    ("peer", false),
    ("task-notification", false),
    ("auto-continuation", false),
    ("coordinator", false),
    ("observer", false),
    ("observer-activity", false),
    ("unclassified", false),
];

fn origin_typed(origin: Option<&MessageOrigin>) -> bool {
    origin.is_some_and(|origin| {
        TYPED_BY_HUMAN
            .iter()
            .any(|(kind, typed)| *kind == origin.kind && *typed)
    })
}

/// Whether a prompt on each surface is one the person typed. An SDK prompt counts only when
/// stamped.
fn typed(surface: &str, origin: Option<&MessageOrigin>) -> bool {
    match surface {
        "interactive" | "ideExtension" | "remoteControl" => true,
        "sdk" => origin_typed(origin),
        _ => false,
    }
}

/// The keyword is `ultracode` (workflows.md names it and no more). Judgment (spec/judgments.md): a
/// whole word in any letter case, anywhere in the prompt, and a prompt that is a slash command
/// (`/effort ultracode`) is not scanned: that command is the effort setting, which the key leaves
/// alone. `/\bultracode\b/i`: without the `u` flag JavaScript folds ASCII letters only, and `\b`
/// is ASCII.
static KEYWORD: Pattern = LazyLock::new(|| Regex::new(r"(?i-u)\bultracode\b").ok());

fn find(text: &str) -> Option<Highlight> {
    if trim_start(text).starts_with('/') {
        return None;
    }
    let found = KEYWORD.as_ref()?.find(text)?;
    let start = code_unit_length(text.get(..found.start()).unwrap_or_default());
    Some(Highlight {
        start,
        end: start + code_unit_length(found.as_str()),
    })
}

const fn miss(reason: &'static str) -> KeywordVerdict {
    KeywordVerdict {
        triggers: false,
        reason: Some(reason),
        highlight: None,
    }
}

/// Whether typing `ultracode` in `prompt` starts a dynamic workflow: workflows are on, the prompt
/// is one a person typed (settings-reference.md, `workflowKeywordTriggerEnabled`; workflows.md,
/// "Where the keyword works"), its highlight was not dismissed, the keyword is in the text and
/// `workflowKeywordTriggerEnabled` is not `false` (default `true`). The key switches the keyword
/// only: `/effort ultracode`, `/workflows` and saved workflow commands do not read it.
pub async fn keyword_trigger(
    view: &dyn PolicyView,
    context: &WorkflowsContext,
    prompt: &PromptInput,
) -> KeywordVerdict {
    if !workflows_decision(view, context).await.enabled {
        return miss("workflowsOff");
    }
    if !typed(&prompt.surface, prompt.origin.as_ref()) {
        return miss("notTypedByHuman");
    }
    if prompt.dismissed == Some(true) {
        return miss("dismissed");
    }
    let Some(highlight) = find(&prompt.text) else {
        return miss("noKeyword");
    };
    if view.get("workflowKeywordTriggerEnabled").await == Some(JsValue::Bool(false)) {
        return miss("keywordSettingOff");
    }
    KeywordVerdict {
        triggers: true,
        reason: None,
        highlight: Some(highlight),
    }
}

crate::registry::exports! {
    "settings/workflows/keyword" {
        "PROMPT_SURFACES" => ok || PROMPT_SURFACES;
        "keywordTrigger" => async ok |view: Port, context: WorkflowsContext, prompt: PromptInput|
            keyword_trigger(&view, &context, &prompt).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keyword_is_a_whole_ascii_word_in_any_case() {
        assert_eq!(find("é UltraCode!"), Some(Highlight { start: 2, end: 11 }));
        assert_eq!(find("ultracodes"), None);
        assert_eq!(find("  /effort ultracode"), None);
    }
}
