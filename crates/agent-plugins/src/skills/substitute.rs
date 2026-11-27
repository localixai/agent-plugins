//! Port of `src/skills/substitute.ts`, switched.
//!
//! Argument substitution (skills.md, "Available string substitutions" and "Pass arguments to
//! skills"): `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N` and `$name` over the original text in one pass, so
//! an argument value is inserted as literal text and never expanded. `${CLAUDE_*}` is substituted
//! afterwards, over the result, as the docs say: "Claude Code still replaces `${CLAUDE_*}`
//! variables such as `${CLAUDE_SKILL_DIR}` after it inserts the arguments".
//!
//! Positions are byte offsets of the text where the TypeScript counts UTF-16 code units: every
//! position is read back from the same text (an `Insert` reads its own map of the command), and
//! every character the scan looks for is ASCII, so the answers are the same.

use serde::{Deserialize, Serialize};

use super::arguments::parse_arguments;
use super::variables::{SkillVariables, substitute_variables};
use crate::format::issues::PluginIssue;
use crate::js::{code_unit_length, is_safe_integer, number, trim};
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::Port;

/// What the arguments are read from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArgumentInput {
    /// The text after the command name, as typed.
    pub raw_arguments: String,
    /// `arguments` of the frontmatter: the names of `$name`, in argument order.
    pub names: Vec<String>,
}

/// What a placeholder at one position stands for. `value` `None`: it stays as written.
#[derive(Clone, Debug, PartialEq)]
pub struct Placeholder {
    /// Its length in the text.
    pub length: usize,
    /// What it is replaced with.
    pub value: Option<String>,
    /// Whether it counts as receiving an argument (decides the `ARGUMENTS:` append).
    pub receives: bool,
    /// The argument it reads, 0-based; absent for `$ARGUMENTS`, the whole text.
    pub index: Option<f64>,
}

struct Typed<'a> {
    input: &'a ArgumentInput,
    args: Vec<String>,
}

type Recognizer = fn(&str, usize, &Typed<'_>) -> Option<Placeholder>;

/// The ASCII digits at the start of `text` (`\d+`), if any.
fn digits_at(text: &str) -> Option<&str> {
    let end = text.bytes().take_while(u8::is_ascii_digit).count();
    text.get(..end).filter(|digits| !digits.is_empty())
}

/// `/[A-Za-z0-9_]/`.
const fn is_identifier_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The argument at a 0-based index; a number too large to be an index reads nothing.
fn argument_at(typed: &Typed<'_>, digits: &str) -> Option<String> {
    let index = number(digits);
    if !is_safe_integer(index) {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a safe integer of digits"
    )]
    let at = index as usize;
    typed.args.get(at).cloned()
}

fn indexed_placeholder(length: usize, digits: &str, typed: &Typed<'_>) -> Placeholder {
    let value = argument_at(typed, digits);
    let receives = value.is_some();
    Placeholder {
        length,
        value,
        receives,
        index: Some(number(digits)),
    }
}

/// `/\$ARGUMENTS\[(\d+)\]/y`.
fn indexed_form(text: &str, at: usize, typed: &Typed<'_>) -> Option<Placeholder> {
    let rest = text.get(at..)?.strip_prefix("$ARGUMENTS[")?;
    let digits = digits_at(rest)?;
    rest.get(digits.len()..)?
        .starts_with(']')
        .then(|| indexed_placeholder("$ARGUMENTS[]".len() + digits.len(), digits, typed))
}

/// `/\$ARGUMENTS(?![A-Za-z0-9_])/y`.
fn whole_form(text: &str, at: usize, typed: &Typed<'_>) -> Option<Placeholder> {
    let rest = text.get(at..)?.strip_prefix("$ARGUMENTS")?;
    let length = "$ARGUMENTS".len();
    (!rest.bytes().next().is_some_and(is_identifier_char)).then(|| Placeholder {
        length,
        value: Some(typed.input.raw_arguments.clone()),
        receives: true,
        index: None,
    })
}

/// `/\$(\d+)/y`.
fn shorthand_form(text: &str, at: usize, typed: &Typed<'_>) -> Option<Placeholder> {
    let rest = text.get(at..)?.strip_prefix('$')?;
    let digits = digits_at(rest)?;
    Some(indexed_placeholder(1 + digits.len(), digits, typed))
}

