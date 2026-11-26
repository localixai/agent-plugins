//! Port of `src/cli/flags/subagent-prompt.ts`, switched.
//!
//! `--append-subagent-system-prompt`, `--append-subagent-system-prompt-file` and
//! `--forward-subagent-text` (cli-reference.md, "CLI flags"; errors.md, "Conflict between a
//! system-prompt flag and its file form"; headless.md, "Follow subagent messages"). The first two
//! append text to every subagent's system prompt and "only apply in non-interactive mode with `-p`";
//! the two "can't be combined". The last forwards subagent text and thinking blocks and "requires
//! `--print` and `--output-format stream-json`"; `CLAUDE_CODE_FORWARD_SUBAGENT_TEXT=1` "enables the
//! same behavior" and, "unlike the flag", "is ignored" outside that mode (env-vars.md). What a flag
//! does when its requirement is missing is not stated: it has no effect (spec/judgments.md).

use super::types::{Contribution, FlagContext, FlagFacts, refuse};
use crate::format::env_flag::is_truthy_env;
use crate::format::issues::warning;
use crate::js::JsValue;

const CONFLICT: &str = "Error: Cannot use both --append-subagent-system-prompt and --append-subagent-system-prompt-file. Please use only one.";

const STREAM_JSON: &str = "stream-json";

/// Whether the flag was given (a member of `given` that is not `undefined`).
fn is_given(context: &FlagContext, flag: &str) -> bool {
    context
        .given
        .get(flag)
        .is_some_and(|value| !matches!(value, JsValue::Undefined))
}

/// Both forms give a refusal; the inline form reports it, so the file form stays silent when both are given.
#[must_use]
pub fn append_subagent_prompt(text: &str, context: &FlagContext) -> Contribution {
    if is_given(context, "--append-subagent-system-prompt-file") {
        return refuse(vec![CONFLICT.to_owned()]);
    }
    if context.facts.print {
        Contribution {
            subagent_prompt_append: Some(text.to_owned()),
            ..Contribution::default()
        }
    } else {
        Contribution::default()
    }
}

/// `--append-subagent-system-prompt-file`.
#[must_use]
pub fn append_subagent_prompt_file(path: &str, context: &FlagContext) -> Contribution {
    if is_given(context, "--append-subagent-system-prompt") {
        return Contribution::default();
    }
    if !context.facts.print {
        return Contribution::default();
    }
    let Some(file) = context.facts.files.get(path) else {
        return refuse(vec![format!(
            "Error: --append-subagent-system-prompt-file file not found: {path}"
        )]);
    };
    Contribution {
        subagent_prompt_append: Some(file.text.clone()),
        ..Contribution::default()
    }
}

fn streams_json(facts: &FlagFacts) -> bool {
    facts.print && facts.output_format.as_deref() == Some(STREAM_JSON)
}

/// The variable alone: ignored, without a warning, outside `--print` with `stream-json`.
#[must_use]
pub fn forward_subagent_text_variable(facts: &FlagFacts) -> Contribution {
    let variable = facts
        .env
        .get("CLAUDE_CODE_FORWARD_SUBAGENT_TEXT")
        .and_then(Option::as_deref);
    if is_truthy_env(variable) && streams_json(facts) {
        Contribution {
            forward_subagent_text: Some(true),
            ..Contribution::default()
        }
    } else {
        Contribution::default()
    }
}

/// `--forward-subagent-text`.
#[must_use]
pub fn forward_subagent_text(_value: bool, context: &FlagContext) -> Contribution {
    if streams_json(&context.facts) {
        return Contribution {
            forward_subagent_text: Some(true),
            ..Contribution::default()
        };
    }
    Contribution {
        warnings: Some(vec![warning(
            "--forward-subagent-text",
            "has no effect without --print and --output-format stream-json",
        )]),
        ..Contribution::default()
    }
}

crate::registry::exports! {
    "cli/flags/subagent-prompt" {
        "appendSubagentPrompt" => ok |text: String, context: FlagContext| append_subagent_prompt(&text, &context);
        "appendSubagentPromptFile" => ok |path: String, context: FlagContext| append_subagent_prompt_file(&path, &context);
        "forwardSubagentTextVariable" => ok |facts: FlagFacts| forward_subagent_text_variable(&facts);
        "forwardSubagentText" => ok |value: bool, context: FlagContext| forward_subagent_text(value, &context);
    }
}
