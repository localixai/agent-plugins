//! Port of `src/format/frontmatter.ts`, switched.
//!
//! A Markdown file's YAML frontmatter and body, as skills.md and output-styles.md describe: the
//! opening `---` must be line 1, a block that does not parse leaves the file loaded with no
//! fields. The YAML is read as the npm package reads it with `yaml` (`crate::yaml`), its errors
//! worded and placed as there.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::format::frontmatter_values::NOT_A_TABLE;
use crate::js::{JsObject, JsValue, Pattern, code_unit_length, is_whitespace, regex_test};
use crate::protocol::is_record::is_record;
use crate::yaml::nodes::Document;
use crate::yaml::{parse_document, to_js};

/// Where the frontmatter stands:
///
/// - `none`: the file does not open with a `---` line, or no closing `---` follows; all of it is
///   body.
/// - `ok`: a YAML table (or an empty block) between the markers; `fields` holds it.
/// - `invalid-yaml`: the YAML between the markers does not parse; the file loads with no fields.
/// - `not-a-table`: it parses to a list or a single value; the file loads with no fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FrontmatterState {
    /// No block.
    None,
    /// A table, or an empty block.
    Ok,
    /// YAML that does not parse.
    InvalidYaml,
    /// A list or a single value.
    NotATable,
}

/// A file split into its frontmatter and body.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Frontmatter {
    /// Where the frontmatter stands.
    pub state: FrontmatterState,
    /// `state` is not `none`: the file has a frontmatter block, valid or not.
    pub present: bool,
    /// Always a plain object: empty unless `state` is `ok`.
    pub fields: JsObject,
    /// The text after the closing marker, or the whole file when `state` is `none`.
    pub body: String,
    /// Why `state` is `invalid-yaml` or `not-a-table`, as the YAML parser or `readTable` words
    /// it, for a notice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
    /// Where the problem is in the file, counted from 1 (the opening `---` is line 1): the first
    /// YAML line is 2. Present when `problem` is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// The problem's column, counted from 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    /// What the block parsed to when `state` is `not-a-table` (a list or a single value).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<JsValue>,
}

/// A place in the file.
#[derive(Clone, Copy, Debug)]
struct Where {
    line: usize,
    column: usize,
}

/// The first line after the opening marker.
const BLOCK_START: Where = Where { line: 2, column: 1 };

static MARKER: Pattern = LazyLock::new(|| Regex::new(r"^---[ \t]*$").ok());
const BOM: char = '\u{FEFF}';

fn is_marker(line: &str) -> bool {
    regex_test(&MARKER, line.strip_suffix('\r').unwrap_or(line))
}

fn none(text: &str) -> Frontmatter {
    Frontmatter {
        state: FrontmatterState::None,
        present: false,
        fields: JsObject::new(),
        body: text.to_owned(),
        problem: None,
        line: None,
        column: None,
        value: None,
    }
}

fn failed(
    state: FrontmatterState,
    body: &str,
    problem: &str,
    Where { line, column }: Where,
) -> Frontmatter {
    Frontmatter {
        state,
        present: true,
        fields: JsObject::new(),
        body: body.to_owned(),
        problem: Some(problem.to_owned()),
        line: Some(line),
        column: Some(column),
        value: None,
    }
}

// `YAML_OPTIONS` (`{ prettyErrors: false, uniqueKeys: false, logLevel: 'error' }`) are the only
// options `crate::yaml::parse_document` reads with: `logLevel: 'error'` keeps the parser's
// warnings (a key that is a collection) off the host's `process.emitWarning`, and the key is read
// as the text it is either way (judgment).

fn relaxed(yaml: &str, strict: Document) -> Document {
    let quoted = quote_plain_scalars(yaml);
    if quoted == yaml {
        return strict;
    }
    let (retry, _) = parse_document(&quoted);
    if retry.errors.is_empty() {
        retry
    } else {
        strict
    }
}

