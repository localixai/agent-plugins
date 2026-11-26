//! The lexer of `yaml` (`dist/parse/lexer.js`): splits a source into lexical tokens, slices of the
//! source and three control tokens. Its states and their order are the package's:
//!
//! ```text
//! stream: directive -> line-end -> stream | indent + line-end -> stream | [else] -> line-start
//! line-start: doc-start -> doc | doc-end -> stream | [else] -> indent -> block-start
//! block-start: seq-item-start, explicit-key-start or map-value-start -> block-start | [else] -> doc
//! doc: line-end -> line-start | flow-start -> flow -> doc | quote-start -> quoted-scalar -> doc
//!      | block-scalar-header -> line-end -> block-scalar(min) -> line-start | [else] -> plain-scalar
//! flow: line-end, spaces, props, comma -> flow | flow-end -> . | [else] -> plain-scalar(true, 0)
//! ```
//!
//! The whole source is lexed at once (`parseDocument` passes it complete), so the package's
//! buffering of incomplete input never applies.

mod push;
mod scalars;

use super::units::{Units, at, is, signed, unit};

/// One lexical token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Lexeme {
    /// `\x02` (Start of Text): a document starts with the next token.
    Document,
    /// `\x18` (Cancel): an unexpected end of flow mode (an error).
    FlowEnd,
    /// `\x1f` (Unit Separator): the next token is a scalar value.
    Scalar,
    /// A slice of the source, `start..end` in code units.
    Text(usize, usize),
}

/// The states of the lexer. The package's `line-start` is a state only while the input is
/// incomplete (`setNext`); the whole text is lexed at once here, so it never is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Stream,
    BlockStart,
    Doc,
    Flow,
}

/// `isEmpty`: the end of the source, a space, a line break or a tab.
pub(crate) fn is_empty(character: Option<u16>) -> bool {
    character.is_none_or(|found| [unit(' '), unit('\n'), unit('\r'), unit('\t')].contains(&found))
}

/// `flowIndicatorChars`.
fn is_flow_indicator(character: Option<u16>) -> bool {
    character.is_some_and(|found| ",[]{}".encode_utf16().any(|known| known == found))
}

/// `isNotAnchorChar`: what ends an anchor or an alias.
fn is_not_anchor_char(character: Option<u16>) -> bool {
    character.is_none_or(|found| " ,[]{}\n\r\t".encode_utf16().any(|known| known == found))
}

/// One line of the buffer: `start..end`, without its line break.
#[derive(Debug, Clone, Copy)]
struct Line {
    start: isize,
    end: isize,
}

/// The lexer's state (the package's fields, but `atEnd`, always true here).
#[derive(Debug)]
struct Lexer<'a> {
    buffer: &'a [u16],
    /// Explicit indent set in a block scalar header, as an offset from the current minimum
    /// indent (`|2+` sets 1); -1 when not set.
    block_scalar_indent: isize,
    /// The block scalar header holds `+` (keep): trailing empty lines are part of the scalar.
    block_scalar_keep: bool,
    /// The map value indicator `:` may immediately follow this node in a flow context.
    flow_key: bool,
    /// How many flow collections surround the position.
    flow_level: usize,
    /// The least indentation the next lines need to be part of the current scalar.
    indent_next: isize,
    /// The indentation of the current line.
    indent_value: isize,
    /// Where the line break after the position is (-1: none), once `get_line` looked: a long
    /// line is searched once, not once per token on it.
    line_end_pos: Option<isize>,
    /// The position in `buffer`.
    pos: isize,
    out: Vec<Lexeme>,
}

/// The lexical tokens of a whole source.
pub(crate) fn lex(source: &Units) -> Vec<Lexeme> {
    let mut lexer = Lexer {
        buffer: source,
        block_scalar_indent: -1,
        block_scalar_keep: false,
        flow_key: false,
        flow_level: 0,
        indent_next: 0,
        indent_value: 0,
        line_end_pos: None,
        pos: 0,
        out: Vec::new(),
    };
    let mut next = State::Stream;
    while lexer.has_chars(1) {
        next = lexer.parse_next(next);
    }
    lexer.out
}

