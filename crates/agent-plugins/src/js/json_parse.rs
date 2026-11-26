//! `JSON.parse` as the npm package's runtime (Bun, JavaScriptCore) answers it: the value of a
//! JSON text, or the message of the `SyntaxError` it throws (`JSON Parse error: <reason>`). The
//! decisions are JavaScriptCore's, found by behaviour against `scripts/rust/js-oracle.ts`:
//!
//! - The lexer reads one token ahead of the parser. When it could not read a token, its message
//!   is the reason, whatever the parser said after; else the parser's; else, when a whole value
//!   was read but more follows (`{} x`, `0;`, `01`, `"a" :`), `Unable to parse JSON string`.
//! - Blanks are JSON's four (space, tab, line feed, carriage return). Any other character that
//!   starts no token is `Unrecognized token 'c'` (a no-break space, a byte order mark, `\0`,
//!   `+`, `\`, anything above U+007F); `'` is `Single quotes (') are not allowed in JSON`.
//! - `true`, `false` and `null` are matched as prefixes (`truex` is `true`, then `x`); another
//!   run of `[A-Za-z0-9_$]` (U+200C and U+200D too, after its first character) is an identifier, which as a value is `Unexpected identifier "w"`
//!   (the run quoted up to 200 characters, `...` after a longer one). A punctuator where a value
//!   belongs is `Unexpected token 'c'` (one of `] } ( ) , : . = ;`); the end is `Unexpected EOF`.
//! - A number is `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`, read as a double. A sign with
//!   no digit is `Invalid number`, a point with no digit `Invalid digits after decimal point`; an
//!   exponent with no digit is left unread, so the `e` is the next token (`[1e]` is `Expected
//!   ']'`).
//! - A string ending in the text or at a character below U+0020 is `Unterminated string`; a bad
//!   escape is `Invalid escape character c`; a `\u` with fewer than five code units from its `u`
//!   is `\u must be followed by 4 hex digits`, else four non-hex digits are `"\uXXXX" is not a
//!   valid unicode escape` (the five units quoted).
//! - In an array, after a value: `,` or `]`, else `Expected ']'`; `]` right after `,` is
//!   `Unexpected comma at the end of array expression`.
//! - In an object: after `{` a string or `}`, else `Expected '}'`; after a key `:`, else
//!   `Expected ':' before value in object property definition`; after a value `,` or `}`, else
//!   `Expected '}'`; after `,` a string, else `Property name must be a string literal`.
//! - An object's members come out in JavaScript's order: array-index keys (`"0"`, `"10"`)
//!   ascending first, then the others as written; a repeated key keeps its first place.
//! - JavaScriptCore sets no limit to nesting (its parser keeps the open arrays and objects on a
//!   stack of its own; a million levels parse). A [`JsValue`] some thousands of levels deep
//!   overflows the stack when it is dropped, cloned or printed, an abort no plugin may cause
//!   (RS4), so past `MAX_NESTING` levels the text is refused: `Nesting deeper than 1000 levels`
//!   (judgment: the one text JavaScriptCore reads that this parser refuses).
//!
//! The value is what JavaScript makes of the text, a [`JsValue`]: every number a double (`-0`
//! kept, a number beyond a double's range an infinity), a repeated key's last value at its first
//! place, `__proto__` an ordinary member, and an object that looks like a wire tag
//! (`{"$undefined": true}`) an ordinary object. A lone surrogate that a `\u` escape writes, which
//! no Rust string holds, reads as U+FFFD (judgment).

mod lexer;

use self::lexer::{Lexer, Token, text};
use crate::js::{JsObject, JsValue};

const PREFIX: &str = "JSON Parse error: ";

/// The longest identifier an error message quotes whole.
const MAX_QUOTED: usize = 200;

/// The deepest nesting of arrays and objects read; a value twice as deep is still dropped,
/// cloned and printed on a 2 MiB stack in a debug build.
const MAX_NESTING: usize = 1000;

