//! Ported from `src/rules/decide/sed-script.ts`; only the core uses it now, so it has no wrapper.
//!
//! Whether a `sed` call only edits the files it names (permission-modes.md, "Auto-approve file edits
//! with acceptEdits mode" lists `sed` as an edit of its operands). A script can also write another
//! file (`w`, `W`, the `s///w` flag), run a command (`e`, the `s///e` flag) or read one (`r`, `R`),
//! and `-f` reads the script from a file the kit does not see: none of those is an edit of the
//! operands, so the call is not approved (spec/judgments.md). The reading is GNU sed's; a script the
//! reader does not know is unsafe.
//!
//! The TypeScript reads the script by UTF-16 code unit (`text[at]`, `at += 2` past a backslash), so
//! the cursor here walks code units too: a character above U+FFFF is two of them, as there.

use std::sync::LazyLock;

use regex::Regex;

use crate::js::{Pattern, regex_test};
use crate::rules::bash::safe_env::ShellWord;

struct Cursor {
    text: Vec<u16>,
    at: usize,
}

/// The code unit at the cursor, `None` past the end (the TypeScript's `''`).
fn peek(cursor: &Cursor) -> Option<u16> {
    cursor.text.get(cursor.at).copied()
}

/// The code unit of a character the reader names, each of them ASCII.
fn unit(character: char) -> u16 {
    u16::try_from(u32::from(character)).unwrap_or(u16::MAX)
}

/// Text up to an unescaped `delimiter`, the cursor left past it; false when it never comes.
fn skip_delimited(cursor: &mut Cursor, delimiter: Option<u16>) -> bool {
    while cursor.at < cursor.text.len() {
        let char = peek(cursor);
        cursor.at += if char == Some(unit('\\')) { 2 } else { 1 };
        if char == delimiter {
            return true;
        }
    }
    false
}

fn skip_while(cursor: &mut Cursor, pattern: fn(u16) -> bool) {
    while cursor.at < cursor.text.len() && peek(cursor).is_some_and(pattern) {
        cursor.at += 1;
    }
}

/// JavaScript's `\s` on one code unit: a lone surrogate is none.
fn is_space(code: u16) -> bool {
    char::from_u32(u32::from(code)).is_some_and(crate::js::is_whitespace)
}

fn is_in(code: u16, set: &str) -> bool {
    set.encode_utf16().any(|member| member == code)
}

/// One address: a line number, `$`, `/regex/` or `\cregexc`, with its `I` and `M` flags.
fn skip_address(cursor: &mut Cursor) -> bool {
    let char = peek(cursor);
    if char.is_some_and(|code| is_in(code, "0123456789$")) {
        skip_while(cursor, |code| is_in(code, "0123456789$~"));
        return true;
    }
    let custom = char == Some(unit('\\'));
    if char != Some(unit('/')) && !custom {
        return true;
    }
    cursor.at += if custom { 2 } else { 1 };
    let delimiter = if custom {
        cursor.text.get(cursor.at - 1).copied()
    } else {
        Some(unit('/'))
    };
    let closed = skip_delimited(cursor, delimiter);
    skip_while(cursor, |code| is_in(code, "IM"));
    closed
}

/// The address range of a command and its `!`.
fn skip_addresses(cursor: &mut Cursor) -> bool {
    if !skip_address(cursor) {
        return false;
    }
    if peek(cursor) == Some(unit(',')) {
        cursor.at += 1;
        if peek(cursor) == Some(unit('+')) || peek(cursor) == Some(unit('~')) {
            cursor.at += 1;
        }
        if !skip_address(cursor) {
            return false;
        }
    }
    skip_while(cursor, |code| is_space(code) || code == unit('!'));
    true
}

/// `s` flags that change only what is printed or matched; `w` writes a file and `e` runs the result.
fn substitute_flags(code: u16) -> bool {
    is_in(code, "gpiImM0123456789")
}

fn substitute(cursor: &mut Cursor) -> bool {
    let delimiter = peek(cursor);
    cursor.at += 1;
    if delimiter.is_none() || delimiter == Some(unit('\n')) || delimiter == Some(unit('\\')) {
        return false;
    }
    if !skip_delimited(cursor, delimiter) || !skip_delimited(cursor, delimiter) {
        return false;
    }
    skip_while(cursor, substitute_flags);
    true
}

fn transliterate(cursor: &mut Cursor) -> bool {
    let delimiter = peek(cursor);
    cursor.at += 1;
    delimiter.is_some() && skip_delimited(cursor, delimiter) && skip_delimited(cursor, delimiter)
}

/// `a`, `i`, `c`, `#` and the labels of `b`, `t`, `T`, `:` run to the end of the line or the `;`.
fn to_line_end(cursor: &mut Cursor) -> bool {
    skip_while(cursor, |code| code != unit('\n'));
    true
}
fn to_command_end(cursor: &mut Cursor) -> bool {
    skip_while(cursor, |code| !is_in(code, "\n;}"));
    true
}
const fn bare(_cursor: &mut Cursor) -> bool {
    true
}
fn with_number(cursor: &mut Cursor) -> bool {
    skip_while(cursor, |code| is_space(code) || is_in(code, "0123456789"));
    true
}

