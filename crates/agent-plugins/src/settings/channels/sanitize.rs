//! Port of `src/settings/channels/sanitize.ts`, switched.
//!
//! How Claude Code prepares `description` and `input_preview` before relaying a permission request
//! (channels-reference.md, "Permission request fields"). Pure text rules.
//!
//! Judgments (spec/judgments.md): each neutralized character becomes U+FFFD; the 3,500 code points
//! are split evenly, the start and the end 1,750 each, around the marker; an array of more than
//! 10,000 items counts as "extremely large"; the credential patterns are the common provider
//! prefixes.
//!
//! `inputPreview` serializes the host's tool input, which may hold a cycle or a `bigint` no
//! boundary can copy, so the package keeps it in TypeScript over these text rules (docs/RUST.md,
//! "What stays TypeScript"); it is here for a Rust host and is not an export.

use crate::js::value::write_json_string;
use crate::js::{JsObject, JsValue, is_whitespace, json_stringify, property_order, trim};

/// "Relays text whole up to 3,500 code points."
const RELAY_LIMIT: usize = 3500;
const HALF: usize = RELAY_LIMIT / 2;
const REPLACEMENT: char = '\u{FFFD}';
const UNSERIALIZABLE: &str = "(value unserializable)";
const MAX_ARRAY: usize = 10_000;

/// Direction overrides and isolates, invisible characters, quote and angle-bracket lookalikes.
fn is_neutralized(character: char) -> bool {
    matches!(
        character,
        '\u{061C}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{00AD}'
            | '\u{FEFF}'
            | '\u{2018}'..='\u{201F}'
            | '\u{2039}'
            | '\u{203A}'
            | '\u{02BB}'
            | '\u{02BC}'
            | '\u{FF02}'
            | '\u{FF07}'
            | '\u{2329}'
            | '\u{232A}'
            | '\u{27E8}'
            | '\u{27E9}'
            | '\u{3008}'
            | '\u{3009}'
            | '\u{FF1C}'
            | '\u{FF1E}'
            | '\u{00AB}'
            | '\u{00BB}'
    )
}

