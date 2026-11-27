//! The lexer's reading helpers: a line, a look ahead, and the `push*` steps that emit a slice of
//! the buffer as a token and move past it.

use super::{Lexeme, Lexer, Line, is_empty, is_flow_indicator, is_not_anchor_char};
use crate::yaml::units::{at, index_of, is, signed, string, unit, unsigned};

/// `tagChars`: what a tag may hold besides `%` escapes.
const TAG_CHARS: &str =
    "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-#;/?:@&=+$_.!~*'()";

fn is_hex_digit(character: Option<u16>) -> bool {
    character.is_some_and(|found| u8::try_from(found).is_ok_and(|byte| byte.is_ascii_hexdigit()))
}

fn is_tag_char(character: u16) -> bool {
    TAG_CHARS.encode_utf16().any(|known| known == character)
}

impl Lexer<'_> {
    /// Whether the rest of the line is blank or a comment.
    pub(super) fn at_line_end(&self) -> bool {
        let mut index = self.pos;
        let mut character = at(self.buffer, index);
        while character == Some(unit(' ')) || character == Some(unit('\t')) {
            index += 1;
            character = at(self.buffer, index);
        }
        match character {
            None => true,
            Some(found) if found == unit('#') || found == unit('\n') => true,
            Some(found) if found == unit('\r') => is(self.buffer, index + 1, '\n'),
            Some(_) => false,
        }
    }

    /// Where a scalar continues on the line starting at `offset`, or -1 when it ends before it.
    pub(super) fn continue_scalar(&self, offset: isize) -> isize {
        let mut character = at(self.buffer, offset);
        if self.indent_next > 0 {
            let mut indent = 0;
            while character == Some(unit(' ')) {
                indent += 1;
                character = at(self.buffer, indent + offset);
            }
            if character == Some(unit('\r')) && is(self.buffer, indent + offset + 1, '\n') {
                return offset + indent + 1;
            }
            return if character == Some(unit('\n')) || indent >= self.indent_next {
                offset + indent
            } else {
                -1
            };
        }
        if character == Some(unit('-')) || character == Some(unit('.')) {
            let marker = self.slice_text(offset, 3);
            if (marker == "---" || marker == "...") && is_empty(at(self.buffer, offset + 3)) {
                return -1;
            }
        }
        offset
    }

    /// The current line from the position, without its line break (`\n` or `\r\n`).
    pub(super) fn get_line(&mut self) -> Line {
        let mut end = match self.line_end_pos {
            Some(end) if end == -1 || end >= self.pos => end,
            Some(_) | None => {
                let end = index_of(self.buffer, '\n', self.pos);
                self.line_end_pos = Some(end);
                end
            }
        };
        if end == -1 {
            return Line {
                start: self.pos,
                end: signed(self.buffer.len()),
            };
        }
        if is(self.buffer, end - 1, '\r') {
            end -= 1;
        }
        // `substring` swaps reversed bounds: at a `\n` after a `\r` the line is that `\r`.
        Line {
            start: self.pos.min(end),
            end: end.max(self.pos),
        }
    }

    /// `buffer.substr(offset, length)` as text.
    fn slice_text(&self, offset: isize, length: isize) -> String {
        let start = unsigned(offset).min(self.buffer.len());
        let end = unsigned(offset + length).min(self.buffer.len());
        string(self.buffer.get(start..end).unwrap_or_default())
    }

    /// `peek(n)`: the next `n` code units as text.
    pub(super) fn peek(&self, n: isize) -> String {
        self.slice_text(self.pos, n)
    }

    /// Emits the next `n` code units; how many were asked for.
    pub(super) fn push_count(&mut self, n: isize) -> isize {
        if n > 0 {
            let start = unsigned(self.pos).min(self.buffer.len());
            let end = unsigned(self.pos + n).min(self.buffer.len());
            self.out.push(Lexeme::Text(start, end));
            self.pos += n;
            return n;
        }
        0
    }

    /// Emits the buffer up to `index`; an empty token only when `allow_empty`.
    pub(super) fn push_to_index(&mut self, index: isize, allow_empty: bool) -> isize {
        let start = unsigned(self.pos).min(self.buffer.len());
        let end = unsigned(index).min(self.buffer.len()).max(start);
        if end > start {
            self.out.push(Lexeme::Text(start, end));
            let length = signed(end - start);
            self.pos += length;
            return length;
        }
        if allow_empty {
            self.out.push(Lexeme::Text(start, start));
        }
        0
    }

    /// Tags, anchors and the indicators `-`, `?` and `:` with the spaces after each.
    pub(super) fn push_indicators(&mut self) -> isize {
        let mut n = 0;
        loop {
            let first = self.char_at(0);
            if first == Some(unit('!')) {
                n += self.push_tag();
                n += self.push_spaces(true);
            } else if first == Some(unit('&')) {
                n += self.push_until(is_not_anchor_char);
                n += self.push_spaces(true);
            } else if first == Some(unit('-'))
                || first == Some(unit('?'))
                || first == Some(unit(':'))
            {
                // `-` is an error here, and `?` outside flow collections.
                let in_flow = self.flow_level > 0;
                let second = self.char_at(1);
                if !(is_empty(second) || (in_flow && is_flow_indicator(second))) {
                    break;
                }
                if !in_flow {
                    self.indent_next = self.indent_value + 1;
                } else if self.flow_key {
                    self.flow_key = false;
                }
                n += self.push_count(1);
                n += self.push_spaces(true);
            } else {
                break;
            }
        }
        n
    }

    fn push_tag(&mut self) -> isize {
        if self.char_at(1) == Some(unit('<')) {
            let mut index = self.pos + 2;
            let mut character = at(self.buffer, index);
            while !is_empty(character) && character != Some(unit('>')) {
                index += 1;
                character = at(self.buffer, index);
            }
            let end = if character == Some(unit('>')) {
                index + 1
            } else {
                index
            };
            return self.push_to_index(end, false);
        }
        let mut index = self.pos + 1;
        while let Some(character) = at(self.buffer, index) {
            if is_tag_char(character) {
                index += 1;
            } else if character == unit('%')
                && is_hex_digit(at(self.buffer, index + 1))
                && is_hex_digit(at(self.buffer, index + 2))
            {
                index += 3;
            } else {
                break;
            }
        }
        self.push_to_index(index, false)
    }

    /// A line break, `\n` or `\r\n`.
    pub(super) fn push_newline(&mut self) -> isize {
        if self.char_at(0) == Some(unit('\n')) {
            self.push_count(1)
        } else if self.char_at(0) == Some(unit('\r')) && self.char_at(1) == Some(unit('\n')) {
            self.push_count(2)
        } else {
            0
        }
    }

    /// Spaces, and tabs too when `allow_tabs`.
    pub(super) fn push_spaces(&mut self, allow_tabs: bool) -> isize {
        let mut index = self.pos;
        loop {
            let character = at(self.buffer, index);
            if character == Some(unit(' ')) || (allow_tabs && character == Some(unit('\t'))) {
                index += 1;
            } else {
                break;
            }
        }
        let n = index - self.pos;
        if n > 0 {
            self.push_count(n);
        }
        n
    }

    /// Everything up to the first character `test` accepts (the end included).
    pub(super) fn push_until(&mut self, test: fn(Option<u16>) -> bool) -> isize {
        let mut index = self.pos;
        while !test(at(self.buffer, index)) {
            index += 1;
        }
        self.push_to_index(index, false)
    }
}
