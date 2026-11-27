//! Port of `src/hooks/output/terminal-sequence.ts`, switched.
//!
//! The `terminalSequence` field (hooks.md, "Emit terminal notifications"): "Restricted to OSC
//! `0`/`1`/`2`/`9`/`99`/`777` and BEL. If the value contains anything outside the allowlist, the
//! field is ignored." The value is "a string of one or more allowlisted escape sequences"; "Sequences
//! may be terminated with BEL or with ST", and CSI sequences, OSC palette sequences, OSC 8
//! hyperlinks, OSC 52 clipboard writes and OSC 1337 are rejected.

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';
const ALLOWED_OSC: [&str; 6] = ["0", "1", "2", "9", "99", "777"];

/// A control character cannot sit in a sequence's text: it could end it early or start another.
fn is_control(character: char) -> bool {
    let code = u32::from(character);
    code < 0x20 || (0x7f..=0x9f).contains(&code)
}

/// The end of the terminator that follows `at`, or `None` when the text there is not one: `BEL`, or
/// `ST` (`ESC` and a backslash).
fn terminator_end(text: &[char], at: usize) -> Option<usize> {
    match (text.get(at), text.get(at + 1)) {
        (Some(&BEL), _) => Some(at + 1),
        (Some(&ESC), Some(&'\\')) => Some(at + 2),
        _ => None,
    }
}

/// The end of the OSC sequence that starts at `at`, or `None` when it is not an allowed one.
fn osc_end(text: &[char], at: usize) -> Option<usize> {
    let mut cursor = at + 2;
    let number_start = cursor;
    while text
        .get(cursor)
        .is_some_and(|&character| character != ';' && !is_control(character))
    {
        cursor += 1;
    }
    let number: String = text.get(number_start..cursor)?.iter().collect();
    if !ALLOWED_OSC.contains(&number.as_str()) {
        return None;
    }
    while let Some(&character) = text.get(cursor) {
        if let Some(end) = terminator_end(text, cursor) {
            return Some(end);
        }
        if is_control(character) {
            return None;
        }
        cursor += 1;
    }
    None
}

/// The end of the allowlisted sequence at `at`: a bare BEL or an allowed OSC; else `None`.
fn sequence_end(text: &[char], at: usize) -> Option<usize> {
    match (text.get(at), text.get(at + 1)) {
        (Some(&BEL), _) => Some(at + 1),
        (Some(&ESC), Some(&']')) => osc_end(text, at),
        _ => None,
    }
}

/// The value itself when every part is allowlisted, else `None`: the field is ignored.
#[must_use]
pub fn allowed_terminal_sequence(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let text: Vec<char> = value.chars().collect();
    let mut cursor = 0;
    while cursor < text.len() {
        cursor = sequence_end(&text, cursor)?;
    }
    Some(value.to_owned())
}

crate::registry::exports! {
    "hooks/output/terminal-sequence" {
        "allowedTerminalSequence" => ok |value: String| allowed_terminal_sequence(&value);
    }
}
