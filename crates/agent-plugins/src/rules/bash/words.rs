//! Ported from `src/rules/bash/words.ts`; only the core uses it now, so it has no wrapper.
//!
//! The words and redirections of one command (bash(1), "SHELL GRAMMAR" and "REDIRECTION"). An
//! unquoted `<` or `>` ends a word wherever it stands, so `echo x>/etc/x` is `echo x` writing
//! `/etc/x`; a descriptor number or `{name}` written right before the operator is part of it.
//!
//! Positions are UTF-16 code units of the command, as the TypeScript counts them (`js::utf16`).

use serde::{Deserialize, Serialize};

use crate::js::utf16::{replace_all, slice, starts_with_at, text_of, unit_at, units};
use crate::rules::bash::dialect::{BASH_DIALECT, ShellDialect};
use crate::rules::bash::scanner::{is_blank, new_scan, skip_unit};
use crate::rules::bash::unquote::read_word;

/// A word or a redirection operator of a command, with where it starts and ends in the command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Word {
    /// The word as written.
    pub text: String,
    /// Its first code unit in the command.
    pub start: usize,
    /// Past its last code unit.
    pub end: usize,
    /// A redirection operator (`>`, `2>&`, `<<`), whose target is the next word.
    pub operator: bool,
}

/// A word of the command as written and as the shell reads it (`Word` and `WordValue`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellWord {
    /// The word as written.
    pub text: String,
    /// Its first code unit in the command.
    pub start: usize,
    /// Past its last code unit.
    pub end: usize,
    /// A redirection operator (`>`, `2>&`, `<<`), whose target is the next word.
    pub operator: bool,
    /// The word with its quotes, escapes and line continuations removed.
    pub value: String,
    /// A `$` or backtick outside single quotes: a variable or substitution may change the value.
    pub expands: bool,
    /// A `*`, `?` or `[` outside quotes: the shell may expand the word to file names.
    pub globs: bool,
}

/// What a redirection does with its target: writes it, reads it, both (`<>`), names a descriptor
/// (`2>&1`, `<&-`), or holds text (a here-document or here-string).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RedirectKind {
    /// `output`.
    Output,
    /// `input`.
    Input,
    /// `both`.
    Both,
    /// `descriptor`.
    Descriptor,
    /// `here`.
    Here,
}

/// One redirection of a command.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Redirect {
    /// The operator as written, its descriptor included (`2>`, `{fd}>`, `&>>`).
    pub operator: String,
    /// What it does with its target.
    pub kind: RedirectKind,
    /// The word after the operator; `None` when none follows, which the shell rejects.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<ShellWord>,
}

/// One command: its words with every redirection taken out, and the redirections.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShellCommand {
    /// The command as written.
    pub text: String,
    /// Its words.
    pub words: Vec<ShellWord>,
    /// Its redirections.
    pub redirects: Vec<Redirect>,
}

/// The operators, longest first; `<(` and `>(` open a process substitution instead:
/// `/&>>|&>|<<<|<<-|<<|<>|<&|>&|>>|>\||<(?!\()|>(?!\()/y`.
const OPERATORS: [&str; 10] = [
    "&>>", "&>", "<<<", "<<-", "<<", "<>", "<&", ">&", ">>", ">|",
];

/// Whether `unit` is the ASCII character `wanted`.
fn is(unit: Option<u16>, wanted: char) -> bool {
    unit.is_some_and(|unit| u32::from(unit) == u32::from(wanted))
}

fn is_digit(unit: u16) -> bool {
    (u16::from(b'0')..=u16::from(b'9')).contains(&unit)
}

fn is_name_unit(unit: u16, first: bool) -> bool {
    let character = char::from_u32(u32::from(unit)).unwrap_or('\0');
    character.is_ascii_alphabetic() || character == '_' || (!first && character.is_ascii_digit())
}

/// `/^(?:\d+|\{[A-Za-z_][A-Za-z0-9_]*\})$/`.
fn is_descriptor_prefix(text: &[u16]) -> bool {
    if !text.is_empty() && text.iter().all(|&unit| is_digit(unit)) {
        return true;
    }
    let Some(([open, first], rest)) = text.split_first_chunk::<2>() else {
        return false;
    };
    let Some((close, name)) = rest.split_last() else {
        return false;
    };
    is(Some(*open), '{')
        && is(Some(*close), '}')
        && is_name_unit(*first, true)
        && name.iter().all(|&unit| is_name_unit(unit, false))
}

/// `/^(?:\d+-?|-)$/`.
fn is_descriptor_target(text: &str) -> bool {
    let digits = text.strip_suffix('-').unwrap_or(text);
    if digits.is_empty() {
        return text == "-";
    }
    digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn operator_at(command: &[u16], at: usize) -> Option<&'static str> {
    if let Some(operator) = OPERATORS
        .iter()
        .find(|operator| starts_with_at(command, at, operator))
    {
        return Some(operator);
    }
    let substitution = is(unit_at(command, at + 1), '(');
    let single = ["<", ">"]
        .into_iter()
        .find(|operator| starts_with_at(command, at, operator));
    single.filter(|_| !substitution)
}

/// Only line continuations: the shell reads nothing there.
fn is_continuation(text: &[u16], dialect: &ShellDialect) -> bool {
    replace_all(text, &format!("{}\n", dialect.escape), "").is_empty()
}

struct Lexer<'a> {
    command: &'a [u16],
    dialect: &'a ShellDialect,
    words: Vec<Word>,
    /// Where the word being read starts, or `None` between words (`-1`).
    start: Option<usize>,
}