/// `\w`: the ASCII word characters (RS6).
const fn is_word(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

/// One alternative of the credential pattern: a literal prefix, then a run of a class, at least
/// `min` long and, for a fixed length, at most `max`.
struct Token {
    prefix: &'static str,
    class: fn(char) -> bool,
    min: usize,
    max: Option<usize>,
}

const fn word_or_dash(character: char) -> bool {
    is_word(character) || character == '-'
}

const fn upper_or_digit(character: char) -> bool {
    character.is_ascii_uppercase() || character.is_ascii_digit()
}

const fn letter_or_digit(character: char) -> bool {
    character.is_ascii_alphanumeric()
}

/// Recognizable provider tokens: only letters, digits, `_` and `-`, so no mask covers shell, path
/// or URL characters. The alternatives of the pattern, in its order; `gh[pousr]_` is five
/// prefixes in one place, tried in the class's order.
const TOKENS: [Token; 10] = [
    Token {
        prefix: "sk-ant-",
        class: word_or_dash,
        min: 10,
        max: None,
    },
    Token {
        prefix: "sk-",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "ghp_",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "gho_",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "ghu_",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "ghs_",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "ghr_",
        class: letter_or_digit,
        min: 20,
        max: None,
    },
    Token {
        prefix: "github_pat_",
        class: is_word,
        min: 20,
        max: None,
    },
    Token {
        prefix: "AKIA",
        class: upper_or_digit,
        min: 16,
        max: Some(16),
    },
    Token {
        prefix: "glpat-",
        class: word_or_dash,
        min: 20,
        max: None,
    },
];

/// `(?<![\w\-/\\.~])` before a token.
fn lookbehind_allows(previous: Option<char>) -> bool {
    previous.is_none_or(|character| {
        !(word_or_dash(character) || matches!(character, '/' | '\\' | '.' | '~'))
    })
}

/// `(?![\w\-/\\]|\.\w)` after a token.
fn lookahead_allows(rest: &[char]) -> bool {
    match rest {
        [first, ..] if word_or_dash(*first) || matches!(*first, '/' | '\\') => false,
        [first, second, ..] if *first == '.' && is_word(*second) => false,
        _ => true,
    }
}

/// The end of the token that starts at `at`, when one does and the characters around it allow it.
/// A shorter run could only stop before a character of its own class, which the lookahead refuses,
/// so the longest run is the only one tried.
fn token_end(chars: &[char], at: usize) -> Option<usize> {
    for token in &TOKENS {
        if let Some(end) = token_at(chars, at, token) {
            return Some(end);
        }
    }
    xox_end(chars, at)
}

fn token_at(chars: &[char], at: usize, token: &Token) -> Option<usize> {
    let rest = chars.get(at..)?;
    let prefix: Vec<char> = token.prefix.chars().collect();
    if !rest.starts_with(&prefix) {
        return None;
    }
    let run_start = at + prefix.len();
    let run = chars.get(run_start..)?;
    let mut length = run
        .iter()
        .take_while(|character| (token.class)(**character))
        .count();
    if let Some(max) = token.max {
        length = length.min(max);
    }
    if length < token.min {
        return None;
    }
    let end = run_start + length;
    lookahead_allows(chars.get(end..)?).then_some(end)
}

/// `xox[abprs]-[\w-]{10,}`.
fn xox_end(chars: &[char], at: usize) -> Option<usize> {
    let rest = chars.get(at..)?;
    let head = ['x', 'o', 'x'];
    if !rest.starts_with(&head) || !matches!(rest.get(3), Some('a' | 'b' | 'p' | 'r' | 's')) {
        return None;
    }
    if rest.get(4) != Some(&'-') {
        return None;
    }
    let run = rest.get(5..)?;
    let length = run
        .iter()
        .take_while(|character| word_or_dash(**character))
        .count();
    if length < 10 {
        return None;
    }
    let end = at + 5 + length;
    lookahead_allows(chars.get(end..)?).then_some(end)
}

/// `[REDACTED]` "in place of a recognizable provider credential token".
#[must_use]
pub fn mask_credentials(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut at = 0;
    while let Some(character) = chars.get(at) {
        let previous = at
            .checked_sub(1)
            .and_then(|index| chars.get(index))
            .copied();
        let end = if lookbehind_allows(previous) {
            token_end(&chars, at)
        } else {
            None
        };
        match end {
            Some(end) => {
                out.push_str("[REDACTED]");
                at = end;
            }
            None => {
                out.push(*character);
                at += 1;
            }
        }
    }
    out
}

/// "For a longer value, you receive its start and its end around a counted `⋯ N code points
/// elided ⋯` marker."
#[must_use]
pub fn elide(text: &str) -> String {
    let points: Vec<char> = text.chars().collect();
    if points.len() <= RELAY_LIMIT {
        return text.to_owned();
    }
    let cut = points.len() - RELAY_LIMIT;
    let start: String = points.iter().take(HALF).collect();
    let end: String = points.iter().skip(points.len() - HALF).collect();
    format!("{start}⋯ {cut} code points elided ⋯{end}")
}

/// Mask, neutralize, fold each run of whitespace to one space, then elide.
#[must_use]
pub fn sanitize_relay_text(text: &str) -> String {
    let mut folded = String::new();
    let mut in_run = false;
    for character in mask_credentials(text).chars() {
        let character = if is_neutralized(character) {
            REPLACEMENT
        } else {
            character
        };
        if is_whitespace(character) {
            if !in_run {
                folded.push(' ');
            }
            in_run = true;
        } else {
            folded.push(character);
            in_run = false;
        }
    }
    elide(&folded)
}

fn serializable(value: &JsValue) -> Option<String> {
    if value
        .as_array()
        .is_some_and(|items| items.len() > MAX_ARRAY)
    {
        return None;
    }
    json_stringify(value)
}

fn field_text(value: &JsValue) -> String {
    if let JsValue::String(text) = value {
        let mut quoted = String::new();
        write_json_string(&sanitize_relay_text(text), &mut quoted);
        return quoted;
    }
    serializable(value).map_or_else(
        || UNSERIALIZABLE.to_owned(),
        |json| sanitize_relay_text(&json),
    )
}

/// The `input_preview`: the arguments "as JSON-shaped display text, keyed per top-level field". The
/// limit applies to each field separately and a string keeps its own quotes; a field that cannot
/// be serialized shows `(value unserializable)` and keeps its key; key names are masked too.
#[must_use]
pub fn input_preview(input: &JsObject) -> String {
    let fields: Vec<String> = property_order(input)
        .into_iter()
        .map(|(key, value)| {
            let mut quoted = String::new();
            write_json_string(&mask_credentials(key), &mut quoted);
            format!("{quoted}: {}", field_text(value))
        })
        .collect();
    format!("{{{}}}", fields.join(", "))
}

/// "When the model gives no description, the field is the constant `Run shell command`."
const NO_DESCRIPTION: &str = "Run shell command";

/// The description as relayed: the model's text sanitized, or the constant when there is none.
#[must_use]
pub fn relay_description(description: Option<&str>) -> String {
    match description {
        Some(text) if !trim(text).is_empty() => sanitize_relay_text(text),
        Some(_) | None => NO_DESCRIPTION.to_owned(),
    }
}

crate::registry::exports! {
    "settings/channels/sanitize" {
        "maskCredentials" => ok |text: String| mask_credentials(&text);
        "elide" => ok |text: String| elide(&text);
        "sanitizeRelayText" => ok |text: String| sanitize_relay_text(&text);
        "relayDescription" => ok |description: Option<String>| relay_description(description.as_deref());
    }
}
