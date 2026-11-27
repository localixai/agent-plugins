//! Ported from `src/format/js-literal.ts`; only the core uses it now, so it has no wrapper.
//!
//! Reads a JavaScript object literal from tokens without running anything: strings, numbers, `true`,
//! `false`, `null`, arrays and objects. A variable, a call, a spread, a computed key, a method, a
//! shorthand property or an expression is not a literal (workflows.md, "Edit a saved script", the
//! `meta` block: "If it contains anything other than literal values, such as a variable, a function
//! call, or a spread, Claude Code drops `/<name>` from `/` autocomplete").
//!
//! Where the TypeScript catches the engine running out of stack, the core stops at `MAX_NESTING`
//! levels of arrays and objects with the engine's message (docs/RUST.md, "Where the crate answers
//! differently").

use serde::{Deserialize, Serialize};

use super::js_tokens::{JsToken, STACK_EXCEEDED, TokenForm};
use crate::js::{JsObject, JsValue, in_property_order, number};

/// How many arrays and objects may nest inside the literal: past it the core answers what the
/// package says when the engine runs out of stack.
pub const MAX_NESTING: usize = 128;

/// One property of the literal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralProperty {
    /// The property's key.
    pub key: String,
    /// Its value.
    pub value: JsValue,
    /// Where the value starts in the source.
    pub value_start: usize,
    /// Where the value ends in the source.
    pub value_end: usize,
}

/// The literal and where its parts are.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteralObject {
    /// The `{`.
    pub start: usize,
    /// The end of the `}`.
    pub end: usize,
    /// The position of the `}` itself.
    pub close_at: usize,
    /// The properties, in order.
    pub properties: Vec<LiteralProperty>,
    /// The last property is followed by a comma.
    pub trailing_comma: bool,
    /// The end of the last property and of its comma, if any; just after the `{` when there is
    /// none.
    pub after_last: usize,
    /// The index of the token after the `}`.
    pub end_index: usize,
}

/// The literal, or why it is not one.
#[derive(Clone, Debug, PartialEq)]
pub enum LiteralRead {
    /// The literal.
    Found {
        /// What was read.
        found: LiteralObject,
    },
    /// Why there is none.
    Problem {
        /// The first reason the text is not a literal.
        problem: String,
    },
}

/// Stands in for a token once the read has failed, so the methods that follow read nothing and
/// stop.
fn no_token() -> JsToken {
    JsToken {
        form: TokenForm::Punct,
        text: String::new(),
        start: 0,
        end: 0,
        value: None,
    }
}

/// Where a property added after the last one goes: after its comma when it has one.
fn after_last(open: &JsToken, last: Option<&LiteralProperty>, before: Option<&JsToken>) -> usize {
    let Some(last) = last else {
        return open.end;
    };
    match before {
        Some(before) if before.text == "," => before.end,
        _ => last.value_end,
    }
}

/// `true`, `false` and `null`.
fn words(text: &str) -> Option<JsValue> {
    match text {
        "true" => Some(JsValue::Bool(true)),
        "false" => Some(JsValue::Bool(false)),
        "null" => Some(JsValue::Null),
        _ => None,
    }
}

struct Reader<'a> {
    /// The first reason the text is not a literal; once set, every read after it returns a
    /// placeholder.
    problem: Option<String>,
    index: usize,
    tokens: &'a [JsToken],
    /// How many arrays and objects the read is inside.
    nesting: usize,
}

