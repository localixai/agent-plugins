//! The boundary's own JSON reader: the arguments and answers a host sends are JSON text its
//! bridge wrote with `JSON.stringify`, so they are well formed and need no JavaScriptCore error
//! wording. `js::json_parse` reads JSON as `JSON.parse` does, message for message, over UTF-16
//! code units; that is right for a plugin's files and far too slow for a call carrying a 64 MiB
//! string. This reads the bytes once, copies each run of plain characters whole, and answers
//! the same value `js::json_parse` gives for any text `JSON.stringify` writes (a lone surrogate
//! escape is U+FFFD there too, docs/RUST.md). Its error is a fault: a bridge wrote bad JSON.

use crate::js::{JsObject, JsValue, in_property_order};

/// Reads the JSON text of a call or a resume into a value, wire tags still in it.
///
/// # Errors
///
/// A description of where the text is not JSON, for a fault.
pub(super) fn read(text: &str) -> Result<JsValue, String> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        text,
        at: 0,
    };
    reader.space();
    let value = reader.value(0)?;
    reader.space();
    if reader.at == reader.bytes.len() {
        Ok(value)
    } else {
        Err(reader.fail("text after the value"))
    }
}

/// How deep the reader goes, as `js::json_parse` does: a value deeper would overflow the stack
/// when dropped.
const DEPTH: usize = 1000;

struct Reader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn fail(&self, what: &str) -> String {
        format!("{what} at byte {}", self.at)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn word(&mut self, word: &str, value: JsValue) -> Result<JsValue, String> {
        if self.text.get(self.at..self.at + word.len()) == Some(word) {
            self.at += word.len();
            Ok(value)
        } else {
            Err(self.fail("an unknown word"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<JsValue, String> {
        if depth > DEPTH {
            return Err(self.fail("nesting deeper than 1000 levels"));
        }
        match self.peek() {
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(JsValue::String),
            Some(b't') => self.word("true", JsValue::Bool(true)),
            Some(b'f') => self.word("false", JsValue::Bool(false)),
            Some(b'n') => self.word("null", JsValue::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(self.fail("no value")),
        }
    }

    fn number(&mut self) -> Result<JsValue, String> {
        let start = self.at;
        while matches!(
            self.peek(),
            Some(b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9')
        ) {
            self.at += 1;
        }
        let digits = self.text.get(start..self.at).unwrap_or_default();
        digits
            .parse::<f64>()
            .map(JsValue::Number)
            .map_err(|_bad| self.fail("a bad number"))
    }

    fn array(&mut self, depth: usize) -> Result<JsValue, String> {
        self.at += 1;
        let mut items = Vec::new();
        self.space();
        if self.peek() == Some(b']') {
            self.at += 1;
            return Ok(JsValue::Array(items));
        }
        loop {
            self.space();
            items.push(self.value(depth + 1)?);
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;
                    return Ok(JsValue::Array(items));
                }
                _ => return Err(self.fail("no `,` or `]`")),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<JsValue, String> {
        self.at += 1;
        let mut members = JsObject::new();
        self.space();
        if self.peek() == Some(b'}') {
            self.at += 1;
            return Ok(JsValue::Object(members));
        }
        loop {
            self.space();
            if self.peek() != Some(b'"') {
                return Err(self.fail("no member name"));
            }
            let key = self.string()?;
            self.space();
            if self.peek() != Some(b':') {
                return Err(self.fail("no `:`"));
            }
            self.at += 1;
            self.space();
            let value = self.value(depth + 1)?;
            // A later member of the same name wins and keeps the first one's place, as in
            // JavaScript; `JSON.stringify` never writes one, but the reader stays total.
            members.insert(key, value);
            self.space();
            match self.peek() {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;
                    return Ok(JsValue::Object(in_property_order(members)));
                }
                _ => return Err(self.fail("no `,` or `}`")),
            }
        }
    }

    /// A string from its opening quote: every run between escapes is copied as it is (the text is
    /// UTF-8 already), each escape decoded.
    fn string(&mut self) -> Result<String, String> {
        self.at += 1;
        let mut out = String::new();
        loop {
            let start = self.at;
            while let Some(byte) = self.peek() {
                if byte == b'"' || byte == b'\\' || byte < 0x20 {
                    break;
                }
                self.at += 1;
            }
            out.push_str(self.text.get(start..self.at).unwrap_or_default());
            match self.peek() {
                Some(b'"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.at += 1;
                    self.escape(&mut out)?;
                }
                _ => return Err(self.fail("an unterminated string")),
            }
        }
    }

    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        let Some(byte) = self.peek() else {
            return Err(self.fail("an unterminated escape"));
        };
        self.at += 1;
        let plain = match byte {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.unicode(out),
            _ => return Err(self.fail("a bad escape")),
        };
        out.push(plain);
        Ok(())
    }

    fn hex4(&mut self) -> Result<u16, String> {
        let digits = self
            .text
            .get(self.at..self.at + 4)
            .ok_or_else(|| self.fail("a short \\u"))?;
        let unit = u16::from_str_radix(digits, 16).map_err(|_bad| self.fail("a bad \\u"))?;
        self.at += 4;
        Ok(unit)
    }

    /// A `\u` escape; a surrogate pair written as two escapes is one character, and a lone
    /// surrogate is U+FFFD, as `js::json_parse` reads it.
    fn unicode(&mut self, out: &mut String) -> Result<(), String> {
        let unit = self.hex4()?;
        if (0xD800..0xDC00).contains(&unit) && self.text.get(self.at..self.at + 2) == Some("\\u") {
            let back = self.at;
            self.at += 2;
            let low = self.hex4()?;
            if (0xDC00..0xE000).contains(&low) {
                out.extend(char::decode_utf16([unit, low]).map(|c| c.unwrap_or('\u{FFFD}')));
                return Ok(());
            }
            self.at = back;
        }
        out.push(char::from_u32(u32::from(unit)).unwrap_or('\u{FFFD}'));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::read;
    use crate::js::json_parse::json_parse;

    /// The reader answers what `js::json_parse` answers for text `JSON.stringify` writes.
    #[test]
    fn reads_as_json_parse_does() {
        for text in [
            r#"[1,-0,2.5,1e+21,1e-7,"a\"b\\c\n\t\u0001","é😀",null,true,false,{},[]]"#,
            r#"{"b":1,"a":{"$number":"NaN"},"2":[{"$undefined":true}],"b":3}"#,
            r#"["\ud83d\ude00","\ud800","x\udc00y","\u00e9"]"#,
            " [ 1 , { \"k\" : [ ] } ] ",
        ] {
            assert_eq!(read(text).ok(), json_parse(text).ok(), "{text}");
        }
    }

    #[test]
    fn refuses_what_is_not_json() {
        for text in [
            "",
            "[1,",
            "[1]x",
            "{\"a\" 1}",
            "\"open",
            "[tru]",
            "[\"\\x\"]",
        ] {
            assert!(read(text).is_err(), "{text}");
        }
    }

    #[test]
    fn copies_a_long_string_in_one_run() {
        let long = "é".repeat(1 << 20);
        let text = format!("[\"{long}\",5]");
        let value = read(&text).ok();
        assert_eq!(value, json_parse(&text).ok());
    }
}