/// `$name` for a declared name: the longest declared name wins, and a name is read whole.
fn named_form(text: &str, at: usize, typed: &Typed<'_>) -> Option<Placeholder> {
    let mut candidates: Vec<&String> = typed
        .input
        .names
        .iter()
        .filter(|name| !name.is_empty())
        .collect();
    candidates.sort_by_key(|name| std::cmp::Reverse(code_unit_length(name)));
    let after = text.get(at + 1..)?;
    for name in candidates {
        let end = 1 + name.len();
        let whole = after.starts_with(name.as_str())
            && !text
                .as_bytes()
                .get(at + end)
                .copied()
                .is_some_and(is_identifier_char);
        if whole {
            let index = typed.input.names.iter().position(|known| known == name);
            let value = index
                .and_then(|index| typed.args.get(index))
                .cloned()
                .unwrap_or_default();
            #[expect(clippy::cast_precision_loss, reason = "a list's position")]
            let index = index.map(|index| index as f64);
            return Some(Placeholder {
                length: end,
                value: Some(value),
                receives: true,
                index,
            });
        }
    }
    None
}

/// In order: `$ARGUMENTS[N]` before `$ARGUMENTS`, which would otherwise take its first part.
const RECOGNIZERS: [Recognizer; 4] = [indexed_form, whole_form, shorthand_form, named_form];

fn recognize(text: &str, at: usize, typed: &Typed<'_>) -> Option<Placeholder> {
    RECOGNIZERS
        .iter()
        .find_map(|recognizer| recognizer(text, at, typed))
}

/// Each `$`, with the backslash directly before it when there is one (`/\\?\$/g`): where the
/// match starts and its length.
fn dollars(content: &str) -> Vec<(usize, usize)> {
    let bytes = content.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        if byte == b'\\' && bytes.get(at + 1) == Some(&b'$') {
            found.push((at, 2));
            at += 2;
        } else {
            if byte == b'$' {
                found.push((at, 1));
            }
            at += 1;
        }
    }
    found
}

struct Scanned {
    text: String,
    received: bool,
}

/// What a placeholder with a value is written as, given where its `$` is: default the value.
pub trait Insert {
    /// The text for `placeholder`, whose `value` is set, at `at`.
    fn insert(&mut self, placeholder: &Placeholder, value: &str, at: usize) -> String;
}

struct PasteValue;

impl Insert for PasteValue {
    fn insert(&mut self, _placeholder: &Placeholder, value: &str, _at: usize) -> String {
        value.to_owned()
    }
}

/// A placeholder with no value stays as written; one with a value is what `insert` writes.
fn inserted_of(
    placeholder: &Placeholder,
    written: &str,
    at: usize,
    insert: &mut dyn Insert,
) -> String {
    match &placeholder.value {
        None => written.to_owned(),
        Some(value) => insert.insert(placeholder, value, at),
    }
}

/// One pass. A single backslash directly before a placeholder escapes it: the backslash goes and
/// the placeholder stays as text. After a backslash that is itself preceded by one (`\\$1`) the
/// placeholder expands and both backslashes stay. A backslash before a `$` that is no placeholder
/// stays as written.
fn scan(content: &str, typed: &Typed<'_>, insert: &mut dyn Insert) -> Scanned {
    let mut out = String::new();
    let mut cursor = 0;
    let mut received = false;
    for (index, length) in dollars(content) {
        let slash = length == 2;
        let dollar = index + length - 1;
        // A `$` inside a name already consumed is part of that name, not a placeholder of its own.
        if dollar < cursor {
            continue;
        }
        let Some(placeholder) = recognize(content, dollar, typed) else {
            continue;
        };
        let written = content
            .get(dollar..dollar + placeholder.length)
            .unwrap_or_default();
        let before = index
            .checked_sub(1)
            .and_then(|at| content.as_bytes().get(at));
        let escaped = slash && before != Some(&b'\\');
        out.push_str(content.get(cursor..index).unwrap_or_default());
        if escaped {
            out.push_str(written);
        } else {
            if slash {
                out.push('\\');
            }
            out.push_str(&inserted_of(&placeholder, written, dollar, insert));
        }
        received = received || (!escaped && placeholder.receives);
        cursor = dollar + placeholder.length;
    }
    out.push_str(content.get(cursor..).unwrap_or_default());
    Scanned {
        text: out,
        received,
    }
}