impl Reader<'_> {
    fn peek(&self) -> Option<&JsToken> {
        self.tokens.get(self.index)
    }

    /// The token `back` places before the current one.
    fn behind(&self, back: usize) -> Option<&JsToken> {
        self.index
            .checked_sub(back)
            .and_then(|at| self.tokens.get(at))
    }

    /// Records the first problem and moves to the end, so no loop reads on.
    fn fail(&mut self, message: &str) {
        if self.problem.is_none() {
            self.problem = Some(message.to_owned());
        }
        self.index = self.tokens.len();
    }

    fn take(&mut self) -> JsToken {
        let Some(token) = self.peek().cloned() else {
            self.fail("the text ends inside the literal");
            return no_token();
        };
        self.index += 1;
        token
    }

    fn expect(&mut self, text: &str) -> JsToken {
        let token = self.take();
        if token.form != TokenForm::Punct || token.text != text {
            self.fail(&format!("expected \"{text}\" but found \"{}\"", token.text));
        }
        token
    }

    fn at_punct(&self, text: &str) -> bool {
        self.peek()
            .is_some_and(|token| token.form == TokenForm::Punct && token.text == text)
    }

    fn reading(&self, closer: &str) -> bool {
        self.problem.is_none() && !self.at_punct(closer)
    }

    fn value(&mut self) -> JsValue {
        let token = self.take();
        match token.form {
            TokenForm::String => return JsValue::String(self.text(&token)),
            TokenForm::Number => return self.number(&token.text, 1.0),
            TokenForm::Word => return self.word(&token),
            TokenForm::Regex | TokenForm::Punct => {}
        }
        if token.text == "-" {
            let next = self.take();
            return self.number(&next.text, -1.0);
        }
        if token.text == "{" || token.text == "[" {
            return self.nested(token.text == "{");
        }
        self.fail(&format!("\"{}\" is not a literal", token.text));
        JsValue::Undefined
    }

    /// An object or an array, one level deeper.
    fn nested(&mut self, object: bool) -> JsValue {
        if self.nesting >= MAX_NESTING {
            self.fail(&format!("code nested too deep to read ({STACK_EXCEEDED})"));
            return JsValue::Undefined;
        }
        self.nesting += 1;
        let value = if object { self.object() } else { self.array() };
        self.nesting -= 1;
        value
    }

    fn text(&mut self, token: &JsToken) -> String {
        if let Some(value) = &token.value {
            return value.clone();
        }
        self.fail("a template literal with ${...} is not a literal");
        String::new()
    }

    fn number(&mut self, text: &str, sign: f64) -> JsValue {
        let value = number(&text.replace('_', ""));
        if !value.is_nan() {
            return JsValue::Number(sign * value);
        }
        self.fail(&format!("\"{text}\" is not a number literal"));
        JsValue::Number(0.0)
    }

    fn word(&mut self, token: &JsToken) -> JsValue {
        if let Some(known) = words(&token.text) {
            return known;
        }
        self.fail(&format!("\"{}\" is not a literal", token.text));
        JsValue::Null
    }

    fn array(&mut self) -> JsValue {
        let mut items = Vec::new();
        while self.reading("]") {
            items.push(self.value());
            if !self.at_punct("]") {
                self.expect(",");
            }
        }
        self.expect("]");
        JsValue::Array(items)
    }

    fn key(&mut self) -> String {
        let token = self.take();
        match token.form {
            TokenForm::Word | TokenForm::Number => token.text,
            TokenForm::String => self.text(&token),
            TokenForm::Regex | TokenForm::Punct => {
                self.fail(&format!("\"{}\" is not a key", token.text));
                String::new()
            }
        }
    }

    fn object(&mut self) -> JsValue {
        let entries: JsObject = self
            .properties()
            .into_iter()
            .map(|each| (each.key, each.value))
            .collect();
        JsValue::Object(in_property_order(entries))
    }

    fn properties(&mut self) -> Vec<LiteralProperty> {
        let mut found = Vec::new();
        while self.reading("}") {
            found.push(self.property());
            if !self.at_punct("}") {
                self.expect(",");
            }
        }
        self.expect("}");
        found
    }

    fn property(&mut self) -> LiteralProperty {
        let key = self.key();
        self.expect(":");
        let value_start = self.peek().map_or(0, |token| token.start);
        let value = self.value();
        let value_end = self.behind(1).map_or(value_start, |last| last.end);
        LiteralProperty {
            key,
            value,
            value_start,
            value_end,
        }
    }

    /// The object that starts at the current token; meaningless when `problem` is set after it.
    fn object_literal(&mut self) -> LiteralObject {
        let open = self.expect("{");
        let properties = self.properties();
        let close = self.behind(1);
        let before = self.behind(2);
        LiteralObject {
            start: open.start,
            end: close.map_or(open.end, |close| close.end),
            close_at: close.map_or(open.start, |close| close.start),
            trailing_comma: before.is_some_and(|before| before.text == ","),
            after_last: after_last(&open, properties.last(), before),
            end_index: self.index,
            properties,
        }
    }
}

/// The object literal that starts at `tokens[from]`, or why it is not a literal. Never throws.
#[must_use]
pub fn read_object_literal(tokens: &[JsToken], from: usize) -> LiteralRead {
    let mut reader = Reader {
        problem: None,
        index: from,
        tokens,
        nesting: 0,
    };
    let found = reader.object_literal();
    match reader.problem {
        None => LiteralRead::Found { found },
        Some(problem) => LiteralRead::Problem { problem },
    }
}

#[cfg(test)]
mod tests {
    use super::{LiteralRead, MAX_NESTING, read_object_literal};
    use crate::format::js_tokens::tokenize;

    /// The problem of reading `{ x: <depth arrays> }`, if any.
    fn problem(depth: usize) -> Option<String> {
        let source = format!("{{ x: {}{} }}", "[".repeat(depth), "]".repeat(depth));
        match read_object_literal(&tokenize(&source, None).tokens, 0) {
            LiteralRead::Found { .. } => None,
            LiteralRead::Problem { problem } => Some(problem),
        }
    }

    #[test]
    fn a_literal_nested_past_the_limit_is_the_engines_stack_problem() {
        assert_eq!(problem(MAX_NESTING - 1), None);
        assert_eq!(
            problem(200_000).as_deref(),
            Some("code nested too deep to read (Maximum call stack size exceeded.)")
        );
    }
}