/// Ends the word being read at `end`; a word of only line continuations is no word.
fn close(lexer: &mut Lexer<'_>, end: usize) {
    if let Some(start) = lexer.start {
        let text = slice(lexer.command, start, end);
        if !is_continuation(text, lexer.dialect) {
            let text = text_of(text);
            lexer.words.push(Word {
                text,
                start,
                end,
                operator: false,
            });
        }
    }
    lexer.start = None;
}

/// An operator at `at`, its descriptor prefix (`2`, `{fd}`) taken from the word before it.
fn push_operator(lexer: &mut Lexer<'_>, at: usize, operator: &str) -> usize {
    let prefixed = lexer
        .start
        .filter(|&start| is_descriptor_prefix(slice(lexer.command, start, at)));
    let from = prefixed.unwrap_or(at);
    if prefixed.is_none() {
        close(lexer, at);
    }
    let end = at + operator.len();
    let text = text_of(slice(lexer.command, from, end));
    lexer.words.push(Word {
        text,
        start: from,
        end,
        operator: true,
    });
    lexer.start = None;
    end
}

/// The words and operators of one command, quoted and nested parts kept whole.
#[must_use]
pub fn words_of(command: &str, dialect: Option<&ShellDialect>) -> Vec<Word> {
    let bash = BASH_DIALECT;
    let dialect = dialect.unwrap_or(&bash);
    let mut scan = new_scan(command, dialect, 0);
    let text = units(command);
    let mut lexer = Lexer {
        command: &text,
        dialect,
        words: Vec::new(),
        start: None,
    };
    while scan.pos < text.len() {
        let at = scan.pos;
        let blank = is_blank(unit_at(&text, at));
        let operator = if blank { None } else { operator_at(&text, at) };
        if let Some(operator) = operator {
            scan.pos = push_operator(&mut lexer, at, operator);
        } else if blank {
            close(&mut lexer, at);
            scan.pos += 1;
        } else {
            lexer.start.get_or_insert(at);
            skip_unit(&mut scan);
        }
    }
    close(&mut lexer, scan.pos.min(text.len()));
    lexer.words
}

/// What each operator does with its target, the descriptor prefix left out (`KINDS`).
fn kinds(bare: &str, target: &str) -> Option<RedirectKind> {
    let descriptor = |or| {
        if is_descriptor_target(target) {
            RedirectKind::Descriptor
        } else {
            or
        }
    };
    Some(match bare {
        ">" | ">>" | ">|" | "&>" | "&>>" => RedirectKind::Output,
        "<" => RedirectKind::Input,
        "<>" => RedirectKind::Both,
        ">&" => descriptor(RedirectKind::Output),
        "<&" => descriptor(RedirectKind::Input),
        "<<" | "<<-" | "<<<" => RedirectKind::Here,
        _ => return None,
    })
}

/// `operator.replace(/^(?:\d+|\{[^}]*\})/, '')`.
fn bare_operator(operator: &str) -> &str {
    let digits = operator.bytes().take_while(u8::is_ascii_digit).count();
    if digits > 0 {
        return operator.get(digits..).unwrap_or_default();
    }
    match operator
        .strip_prefix('{')
        .and_then(|rest| rest.split_once('}'))
    {
        Some((_, rest)) => rest,
        None => operator,
    }
}

fn kind_of(operator: &str, target: Option<&ShellWord>) -> RedirectKind {
    let value = target.map_or("", |target| target.value.as_str());
    kinds(bare_operator(operator), value).unwrap_or(RedirectKind::Output)
}

fn shell_word(word: Word, dialect: &ShellDialect) -> ShellWord {
    let value = read_word(&word.text, Some(dialect));
    ShellWord {
        text: word.text,
        start: word.start,
        end: word.end,
        operator: word.operator,
        value: value.value,
        expands: value.expands,
        globs: value.globs,
    }
}

/// One command read into its words and redirections.
#[must_use]
pub fn read_command(text: &str, dialect: Option<&ShellDialect>) -> ShellCommand {
    let bash = BASH_DIALECT;
    let dialect = dialect.unwrap_or(&bash);
    let mut tokens = words_of(text, Some(dialect)).into_iter().peekable();
    let mut words = Vec::new();
    let mut redirects = Vec::new();
    while let Some(token) = tokens.next() {
        if !token.operator {
            words.push(shell_word(token, dialect));
            continue;
        }
        let target = tokens
            .next_if(|next| !next.operator)
            .map(|next| shell_word(next, dialect));
        let kind = kind_of(&token.text, target.as_ref());
        redirects.push(Redirect {
            operator: token.text,
            kind,
            target,
        });
    }
    ShellCommand {
        text: text.to_owned(),
        words,
        redirects,
    }
}

/// A word as the shell passes it and as written. A Windows path is read both ways: its `\` is a
/// separator to the program though Bash reads it as an escape (`C:\Windows`, `\\server\share`).
#[must_use]
pub fn spellings_of(word: &ShellWord) -> Vec<String> {
    if word.text == word.value {
        vec![word.value.clone()]
    } else {
        vec![word.value.clone(), word.text.clone()]
    }
}

/// The command's words as a rule reads them: the name unquoted, one space apart, continuations
/// cut.
#[must_use]
pub fn spoken_form(words: &[ShellWord], dialect: &ShellDialect) -> String {
    let Some((name, rest)) = words.split_first() else {
        return String::new();
    };
    let continuation = format!("{}\n", dialect.escape);
    let args = rest.iter().map(|word| word.text.replace(&continuation, ""));
    std::iter::once(name.value.clone())
        .chain(args)
        .collect::<Vec<_>>()
        .join(" ")
}