/// Judgment: the appended line is `ARGUMENTS: <value>` after a blank line (the docs give the line,
/// not the separator), with the arguments as typed.
fn appended(raw_arguments: &str) -> String {
    format!("\n\nARGUMENTS: {raw_arguments}")
}

/// `text` with the `ARGUMENTS:` line a body that receives no argument gets, when arguments were
/// passed: a core package's skill, whose body is never scanned (Agent Plugins Extended §18.1.5 item 5).
#[must_use]
pub fn append_arguments(text: &str, raw_arguments: &str) -> String {
    if trim(raw_arguments).is_empty() {
        text.to_owned()
    } else {
        text.to_owned() + &appended(raw_arguments)
    }
}

fn typed_of(input: &ArgumentInput) -> Typed<'_> {
    Typed {
        input,
        args: parse_arguments(&input.raw_arguments),
    }
}

/// `substituteArguments` over the pieces of one text, for a text cut at its `` !`command` ``
/// placeholders (`skills/expand`): a placeholder in any piece counts as receiving the arguments, and
/// the `ARGUMENTS:` line, when none did, ends the last piece. Each piece is scanned alone, so an
/// argument value inserted into one can never reach another.
#[must_use]
pub fn substitute_argument_pieces(pieces: &[String], input: &ArgumentInput) -> Vec<String> {
    let typed = typed_of(input);
    let scanned: Vec<Scanned> = pieces
        .iter()
        .map(|piece| scan(piece, &typed, &mut PasteValue))
        .collect();
    let append =
        !scanned.iter().any(|piece| piece.received) && !trim(&input.raw_arguments).is_empty();
    let last = scanned.len().checked_sub(1);
    scanned
        .into_iter()
        .enumerate()
        .map(|(index, piece)| {
            if append && Some(index) == last {
                piece.text + &appended(&input.raw_arguments)
            } else {
                piece.text
            }
        })
        .collect()
}

/// The placeholders of one text written by `insert`, with the same recognition and escapes as the
/// substitution: how a `!` command binds its arguments (`shell-arguments.ts`). No `ARGUMENTS:` line.
pub fn rewrite_arguments(content: &str, input: &ArgumentInput, insert: &mut dyn Insert) -> String {
    scan(content, &typed_of(input), insert).text
}

/// `$ARGUMENTS`, `$ARGUMENTS[N]`, `$N` and `$name` in `content`. With arguments typed and no
/// placeholder receiving one, `ARGUMENTS: <value>` is appended to the end.
#[must_use]
pub fn substitute_arguments(content: &str, input: &ArgumentInput) -> String {
    substitute_argument_pieces(&[content.to_owned()], input).concat()
}

/// `ArgumentInput` with the scope `${CLAUDE_*}` resolves against.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubstitutionInput<L = Port, E = Port> {
    /// The text after the command name, as typed.
    pub raw_arguments: String,
    /// `arguments` of the frontmatter: the names of `$name`, in argument order.
    pub names: Vec<String>,
    /// What `${CLAUDE_*}` resolves against.
    pub vars: SkillVariables<L, E>,
}

/// A text substituted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Substitution {
    /// The text.
    pub text: String,
    /// What the substitution found.
    pub issues: Vec<PluginIssue>,
}

/// Arguments first, then `${CLAUDE_*}` over the result (so over the inserted arguments too).
pub async fn substitute<L: Locations + Clone, E: Environment>(
    content: &str,
    input: &SubstitutionInput<L, E>,
    path: &str,
) -> Substitution {
    let arguments = ArgumentInput {
        raw_arguments: input.raw_arguments.clone(),
        names: input.names.clone(),
    };
    let done = substitute_variables(
        &substitute_arguments(content, &arguments),
        &input.vars,
        path,
    )
    .await;
    Substitution {
        text: done.text,
        issues: done.issues,
    }
}

crate::registry::exports! {
    "skills/substitute" {
        "substituteArgumentPieces" => ok |pieces: Vec<String>, input: ArgumentInput| substitute_argument_pieces(&pieces, &input);
        "substituteArguments" => ok |content: String, input: ArgumentInput| substitute_arguments(&content, &input);
        "substitute" => async ok |content: String, input: SubstitutionInput, path: Option<String>| substitute(&content, &input, path.as_deref().unwrap_or("")).await;
    }
}