/// One line of `text` from `at`, and where the next line starts (past the end on the last line).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LineAt {
    /// The line, without its `\n`.
    pub line: String,
    /// Where the next line starts, in UTF-16 code units.
    pub next: usize,
}

/// One line of `text` from `at` (UTF-16 code units, as JavaScript counts), and where the next
/// line starts (past the end on the last line).
#[must_use]
pub fn line_at(text: &str, at: usize) -> LineAt {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (line, next) = line_at_units(&units, at);
    LineAt {
        line: String::from_utf16_lossy(line),
        next,
    }
}

/// [`line_at`] over the text's UTF-16 code units.
fn line_at_units(units: &[u16], at: usize) -> (&[u16], usize) {
    let rest = units.get(at..).unwrap_or_default();
    match rest.iter().position(|unit| *unit == u16::from(b'\n')) {
        Some(offset) => (rest.get(..offset).unwrap_or_default(), at + offset + 1),
        None => (rest, units.len() + 1),
    }
}

/// The YAML text between the markers and the body after them, or `None` without a block. Reads
/// line by line up to the closing marker only, so a long body is never split into lines.
fn split_block(text: &str) -> Option<(String, String)> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let (opening, opening_next) = line_at_units(&units, 0);
    if !is_marker(&String::from_utf16_lossy(opening)) {
        return None;
    }
    let mut at = opening_next;
    while at <= units.len() {
        let (line, next) = line_at_units(&units, at);
        if is_marker(&String::from_utf16_lossy(line)) {
            let end = opening_next.max(at.saturating_sub(1));
            let yaml = String::from_utf16_lossy(units.get(opening_next..end).unwrap_or_default());
            let body = String::from_utf16_lossy(units.get(next..).unwrap_or_default());
            return Some((drop_carriage_returns(&yaml), body));
        }
        at = next;
    }
    None
}

/// `yaml.replace(/\r(?=\n|$)/g, '')`.
fn drop_carriage_returns(yaml: &str) -> String {
    let mut out = String::with_capacity(yaml.len());
    let mut chars = yaml.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\r' || !matches!(chars.peek(), None | Some('\n')) {
            out.push(character);
        }
    }
    out
}

// Claude Code retries a block its YAML parser refuses with the value of each top-level
// `key: value` line quoted when it holds a character plain YAML cannot (oracle 2.1.319:
// `description: Does X: then Y`, `argument-hint: "<a>" [--b]`, `tools: *`, `metadata: {"a": 1}`
// read as the text written). What the retry does not touch stays an error: an indented line, a
// value already wrapped in one kind of quote (`"a" or "b"`), a value with no such character
// (`"a" b`), and a block scalar marker, which the retry does quote and so breaks the lines that
// follow it.
//
// `\w` is ASCII, `\S` is not one of JavaScript's blanks, `.` is not a line terminator (RS6).
static TOP_LEVEL: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^([A-Za-z_][A-Za-z0-9_-]*:[ \t]+)([^\t\n\x0B\x0C\r \u{A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}][^\n\r\u{2028}\u{2029}]*)$",
    )
    .ok()
});
static SPECIAL: Pattern = LazyLock::new(|| Regex::new(r"[{}\[\]*&#!|>%@`]|: ").ok());

/// A value that opens and closes with one kind of quote: the retry takes it as quoted already.
fn wrapped(value: &str) -> bool {
    code_unit_length(value) > 1
        && value
            .chars()
            .next()
            .is_some_and(|first| (first == '"' || first == '\'') && value.ends_with(first))
}

fn quote_refused(line: &str) -> String {
    let Some(captures) = TOP_LEVEL
        .as_ref()
        .and_then(|pattern| pattern.captures(line))
    else {
        return line.to_owned();
    };
    let (Some(key), Some(value)) = (captures.get(1), captures.get(2)) else {
        return line.to_owned();
    };
    let value = value.as_str().trim_end_matches(is_whitespace);
    if wrapped(value) || !regex_test(&SPECIAL, value) {
        return line.to_owned();
    }
    format!("{}'{}'", key.as_str(), value.replace('\'', "''"))
}

