//! The scanner's strings and templates, from `src/format/js-tokens.ts`: quoted strings, their
//! escapes, and templates with the tokens of each `${...}` put right after the template's own.

use super::patterns::{hex_value, hex2_at, is, unicode_at};
use super::{JsToken, MAX_NESTING, STACK_EXCEEDED, Scanner, TokenForm};
use crate::js::utf16::{from_code_point, slice, text_of};

const SIMPLE_ESCAPES: [(char, char); 7] = [
    ('n', '\n'),
    ('t', '\t'),
    ('r', '\r'),
    ('b', '\u{8}'),
    ('f', '\u{C}'),
    ('v', '\u{B}'),
    ('0', '\0'),
];

impl Scanner<'_> {
    pub(super) fn quoted(&mut self, quote: Option<u16>) {
        let start = self.at;
        self.at += 1;
        let mut value: Vec<u16> = Vec::new();
        while self.at < self.text.len() {
            let char = self.text.get(self.at).copied();
            self.at += 1;
            if char == quote {
                let text = text_of(slice(self.text, start, self.at));
                let value = Some(text_of(&value));
                return self.push(JsToken {
                    form: TokenForm::String,
                    text,
                    start,
                    end: self.at,
                    value,
                });
            }
            if is(char, '\n') {
                break;
            }
            if is(char, '\\') {
                value.extend(self.escape());
            } else {
                value.extend(char);
            }
        }
        self.fail("unterminated string");
    }

    /// The character an escape stands for; `self.at` is just after the backslash.
    fn escape(&mut self) -> Vec<u16> {
        let char = self.text.get(self.at).copied();
        self.at += 1;
        let simple = SIMPLE_ESCAPES.iter().find(|(name, _)| is(char, *name));
        if let Some((_, stands_for)) = simple {
            return vec![u16::try_from(u32::from(*stands_for)).unwrap_or_default()];
        }
        if is(char, 'x') {
            return self.numeric();
        }
        if is(char, 'u') {
            return self.unicode();
        }
        if is(char, '\r') && is(self.text.get(self.at).copied(), '\n') {
            self.at += 1;
        }
        if is(char, '\n') || is(char, '\r') {
            Vec::new()
        } else {
            char.into_iter().collect()
        }
    }

    /// `\xHH`, or `x` when two hexadecimal digits do not follow.
    fn numeric(&mut self) -> Vec<u16> {
        let Some(length) = hex2_at(self.text, self.at) else {
            return vec![u16::from(b'x')];
        };
        let code = hex_value(slice(self.text, self.at, self.at + length));
        self.at += length;
        from_code_point(u32::try_from(code).unwrap_or_default())
    }

    fn unicode(&mut self) -> Vec<u16> {
        let Some((length, digits)) = unicode_at(self.text, self.at) else {
            return vec![u16::from(b'u')];
        };
        let code = hex_value(digits);
        self.at += length;
        if code > 0x10_FFFF {
            Vec::new()
        } else {
            from_code_point(u32::try_from(code).unwrap_or_default())
        }
    }

    pub(super) fn template(&mut self) {
        let start = self.at;
        self.at += 1;
        let index = self.tokens.len();
        let mut value: Option<Vec<u16>> = Some(Vec::new());
        let mut inner: Vec<JsToken> = Vec::new();
        while self.at < self.text.len() && self.problem.is_none() {
            let char = self.text.get(self.at).copied();
            self.at += 1;
            if is(char, '`') {
                return self.close_template(start, index, value, inner);
            }
            if is(char, '$') && is(self.text.get(self.at).copied(), '{') {
                value = None;
                self.at += 1;
                inner.extend(self.interpolation());
            } else if value.is_some() {
                let read = if is(char, '\\') {
                    self.escape()
                } else {
                    char.into_iter().collect()
                };
                value.iter_mut().for_each(|value| value.extend(&read));
            } else if is(char, '\\') {
                self.escape();
            }
        }
        self.fail("unterminated template");
    }

    /// The tokens of one `${...}`, scanned by a scanner of their own.
    fn interpolation(&mut self) -> Vec<JsToken> {
        if self.level >= MAX_NESTING {
            self.fail(&format!("code nested too deep to scan ({STACK_EXCEEDED})"));
            return Vec::new();
        }
        let mut inner = Scanner::new(self.text, self.level + 1);
        inner.at = self.at;
        inner.run(true, None);
        if self.problem.is_none() {
            self.problem = inner.problem;
        }
        self.at = inner.at + 1;
        inner.tokens
    }

    fn close_template(
        &mut self,
        start: usize,
        index: usize,
        value: Option<Vec<u16>>,
        inner: Vec<JsToken>,
    ) {
        let token = JsToken {
            form: TokenForm::String,
            text: text_of(slice(self.text, start, self.at)),
            start,
            end: self.at,
            value: value.map(|value| text_of(&value)),
        };
        let at = index.min(self.tokens.len());
        self.tokens
            .splice(at..at, std::iter::once(token.clone()).chain(inner));
        self.previous = Some(token);
    }
}