impl Lexer<'_> {
    fn char_at(&self, n: isize) -> Option<u16> {
        at(self.buffer, self.pos + n)
    }

    fn has_chars(&self, n: isize) -> bool {
        self.pos + n <= signed(self.buffer.len())
    }

    /// `line[n]` of a line read by `get_line`.
    fn line_at(&self, line: Line, n: isize) -> Option<u16> {
        if line.start + n < line.end {
            at(self.buffer, line.start + n)
        } else {
            None
        }
    }

    fn parse_next(&mut self, next: State) -> State {
        match next {
            State::Stream => self.parse_stream(),
            State::BlockStart => self.parse_block_start(),
            State::Doc => self.parse_document(),
            State::Flow => self.parse_flow_collection(),
        }
    }

    fn parse_stream(&mut self) -> State {
        let mut line = self.get_line();
        if self.line_at(line, 0) == Some(0xFEFF) {
            self.push_count(1);
            line.start += 1;
        }
        if self.line_at(line, 0) == Some(unit('%')) {
            self.push_directive(line);
            return State::Stream;
        }
        if self.at_line_end() {
            let spaces = self.push_spaces(true);
            self.push_count(line.end - line.start - spaces);
            self.push_newline();
            return State::Stream;
        }
        self.out.push(Lexeme::Document);
        self.parse_line_start()
    }

    /// A `%` directive line: its text, the spaces after it, then a possible comment. The
    /// package's `this.pushNewline()` there is a generator never run, so the line break is left
    /// for the next `stream` state.
    fn push_directive(&mut self, line: Line) {
        let length = line.end - line.start;
        let mut directive_end = length;
        let mut comment = self.line_index_of(line, '#', 0);
        while comment != -1 {
            let before = self.line_at(line, comment - 1);
            if before == Some(unit(' ')) || before == Some(unit('\t')) {
                directive_end = comment - 1;
                break;
            }
            comment = self.line_index_of(line, '#', comment + 1);
        }
        loop {
            let before = self.line_at(line, directive_end - 1);
            if before == Some(unit(' ')) || before == Some(unit('\t')) {
                directive_end -= 1;
            } else {
                break;
            }
        }
        let n = self.push_count(directive_end) + self.push_spaces(true);
        self.push_count(length - n);
    }

    fn line_index_of(&self, line: Line, character: char, from: isize) -> isize {
        let mut index = from.max(0);
        while line.start + index < line.end {
            if is(self.buffer, line.start + index, character) {
                return index;
            }
            index += 1;
        }
        -1
    }

    fn parse_line_start(&mut self) -> State {
        let first = self.char_at(0);
        if first == Some(unit('-')) || first == Some(unit('.')) {
            let marker = self.peek(3);
            if (marker == "---" || marker == "...") && is_empty(self.char_at(3)) {
                self.push_count(3);
                self.indent_value = 0;
                self.indent_next = 0;
                return if marker == "---" {
                    State::Doc
                } else {
                    State::Stream
                };
            }
        }
        self.indent_value = self.push_spaces(false);
        if self.indent_next > self.indent_value && !is_empty(self.char_at(1)) {
            self.indent_next = self.indent_value;
        }
        self.parse_block_start()
    }

    fn parse_block_start(&mut self) -> State {
        let first = self.char_at(0);
        let indicator = [unit('-'), unit('?'), unit(':')];
        if first.is_some_and(|found| indicator.contains(&found)) && is_empty(self.char_at(1)) {
            let n = self.push_count(1) + self.push_spaces(true);
            self.indent_next = self.indent_value + 1;
            self.indent_value += n;
            return State::BlockStart;
        }
        State::Doc
    }

    fn parse_document(&mut self) -> State {
        self.push_spaces(true);
        let line = self.get_line();
        let mut n = self.push_indicators();
        let Some(next) = self.line_at(line, n) else {
            self.push_newline();
            return self.parse_line_start();
        };
        match char::from_u32(u32::from(next)).unwrap_or('\0') {
            '#' => {
                self.push_count(line.end - line.start - n);
                self.push_newline();
                self.parse_line_start()
            }
            '{' | '[' => {
                self.push_count(1);
                self.flow_key = false;
                self.flow_level = 1;
                State::Flow
            }
            // An error.
            '}' | ']' => {
                self.push_count(1);
                State::Doc
            }
            '*' => {
                self.push_until(is_not_anchor_char);
                State::Doc
            }
            '"' | '\'' => self.parse_quoted_scalar(),
            '|' | '>' => {
                n += self.parse_block_scalar_header();
                n += self.push_spaces(true);
                self.push_count(line.end - line.start - n);
                self.push_newline();
                self.parse_block_scalar()
            }
            _ => self.parse_plain_scalar(),
        }
    }

    fn parse_flow_collection(&mut self) -> State {
        let mut indent = -1;
        loop {
            let newline = self.push_newline();
            let mut spaces = 0;
            if newline > 0 {
                spaces = self.push_spaces(false);
                self.indent_value = spaces;
                indent = spaces;
            }
            spaces += self.push_spaces(true);
            if newline + spaces == 0 {
                break;
            }
        }
        let line = self.get_line();
        let first = self.line_at(line, 0);
        let at_marker = indent == 0
            && (self.line_starts_with(line, "---") || self.line_starts_with(line, "..."))
            && is_empty(self.line_at(line, 3));
        if (indent != -1 && indent < self.indent_next && first != Some(unit('#'))) || at_marker {
            // A terminal `]` or `}` at the indent of the opening one is technically invalid, but
            // failing there would surprise users.
            let at_flow_end_marker = indent == self.indent_next - 1
                && self.flow_level == 1
                && (first == Some(unit(']')) || first == Some(unit('}')));
            if !at_flow_end_marker {
                // An error.
                self.flow_level = 0;
                self.out.push(Lexeme::FlowEnd);
                return self.parse_line_start();
            }
        }
        let mut n = 0;
        while self.line_at(line, n) == Some(unit(',')) {
            n += self.push_count(1);
            n += self.push_spaces(true);
            self.flow_key = false;
        }
        n += self.push_indicators();
        self.flow_item(line, n)
    }

    fn line_starts_with(&self, line: Line, prefix: &str) -> bool {
        prefix
            .encode_utf16()
            .enumerate()
            .all(|(index, expected)| self.line_at(line, signed(index)) == Some(expected))
    }

    /// The token at `line[n]` inside a flow collection.
    fn flow_item(&mut self, line: Line, n: isize) -> State {
        let Some(next) = self.line_at(line, n) else {
            return State::Flow;
        };
        match char::from_u32(u32::from(next)).unwrap_or('\0') {
            '#' => {
                self.push_count(line.end - line.start - n);
                State::Flow
            }
            '{' | '[' => {
                self.push_count(1);
                self.flow_key = false;
                self.flow_level += 1;
                State::Flow
            }
            '}' | ']' => {
                self.push_count(1);
                self.flow_key = true;
                self.flow_level = self.flow_level.saturating_sub(1);
                if self.flow_level > 0 {
                    State::Flow
                } else {
                    State::Doc
                }
            }
            '*' => {
                self.push_until(is_not_anchor_char);
                State::Flow
            }
            '"' | '\'' => {
                self.flow_key = true;
                self.parse_quoted_scalar()
            }
            ':' if self.flow_key
                || is_empty(self.char_at(1))
                || self.char_at(1) == Some(unit(',')) =>
            {
                self.flow_key = false;
                self.push_count(1);
                self.push_spaces(true);
                State::Flow
            }
            _ => {
                self.flow_key = false;
                self.parse_plain_scalar()
            }
        }
    }
}
