//! The lexer's scalars: quoted, block and plain, each emitted whole as one token.

use super::{Lexeme, Lexer, State, is_empty, is_flow_indicator};
use crate::yaml::units::{at, index_of, is, signed, unit};

impl Lexer<'_> {
    pub(super) fn parse_quoted_scalar(&mut self) -> State {
        let Some(quote) = self.char_at(0) else {
            return State::Doc;
        };
        let mut end = self.quote_end(quote);
        // Only newlines inside the quotes count: `substring(0, -1)` is empty.
        let inside = usize::try_from(end).map_or(0, |end| end.min(self.buffer.len()));
        let quoted = self.buffer.get(..inside).unwrap_or_default();
        let mut newline = index_of(quoted, '\n', self.pos);
        if newline != -1 {
            while newline != -1 {
                let continues = self.continue_scalar(newline + 1);
                if continues == -1 {
                    break;
                }
                newline = index_of(quoted, '\n', continues);
            }
            if newline != -1 {
                // An error caused by an unexpected unindent.
                end = newline - if is(quoted, newline - 1, '\r') { 2 } else { 1 };
            }
        }
        if end == -1 {
            end = signed(self.buffer.len());
        }
        self.push_to_index(end + 1, false);
        if self.flow_level > 0 {
            State::Flow
        } else {
            State::Doc
        }
    }

    /// Where the closing quote is, -1 when there is none: `''` is an escaped single quote, and a
    /// `"` after an odd run of backslashes an escaped double one.
    fn quote_end(&self, quote: u16) -> isize {
        let single = quote == unit('\'');
        let mark = if single { '\'' } else { '"' };
        let mut end = index_of(self.buffer, mark, self.pos + 1);
        if single {
            while end != -1 && is(self.buffer, end + 1, '\'') {
                end = index_of(self.buffer, '\'', end + 2);
            }
            return end;
        }
        while end != -1 {
            let mut n = 0;
            while is(self.buffer, end - 1 - n, '\\') {
                n += 1;
            }
            if n % 2 == 0 {
                break;
            }
            end = index_of(self.buffer, '"', end + 1);
        }
        end
    }

    pub(super) fn parse_block_scalar_header(&mut self) -> isize {
        self.block_scalar_indent = -1;
        self.block_scalar_keep = false;
        let mut index = self.pos;
        loop {
            index += 1;
            let character = at(self.buffer, index);
            if character == Some(unit('+')) {
                self.block_scalar_keep = true;
            } else if let Some(digit) =
                character.filter(|&found| found > unit('0') && found <= unit('9'))
            {
                self.block_scalar_indent = isize::try_from(digit - unit('0')).unwrap_or(0) - 1;
            } else if character != Some(unit('-')) {
                break;
            }
        }
        self.push_until(|character| is_empty(character) || character == Some(unit('#')))
    }

    pub(super) fn parse_block_scalar(&mut self) -> State {
        // May be -1 when the position is 0.
        let mut newline = self.pos - 1;
        let mut indent = 0;
        let mut index = self.pos;
        while let Some(character) = at(self.buffer, index) {
            if character == unit(' ') {
                indent += 1;
            } else if character == unit('\n') {
                newline = index;
                indent = 0;
            } else if !(character == unit('\r') && is(self.buffer, index + 1, '\n')) {
                break;
            }
            index += 1;
        }
        if indent >= self.indent_next {
            if self.block_scalar_indent == -1 {
                self.indent_next = indent;
            } else {
                self.indent_next = self.block_scalar_indent
                    + if self.indent_next == 0 {
                        1
                    } else {
                        self.indent_next
                    };
            }
            loop {
                let continues = self.continue_scalar(newline + 1);
                if continues == -1 {
                    break;
                }
                newline = index_of(self.buffer, '\n', continues);
                if newline == -1 {
                    break;
                }
            }
            if newline == -1 {
                newline = signed(self.buffer.len());
            }
        }
        newline = self.block_scalar_end(newline, indent);
        self.out.push(Lexeme::Scalar);
        self.push_to_index(newline + 1, true);
        self.parse_line_start()
    }

    /// Where the block scalar's text ends: trailing insufficiently indented tabs are invalid, so
    /// they are taken into the value for the composer to catch; without `+`, trailing lines that
    /// are not more indented are left out.
    fn block_scalar_end(&self, mut newline: isize, indent: isize) -> isize {
        let mut index = newline + 1;
        let mut character = at(self.buffer, index);
        while character == Some(unit(' ')) {
            index += 1;
            character = at(self.buffer, index);
        }
        if character == Some(unit('\t')) {
            while character.is_some_and(|found| {
                [unit('\t'), unit(' '), unit('\r'), unit('\n')].contains(&found)
            }) {
                index += 1;
                character = at(self.buffer, index);
            }
            return index - 1;
        }
        if self.block_scalar_keep {
            return newline;
        }
        loop {
            let mut index = newline - 1;
            let mut character = at(self.buffer, index);
            if character == Some(unit('\r')) {
                index -= 1;
                character = at(self.buffer, index);
            }
            // Drop the line if its last character is not more indented.
            let last_char = index;
            while character == Some(unit(' ')) {
                index -= 1;
                character = at(self.buffer, index);
            }
            if character == Some(unit('\n')) && index >= self.pos && index + 1 + indent > last_char
            {
                newline = index;
            } else {
                break;
            }
        }
        newline
    }

    pub(super) fn parse_plain_scalar(&mut self) -> State {
        let in_flow = self.flow_level > 0;
        let mut end = self.pos - 1;
        let mut index = self.pos - 1;
        loop {
            index += 1;
            let Some(mut character) = at(self.buffer, index) else {
                break;
            };
            if character == unit(':') {
                let next = at(self.buffer, index + 1);
                if is_empty(next) || (in_flow && is_flow_indicator(next)) {
                    break;
                }
                end = index;
            } else if is_empty(Some(character)) {
                let mut next = at(self.buffer, index + 1);
                if character == unit('\r') {
                    if next == Some(unit('\n')) {
                        index += 1;
                        character = unit('\n');
                        next = at(self.buffer, index + 1);
                    } else {
                        end = index;
                    }
                }
                if next == Some(unit('#')) || (in_flow && is_flow_indicator(next)) {
                    break;
                }
                if character == unit('\n') {
                    let continues = self.continue_scalar(index + 1);
                    if continues == -1 {
                        break;
                    }
                    // To advance, but still account for ` #`.
                    index = index.max(continues - 2);
                }
            } else {
                if in_flow && is_flow_indicator(Some(character)) {
                    break;
                }
                end = index;
            }
        }
        self.out.push(Lexeme::Scalar);
        self.push_to_index(end + 1, true);
        if in_flow { State::Flow } else { State::Doc }
    }
}