fn quote_plain_scalars(yaml: &str) -> String {
    yaml.split('\n')
        .map(quote_refused)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The block's YAML document as [`parse_frontmatter`] reads it (the relaxed retry applied), the
/// body after it, and where the strict read's lines start.
#[derive(Clone, Debug)]
pub(crate) struct FrontmatterDocument {
    pub(crate) body: String,
    pub(crate) document: Document,
    line_starts: Vec<usize>,
}

/// The block's YAML document as [`parse_frontmatter`] reads it (the relaxed retry applied) and the
/// body after it; `None` when the file has no block. The writer edits this document, so a file is
/// edited as it is read.
pub(crate) fn frontmatter_document(text: &str) -> Option<FrontmatterDocument> {
    let source = text.strip_prefix(BOM).unwrap_or(text);
    let (yaml, body) = split_block(source)?;
    let (strict, line_starts) = parse_document(&yaml);
    let document = if strict.errors.is_empty() {
        strict
    } else {
        relaxed(&yaml, strict)
    };
    Some(FrontmatterDocument {
        body,
        document,
        line_starts,
    })
}

/// `lineCounter.linePos(offset)`: the line (counted from 1) and column of an offset.
fn line_pos(line_starts: &[usize], offset: usize) -> (usize, usize) {
    let low = line_starts.partition_point(|start| *start < offset);
    if line_starts.get(low) == Some(&offset) {
        return (low + 1, 1);
    }
    match low
        .checked_sub(1)
        .and_then(|previous| line_starts.get(previous))
    {
        Some(start) => (low, offset - start + 1),
        None => (0, offset),
    }
}

/// Split a Markdown file into its YAML frontmatter and body, as skills.md and output-styles.md
/// describe: the opening `---` must be line 1, a block that does not parse leaves the file loaded
/// with no fields. Never panics; CRLF files read like LF ones.
#[must_use]
pub fn parse_frontmatter(text: &str) -> Frontmatter {
    let Some(read) = frontmatter_document(text) else {
        return none(text);
    };
    let FrontmatterDocument {
        body,
        document,
        line_starts,
    } = read;
    if let Some(first) = document.errors.first() {
        let (line, column) = line_pos(&line_starts, first.offset);
        let place = Where {
            line: line + 1,
            column,
        };
        return failed(FrontmatterState::InvalidYaml, &body, &first.message, place);
    }
    match to_js(&document) {
        Ok(JsValue::Null | JsValue::Undefined) => Frontmatter {
            state: FrontmatterState::Ok,
            present: true,
            fields: JsObject::new(),
            body,
            problem: None,
            line: None,
            column: None,
            value: None,
        },
        Ok(value) if !is_record(&value) => Frontmatter {
            value: Some(value),
            ..failed(FrontmatterState::NotATable, &body, NOT_A_TABLE, BLOCK_START)
        },
        Ok(value) => Frontmatter {
            state: FrontmatterState::Ok,
            present: true,
            fields: match value {
                JsValue::Object(fields) => fields,
                JsValue::Undefined
                | JsValue::Null
                | JsValue::Bool(_)
                | JsValue::Number(_)
                | JsValue::String(_)
                | JsValue::Array(_) => JsObject::new(),
            },
            body,
            problem: None,
            line: None,
            column: None,
            value: None,
        },
        // `toJS` refuses an alias bomb by throwing: that is a document that does not parse.
        Err(message) => failed(FrontmatterState::InvalidYaml, &body, &message, BLOCK_START),
    }
}

crate::registry::exports! {
    "format/frontmatter" {
        "lineAt" => ok |text: String, at: usize| line_at(&text, at);
        "parseFrontmatter" => ok |text: String| parse_frontmatter(&text);
    }
}
