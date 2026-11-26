//! Port of `src/format/validate/shell-command.ts`, switched.
//!
//! The unquoted-placeholder warning of `claude plugin validate` on a shell-form hook command
//! (oracle 2.1.325, spec §3.2.3 item 5 and J3.20: the placeholders in order of first appearance;
//! the changelog's "leaves `${CLAUDE_PLUGIN_ROOT}` unquoted (it breaks on plugin paths with
//! spaces)"). The validator reads three placeholders; a use inside single or double quotes is
//! fine.

const PLACEHOLDERS: [&str; 3] = [
    "CLAUDE_PLUGIN_DATA",
    "CLAUDE_PLUGIN_ROOT",
    "CLAUDE_PROJECT_DIR",
];

/// The placeholder `^(?:\{(NAMES)\}|(NAMES)(?![A-Za-z0-9_]))` reads at the start of `rest`, the
/// text after a `$`.
fn use_of(rest: &str) -> Option<&'static str> {
    PLACEHOLDERS.iter().copied().find(|name| {
        let braced = rest
            .strip_prefix('{')
            .and_then(|inner| inner.strip_prefix(name))
            .is_some_and(|after| after.starts_with('}'));
        let bare = rest.strip_prefix(name).is_some_and(|after| {
            !after.starts_with(|next: char| next.is_ascii_alphanumeric() || next == '_')
        });
        braced || bare
    })
}

/// The quote state after one character: a quote toggles its own kind (`None` is outside any).
fn advance(state: Option<char>, character: char) -> Option<char> {
    if state.is_none() && (character == '\'' || character == '"') {
        return Some(character);
    }
    if state == Some(character) {
        None
    } else {
        state
    }
}

/// The placeholders a command uses outside any quotes, in order of first appearance, without
/// repeats. `\` escapes the next character outside single quotes.
fn unquoted_placeholders(command: &str) -> Vec<&'static str> {
    let mut found: Vec<&'static str> = Vec::new();
    let mut state: Option<char> = None;
    let mut characters = command.char_indices();
    while let Some((index, character)) = characters.next() {
        if character == '\\' && state != Some('\'') {
            characters.next();
            continue;
        }
        if state.is_none() && character == '$' {
            let name = command.get(index + 1..).and_then(use_of);
            if let Some(name) = name
                && !found.contains(&name)
            {
                found.push(name);
            }
        }
        state = advance(state, character);
    }
    found
}

const ADVICE: &str = "If the expanded path contains a space the command can split into several words and fail. Wrap the placeholder in double quotes, or use exec form: {\"command\": \"<executable>\", \"args\": [\"${CLAUDE_PLUGIN_ROOT}/...\"]}.";

/// The warning's text, or `None` when every placeholder is quoted.
#[must_use]
pub fn unquoted_message(command: &str) -> Option<String> {
    let names = unquoted_placeholders(command);
    if names.is_empty() {
        return None;
    }
    let list = names
        .iter()
        .map(|name| format!("${{{name}}}"))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!(
        "Shell command uses {list} without quotes: {command}. {ADVICE}"
    ))
}

crate::registry::exports! {
    "format/validate/shell-command" {
        "unquotedMessage" => ok |command: String| unquoted_message(&command);
    }
}