/// The value of a JSON text, or the message `JSON.parse` throws for it.
///
/// # Errors
///
/// The `SyntaxError` message of the text that is not JSON.
pub fn json_parse(source: &str) -> Result<JsValue, String> {
    let mut parser = Parser {
        lexer: Lexer::new(source),
        open: Vec::new(),
    };
    let parsed = parser.parse().map_err(Some);
    let whole = parsed.and_then(|value| {
        if *parser.lexer.current() == Token::End {
            Ok(value)
        } else {
            Err(None)
        }
    });
    whole.map_err(|reason| {
        let reason = parser
            .lexer
            .error()
            .map(str::to_owned)
            .or(reason)
            .unwrap_or_else(|| "Unable to parse JSON string".to_owned());
        format!("{PREFIX}{reason}")
    })
}

/// An array or object whose closing token is still ahead, with what it holds so far; an object
/// also holds the key of the member whose value is being read.
enum Open {
    Array(Vec<JsValue>),
    Object(JsObject, String),
}

/// Where the parser is: at the start of a value, or past one it has just read.
enum Step {
    Value,
    Done(JsValue),
}

struct Parser {
    lexer: Lexer,
    open: Vec<Open>,
}

impl Parser {
    /// Reads one value; each value read is handed to the array or object it belongs to, until
    /// the outermost is closed.
    fn parse(&mut self) -> Result<JsValue, String> {
        loop {
            let mut step = self.value()?;
            while let Step::Done(value) = step {
                step = match self.open.pop() {
                    None => return Ok(value),
                    Some(Open::Array(items)) => self.after_element(items, value)?,
                    Some(Open::Object(members, key)) => self.after_member(members, key, value)?,
                };
            }
        }
    }

    /// The value at the current token: a primitive read whole, or an array or object opened.
    fn value(&mut self) -> Result<Step, String> {
        let value = match self.lexer.take() {
            Token::Punctuator('[') => return self.start_array(),
            Token::Punctuator('{') => return self.start_object(),
            Token::String(units) => JsValue::String(text(&units)),
            Token::Number(number) => JsValue::Number(number),
            Token::True => JsValue::Bool(true),
            Token::False => JsValue::Bool(false),
            Token::Null => JsValue::Null,
            Token::Identifier(word) => return Err(unexpected_identifier(&word)),
            Token::Punctuator(other) => return Err(format!("Unexpected token '{other}'")),
            Token::End => return Err("Unexpected EOF".to_owned()),
            Token::Error => return Err("Could not parse value expression".to_owned()),
        };
        self.lexer.next();
        Ok(Step::Done(value))
    }

    /// Refuses an array or object that would sit deeper than `MAX_NESTING`.
    fn nest(&self) -> Result<(), String> {
        if self.open.len() < MAX_NESTING {
            return Ok(());
        }
        Err(format!("Nesting deeper than {MAX_NESTING} levels"))
    }

    fn start_array(&mut self) -> Result<Step, String> {
        self.nest()?;
        self.lexer.next();
        if *self.lexer.current() == Token::Punctuator(']') {
            self.lexer.next();
            return Ok(Step::Done(JsValue::Array(Vec::new())));
        }
        self.open.push(Open::Array(Vec::new()));
        Ok(Step::Value)
    }

    fn after_element(&mut self, mut items: Vec<JsValue>, value: JsValue) -> Result<Step, String> {
        items.push(value);
        if *self.lexer.current() == Token::Punctuator(',') {
            self.lexer.next();
            if *self.lexer.current() == Token::Punctuator(']') {
                return Err("Unexpected comma at the end of array expression".to_owned());
            }
            self.open.push(Open::Array(items));
            return Ok(Step::Value);
        }
        if *self.lexer.current() == Token::Punctuator(']') {
            self.lexer.next();
            return Ok(Step::Done(JsValue::Array(items)));
        }
        Err("Expected ']'".to_owned())
    }

    fn start_object(&mut self) -> Result<Step, String> {
        self.nest()?;
        self.lexer.next();
        let token = self.lexer.take();
        if let Token::String(key) = token {
            return self.member(JsObject::new(), &key);
        }
        if token == Token::Punctuator('}') {
            self.lexer.next();
            return Ok(Step::Done(JsValue::Object(JsObject::new())));
        }
        Err("Expected '}'".to_owned())
    }

