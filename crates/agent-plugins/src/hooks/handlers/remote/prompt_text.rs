//! Port of `src/hooks/handlers/remote/prompt-text.ts`, switched.
//!
//! The text a `prompt` or `agent` hook sends (hooks.md, "Prompt and agent hook fields" and "Prompt
//! hook configuration"): "Use `$ARGUMENTS` as a placeholder for the hook input JSON. Escape with a
//! backslash to include literal text: `\$1.00` renders as `$1.00`", and "If `$ARGUMENTS` is not
//! present, input JSON is appended to the prompt".

/// An escaped dollar sign, or the placeholder. The escape comes first, so `\$ARGUMENTS` is text.
const ESCAPED: &str = "\\$";
const PLACEHOLDER: &str = "$ARGUMENTS";

/// Judgment (spec/judgments.md): the input is appended after a blank line.
const APPEND_SEPARATOR: &str = "\n\n";

/// The prompt with every `$ARGUMENTS` replaced by `inputJson` and every `\$` by `$`, in one pass so
/// the input JSON is never unescaped or searched for the placeholder again; the input alone is
/// appended when the prompt has no placeholder.
#[must_use]
pub fn compose_prompt(prompt: &str, input_json: &str) -> String {
    let mut has_placeholder = false;
    let mut text = String::new();
    let mut rest = prompt;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix(ESCAPED) {
            text.push('$');
            rest = after;
        } else if let Some(after) = rest.strip_prefix(PLACEHOLDER) {
            has_placeholder = true;
            text.push_str(input_json);
            rest = after;
        } else {
            let mut characters = rest.chars();
            if let Some(character) = characters.next() {
                text.push(character);
            }
            rest = characters.as_str();
        }
    }
    if has_placeholder {
        text
    } else {
        format!("{text}{APPEND_SEPARATOR}{input_json}")
    }
}

crate::registry::exports! {
    "hooks/handlers/remote/prompt-text" {
        "composePrompt" => ok |prompt: String, input_json: String| compose_prompt(&prompt, &input_json);
    }
}
