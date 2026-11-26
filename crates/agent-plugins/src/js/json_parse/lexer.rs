//! The tokens of a JSON text as JavaScriptCore's JSON lexer reads them: over UTF-16 code units,
//! one token ahead of the parser, keeping the message of the first token it could not read. That
//! message, when there is one, is the one `JSON.parse` throws, whatever the parser then says.

/// One token of the text.
#[derive(Clone, Debug, PartialEq)]
pub(super) enum Token {
    /// One of `[ ] { } ( ) , : . = ;`: a character JavaScript's grammar knows, of which JSON
    /// uses the first six and `:`.
    Punctuator(char),
    /// A string's code units, its escapes read.
    String(Vec<u16>),
    /// A number, read as a double.
    Number(f64),
    /// A run of `[A-Za-z0-9_$]`, and U+200C and U+200D after its first character, that starts
    /// with no digit and is no `true`, `false` or `null`.
    Identifier(Vec<u16>),
    True,
    False,
    Null,
    /// The end of the text.
    End,
    /// A token that could not be read; the lexer keeps why.
    Error,
}

/// JSON's four blanks, not JavaScript's whole `trim` set: a no-break space or a byte order mark
/// is an unrecognized token.
const fn is_blank(unit: u16) -> bool {
    matches!(unit, 0x20 | 0x09 | 0x0A | 0x0D)
}

/// The character of one code unit; a surrogate, which is half of one, reads as U+FFFD.
fn character(unit: u16) -> char {
    char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}')
}

/// Code units as text; a lone surrogate, which no Rust string holds, reads as U+FFFD.
pub(super) fn text(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}

/// Reads a JSON text one token at a time.
pub(super) struct Lexer {
    units: Vec<u16>,
    position: usize,
    current: Token,
    error: Option<String>,
}

impl Lexer {
    /// A lexer on the first token of `source`.
    pub(super) fn new(source: &str) -> Self {
        let mut lexer = Self {
            units: source.encode_utf16().collect(),
            position: 0,
            current: Token::End,
            error: None,
        };
        lexer.next();
        lexer
    }

    /// The token the parser is on.
    pub(super) const fn current(&self) -> &Token {
        &self.current
    }

    /// The token the parser is on, taken out to keep its text; the parser moves on or stops.
    pub(super) fn take(&mut self) -> Token {
        std::mem::replace(&mut self.current, Token::End)
    }

    /// Why a token could not be read, once one could not.
    pub(super) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Moves to the next token.
    pub(super) fn next(&mut self) {
        self.current = self.lex();
    }

    fn peek(&self) -> Option<char> {
        self.units.get(self.position).copied().map(character)
    }

    fn at(&self, wanted: char) -> bool {
        self.peek() == Some(wanted)
    }

    fn at_digit(&self) -> bool {
        self.peek().is_some_and(|next| next.is_ascii_digit())
    }

    fn follows(&self, word: &str) -> bool {
        let units = self.units.get(self.position..).unwrap_or_default();
        units.starts_with(&word.encode_utf16().collect::<Vec<u16>>())
    }

    fn fail(&mut self, message: String) -> Token {
        self.error = Some(message);
        Token::Error
    }

    /// `true`, `false` and `null` are matched as prefixes: `truex` is `true` and then `x`.
    fn lex(&mut self) -> Token {
        while self.units.get(self.position).copied().is_some_and(is_blank) {
            self.position += 1;
        }
        let Some(next) = self.peek() else {
            return Token::End;
        };
        let word = [
            ("true", Token::True),
            ("false", Token::False),
            ("null", Token::Null),
        ]
        .into_iter()
        .find(|(word, _)| word.starts_with(next) && self.follows(word));
        if let Some((word, token)) = word {
            self.position += word.len();
            return token;
        }
        match next {
            '"' => self.string(),
            '\'' => self.fail("Single quotes (') are not allowed in JSON".to_owned()),
            '-' | '0'..='9' => self.number(),
            'A'..='Z' | 'a'..='z' | '_' | '$' => self.identifier(),
            '[' | ']' | '{' | '}' | '(' | ')' | ',' | ':' | '.' | '=' | ';' => {
                self.position += 1;
                Token::Punctuator(next)
            }
            _ => self.fail(format!("Unrecognized token '{next}'")),
        }
    }