    /// A member from its key on: the `:`, then its value is read.
    fn member(&mut self, members: JsObject, key: &[u16]) -> Result<Step, String> {
        self.lexer.next();
        if *self.lexer.current() != Token::Punctuator(':') {
            return Err("Expected ':' before value in object property definition".to_owned());
        }
        self.lexer.next();
        self.open.push(Open::Object(members, text(key)));
        Ok(Step::Value)
    }

    fn after_member(
        &mut self,
        mut members: JsObject,
        key: String,
        value: JsValue,
    ) -> Result<Step, String> {
        members.insert(key, value);
        if *self.lexer.current() == Token::Punctuator(',') {
            self.lexer.next();
            if let Token::String(key) = self.lexer.take() {
                return self.member(members, &key);
            }
            return Err("Property name must be a string literal".to_owned());
        }
        if *self.lexer.current() == Token::Punctuator('}') {
            self.lexer.next();
            return Ok(Step::Done(JsValue::Object(super::in_property_order(
                members,
            ))));
        }
        Err("Expected '}'".to_owned())
    }
}

/// An identifier where a value belongs, quoted up to `MAX_QUOTED` characters and `...` after.
fn unexpected_identifier(word: &[u16]) -> String {
    let quoted = word.get(..MAX_QUOTED).unwrap_or(word);
    let more = if quoted.len() < word.len() { "..." } else { "" };
    format!("Unexpected identifier \"{}{more}\"", text(quoted))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(text: &str) -> String {
        json_parse(text).err().unwrap_or_default()
    }

    #[test]
    fn a_json_text_is_its_value() {
        let value = json_parse(r#"{ "name": "plug", "a": 1, "name": [1.5, -0, 1e400] }"#);
        assert_eq!(
            value.map(|parsed| parsed.to_json_text()).ok().as_deref(),
            Some(r#"{"name":[1.5,{"$number":"-0"},{"$number":"Infinity"}],"a":1}"#)
        );
        let tag = json_parse(r#"{"$undefined":true}"#).ok();
        assert!(tag.is_some_and(|value| value.is_object()));
    }

    // bun -e 'for (const t of ["tru\u200d", "nul\u200dl", "true\u200d", "\u200dx", "x\u200cy"])
    //   try { JSON.parse(t) } catch (e) { console.log(e.message) }'
    // The identifier run takes U+200C and U+200D after its first character, and no other
    // non-ASCII one (a scan of every code point above U+007F after `x` found only these two).
    #[test]
    fn an_identifier_run_takes_the_joiners() {
        let cases = [
            ("tru\u{200D}", "Unexpected identifier \"tru\u{200D}\""),
            ("nul\u{200D}l", "Unexpected identifier \"nul\u{200D}l\""),
            ("true\u{200D}", "Unrecognized token '\u{200D}'"),
            ("\u{200D}x", "Unrecognized token '\u{200D}'"),
            ("x\u{200C}y", "Unexpected identifier \"x\u{200C}y\""),
        ];
        for (text, reason) in cases {
            assert_eq!(message(text), format!("{PREFIX}{reason}"), "{text:?}");
        }
    }

    #[test]
    fn javascript_core_words_the_reason() {
        let cases = [
            ("{ nope", "Expected '}'"),
            (r#"{ "name": "#, "Unexpected EOF"),
            ("[1,]", "Unexpected comma at the end of array expression"),
            ("[1e]", "Expected ']'"),
            ("\u{FEFF}{}", "Unrecognized token '\u{FEFF}'"),
            ("{} x", "Unable to parse JSON string"),
            ("{} \\", "Unrecognized token '\\'"),
        ];
        for (text, reason) in cases {
            assert_eq!(message(text), format!("{PREFIX}{reason}"), "{text:?}");
        }
    }

    #[test]
    fn nesting_ends_at_a_thousand_levels() {
        let deep = |depth: usize| format!("{}1{}", "[".repeat(depth), "]".repeat(depth));
        assert!(json_parse(&deep(MAX_NESTING)).is_ok());
        assert_eq!(
            message(&deep(MAX_NESTING + 1)),
            format!("{PREFIX}Nesting deeper than 1000 levels")
        );
        assert_eq!(
            message(&"{\"a\":".repeat(100_000)),
            format!("{PREFIX}Nesting deeper than 1000 levels")
        );
    }
}
