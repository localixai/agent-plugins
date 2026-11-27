//! The scanners of `PICKED` and `TYPED` in `src/agents/catalog/invoke.ts`, written by hand: those
//! regular expressions read the character before an `@` (`(?<![\w@])`), which the Rust `regex`
//! crate cannot (docs/RUST.md, RS6). The TypeScript has no file of its own for them.

use crate::js::is_whitespace;

/// `(?<![\w@])` before the `@` at byte `at`: no word character (ASCII) or `@` right before it.
fn standalone_at(message: &str, at: usize) -> bool {
    !message
        .get(..at)
        .and_then(|before| before.chars().next_back())
        .is_some_and(|before| before.is_ascii_alphanumeric() || before == '_' || before == '@')
}

/// The text after `@"` of `PICKED` = `/(?<![\w@])@"([^"]+) \(agent\)"/g` at byte `at`: the name
/// and the length of the whole match.
pub(super) fn picked_at(message: &str, at: usize) -> Option<(&str, usize)> {
    let rest = message.get(at..)?.strip_prefix("@\"")?;
    if !standalone_at(message, at) {
        return None;
    }
    let segment = rest.get(..rest.find('"')?)?;
    let name = segment
        .strip_suffix(" (agent)")
        .filter(|name| !name.is_empty())?;
    Some((name, 2 + segment.len() + 1))
}

/// The token after `@agent-` of `TYPED` = `/(?<![\w@])@agent-([^\s"@]+)/g` at byte `at`, and the length
/// of the whole match.
pub(super) fn typed_at(message: &str, at: usize) -> Option<(&str, usize)> {
    let rest = message.get(at..)?.strip_prefix("@agent-")?;
    if !standalone_at(message, at) {
        return None;
    }
    let end = rest
        .find(|character: char| is_whitespace(character) || character == '"' || character == '@')
        .unwrap_or(rest.len());
    let token = rest.get(..end).filter(|token| !token.is_empty())?;
    Some((token, "@agent-".len() + token.len()))
}

/// Every match of a pattern, scanned as `matchAll` does: from the end of the last match, one
/// character further after a failure.
pub(super) fn matches_of(
    message: &str,
    at: fn(&str, usize) -> Option<(&str, usize)>,
) -> Vec<(usize, usize, String)> {
    let mut found = Vec::new();
    let mut next = 0;
    for (index, _) in message.char_indices() {
        if index < next {
            continue;
        }
        if let Some((captured, length)) = at(message, index) {
            found.push((index, length, captured.to_owned()));
            next = index + length;
        }
    }
    found
}