    fn identifier(&mut self) -> Token {
        let start = self.position;
        while self.peek().is_some_and(|next| {
            next.is_ascii_alphanumeric() || matches!(next, '_' | '$' | '\u{200C}' | '\u{200D}')
        }) {
            self.position += 1;
        }
        Token::Identifier(self.since(start))
    }

    fn since(&self, start: usize) -> Vec<u16> {
        self.units
            .get(start..self.position)
            .unwrap_or_default()
            .to_vec()
    }

    fn skip_digits(&mut self) {
        while self.at_digit() {
            self.position += 1;
        }
    }

    /// `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`. A sign with no digit and a point with no
    /// digit are errors; an exponent with no digit is not read, so the number ends before its `e`
    /// and the `e` is the next token.
    fn number(&mut self) -> Token {
        let start = self.position;
        if self.at('-') {
            self.position += 1;
        }
        if self.at('0') {
            self.position += 1;
        } else if self.peek().is_some_and(|next| matches!(next, '1'..='9')) {
            self.skip_digits();
        } else {
            return self.fail("Invalid number".to_owned());
        }
        if self.at('.') {
            self.position += 1;
            if !self.at_digit() {
                return self.fail("Invalid digits after decimal point".to_owned());
            }
            self.skip_digits();
        }
        if self.at('e') || self.at('E') {
            let exponent = self.position;
            self.position += 1;
            if self.at('+') || self.at('-') {
                self.position += 1;
            }
            if self.at_digit() {
                self.skip_digits();
            } else {
                self.position = exponent;
            }
        }
        // The digits are ASCII and of JSON's shape, which Rust reads correctly rounded, as
        // JavaScriptCore does.
        Token::Number(text(&self.since(start)).parse().unwrap_or(f64::NAN))
    }

    /// A string ends at its `"`; a character below U+0020 or the end of the text first leaves it
    /// unterminated.
    fn string(&mut self) -> Token {
        self.position += 1;
        let mut units = Vec::new();
        loop {
            match self.units.get(self.position).copied() {
                None => return self.fail("Unterminated string".to_owned()),
                Some(0x22) => {
                    self.position += 1;
                    return Token::String(units);
                }
                Some(0x5C) => {
                    self.position += 1;
                    if let Err(message) = self.escape(&mut units) {
                        return self.fail(message);
                    }
                }
                Some(unit) if unit < 0x20 => return self.fail("Unterminated string".to_owned()),
                Some(unit) => {
                    units.push(unit);
                    self.position += 1;
                }
            }
        }
    }

    /// The escape after a `\`. A `\u` needs five code units from its `u` before its digits are
    /// looked at; then each of the four must be hexadecimal. A surrogate it writes stays a code
    /// unit, so two escapes of a pair are one character.
    fn escape(&mut self, units: &mut Vec<u16>) -> Result<(), String> {
        let Some(unit) = self.units.get(self.position).copied() else {
            return Err("Unterminated string".to_owned());
        };
        let simple = match character(unit) {
            '"' => Some(0x22),
            '\\' => Some(0x5C),
            '/' => Some(0x2F),
            'b' => Some(0x08),
            'f' => Some(0x0C),
            'n' => Some(0x0A),
            'r' => Some(0x0D),
            't' => Some(0x09),
            'u' => None,
            other => return Err(format!("Invalid escape character {other}")),
        };
        if let Some(simple) = simple {
            units.push(simple);
            self.position += 1;
            return Ok(());
        }
        let Some(escape) = self.units.get(self.position..self.position + 5) else {
            return Err("\\u must be followed by 4 hex digits".to_owned());
        };
        let digits = text(escape.get(1..).unwrap_or_default());
        let value = digits
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
            .then(|| u16::from_str_radix(&digits, 16).ok())
            .flatten();
        let Some(value) = value else {
            return Err(format!(
                "\"\\{}\" is not a valid unicode escape",
                text(escape)
            ));
        };
        units.push(value);
        self.position += 5;
        Ok(())
    }
}