type Read = fn(&mut Cursor) -> bool;

/// The commands that touch nothing but the pattern and hold spaces and the output.
const COMMANDS: [(&str, Read); 6] = [
    ("s", substitute),
    ("y", transliterate),
    ("aic#", to_line_end),
    ("btT:v", to_command_end),
    ("qQlL", with_number),
    ("pdDnNgGhHxz={}", bare),
];

/// `COMMANDS.get(name)`: each name of a group is one command.
fn command_of(name: Option<u16>) -> Option<Read> {
    let name = name?;
    COMMANDS
        .iter()
        .find(|(names, _)| is_in(name, names))
        .map(|(_, read)| *read)
}

/// One command and what may follow it: a `;`, a newline, a `}` or the end.
fn read_command(cursor: &mut Cursor) -> bool {
    if !skip_addresses(cursor) {
        return false;
    }
    let name = peek(cursor);
    let read = command_of(name);
    cursor.at += 1;
    if !read.is_some_and(|read| read(cursor)) {
        return false;
    }
    skip_while(cursor, |code| is_in(code, " \t"));
    // A `{` opens a block: the next command may follow it on the same line.
    name == Some(unit('{')) || peek(cursor).is_none_or(|code| is_in(code, ";\n}"))
}

/// Whether every command of a sed script is one of `COMMANDS`.
fn is_safe_sed_script(script: &str) -> bool {
    let mut cursor = Cursor {
        text: script.encode_utf16().collect(),
        at: 0,
    };
    loop {
        skip_while(&mut cursor, |code| is_space(code) || code == unit(';'));
        if cursor.at >= cursor.text.len() {
            return true;
        }
        if !read_command(&mut cursor) {
            return false;
        }
    }
}

/// Options that change nothing but how the operands are read and written (`.` stops at a line
/// terminator, as JavaScript's does without the `s` flag).
static PLAIN_OPTION: Pattern = LazyLock::new(|| {
    Regex::new(r"^(?:-[nrEsuz]*(?:i[^\n\r\u{2028}\u{2029}]*)?|--(?:in-place(?:=[^\n\r\u{2028}\u{2029}]*)?|quiet|silent|regexp-extended|separate|null-data|posix|debug|unbuffered|follow-symlinks|sandbox))$").ok()
});
static EXPRESSION: Pattern =
    LazyLock::new(|| Regex::new(r"(?s)^(?:-[nrEsuz]*e(.*)|--expression(?:=(.*))?)$").ok());

struct SedCall {
    scripts: Vec<ShellWord>,
    operands: Vec<ShellWord>,
    safe: bool,
}

/// One option of a sed call: a plain one, an `-e` script (here or in the next word), or an unsafe one.
fn read_option(words: &[ShellWord], at: usize, call: &mut SedCall) -> usize {
    let value = words.get(at).map_or("", |word| word.value.as_str());
    let expression = EXPRESSION
        .as_ref()
        .and_then(|pattern| pattern.captures(value));
    let Some(expression) = expression else {
        call.safe = call.safe && regex_test(&PLAIN_OPTION, value);
        return at;
    };
    let attached = expression.get(1).or_else(|| expression.get(2));
    let word = words.get(at);
    if let (Some(attached), Some(word)) = (attached, word)
        && !attached.as_str().is_empty()
    {
        call.scripts.push(ShellWord {
            value: attached.as_str().to_owned(),
            ..word.clone()
        });
        return at;
    }
    match words.get(at + 1) {
        None => call.safe = false,
        Some(next) => call.scripts.push(next.clone()),
    }
    at + 1
}

/// The files a sed call edits, or `undefined` when its script does more than edit them (a script
/// a variable hides counts as unsafe).
#[must_use]
pub fn sed_operands(args: &[ShellWord]) -> Option<Vec<ShellWord>> {
    let mut call = SedCall {
        scripts: Vec::new(),
        operands: Vec::new(),
        safe: true,
    };
    let mut options = true;
    let mut at = 0;
    while at < args.len() {
        if let Some(word) = args.get(at) {
            if options && word.value == "--" {
                options = false;
            } else if options && word.value.starts_with('-') {
                at = read_option(args, at, &mut call);
            } else {
                call.operands.push(word.clone());
            }
        }
        at += 1;
    }
    edited_files(&call)
}

/// The operands left once the script is known safe: with no `-e`, the first operand is the script.
fn edited_files(call: &SedCall) -> Option<Vec<ShellWord>> {
    let inline = call.scripts.is_empty();
    let scripts = if inline {
        call.operands.get(..1).unwrap_or_default()
    } else {
        &call.scripts
    };
    let safe = call.safe
        && !scripts.is_empty()
        && scripts
            .iter()
            .all(|script| !script.expands && is_safe_sed_script(&script.value));
    safe.then(|| {
        call.operands
            .get(usize::from(inline)..)
            .unwrap_or_default()
            .to_vec()
    })
}

#[cfg(test)]
mod tests;
