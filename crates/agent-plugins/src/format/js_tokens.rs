//! Port of `src/format/js-tokens.ts`, switched.
//!
//! A tokenizer for the JavaScript of a workflow script (workflows.md, "What the saved script looks
//! like", "Edit a saved script"). The kit never runs a script and never parses it fully: it needs to
//! tell code from the text that only looks like code (comments, strings, template text, regular
//! expressions) and to read a literal. So this keeps words, numbers, strings and one-character
//! punctuation (`...` is one token), skips comments, and puts the tokens of a template's `${...}` in
//! the stream right after the template's own token.
//!
//! Positions count UTF-16 code units, as the TypeScript's string indexes do. Where the TypeScript
//! catches the engine running out of stack, the core stops at `MAX_NESTING` levels of `${...}` with
//! the engine's message (docs/RUST.md, "Where the crate answers differently").
//!
//! The source's sticky patterns are matchers in `js_tokens/patterns.rs`, and the scanner's strings
//! and templates are in `js_tokens/strings.rs`, in the source's order, to keep each file in RS5.

use serde::{Deserialize, Serialize};

mod patterns;
mod strings;

use patterns::{is, number_at, space_at, word_at};

use crate::js::utf16::{index_of, slice, starts_with_at, text_of};
use crate::js::{JsValue, value::to_js_value};
use crate::registry::Callback;

/// What a token is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenForm {
    /// An identifier or a keyword.
    Word,
    /// A number literal.
    Number,
    /// A string or a template.
    String,
    /// A regular expression literal.
    Regex,
    /// One character of punctuation, or `...`.
    Punct,
}

/// One token of a script.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsToken {
    /// What the token is.
    pub form: TokenForm,
    /// The source text of the token.
    pub text: String,
    /// Where it starts, in UTF-16 code units.
    pub start: usize,
    /// Where it ends, in UTF-16 code units.
    pub end: usize,
    /// A string's decoded text; `None` for a template with a `${...}` in it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// The tokens of a script and why it cannot be JavaScript.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct JsTokens {
    /// The tokens, in order.
    pub tokens: Vec<JsToken>,
    /// Why the text cannot be JavaScript at all: an unterminated string, comment, template or
    /// regex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// After one of these words a `/` starts a regular expression.
const REGEX_AFTER_WORD: [&str; 14] = [
    "return",
    "typeof",
    "instanceof",
    "in",
    "of",
    "new",
    "delete",
    "void",
    "throw",
    "case",
    "do",
    "else",
    "yield",
    "await",
];
const DIVISION_AFTER_PUNCT: [&str; 3] = [")", "]", "}"];

/// How a character changes the brace depth of the code around it.
fn brace_depth(char: Option<u16>) -> i64 {
    match char {
        Some(0x7B) => 1,
        Some(0x7D) => -1,
        _ => 0,
    }
}

/// How many `${...}` may nest inside each other: past it the core answers what the package says
/// when the engine runs out of stack.
pub const MAX_NESTING: usize = 128;

/// The message of the engine running out of stack (JavaScriptCore's `RangeError`).
pub const STACK_EXCEEDED: &str = "Maximum call stack size exceeded.";

/// Whether a top-level scan can stop after the token just taken: the tokens so far and the brace
/// depth after it. A scan that stops reports only the problems met so far.
pub type TokenStop<'a> = &'a mut dyn FnMut(&[JsToken], i64) -> bool;

struct Scanner<'a> {
    tokens: Vec<JsToken>,
    problem: Option<String>,
    at: usize,
    previous: Option<JsToken>,
    text: &'a [u16],
    /// How many `${...}` this scanner is inside.
    level: usize,
}

impl<'a> Scanner<'a> {
    const fn new(text: &'a [u16], level: usize) -> Self {
        Self {
            tokens: Vec::new(),
            problem: None,
            at: 0,
            previous: None,
            text,
            level,
        }
    }

    /// Scans to the end, or (inside a template) to the `}` that closes the `${`, or (at the top) to
    /// where `stop` says the caller has what it needs.
    fn run(&mut self, in_template: bool, stop: Option<TokenStop<'_>>) {
        let mut stop = stop;
        let mut depth = 0;
        while self.step(in_template, &mut depth) {
            if let Some(stop) = stop.as_mut()
                && stop(&self.tokens, depth)
            {
                return;
            }
        }
    }

    /// One turn of `run`'s loop: false when the scan ends before taking a token.
    fn step(&mut self, in_template: bool, depth: &mut i64) -> bool {
        if self.problem.is_some() || !self.skip_trivia() {
            return false;
        }
        let char = self.text.get(self.at).copied();
        if in_template && is(char, '}') && *depth == 0 {
            return false;
        }
        *depth += brace_depth(char);
        self.next(char);
        true
    }

    fn fail(&mut self, message: &str) {
        if self.problem.is_none() {
            self.problem = Some(message.to_owned());
        }
    }

    /// Skips whitespace and comments; false at the end of the text.
    fn skip_trivia(&mut self) -> bool {
        loop {
            let space = space_at(self.text, self.at);
            self.at += space;
            if starts_with_at(self.text, self.at, "//") {
                self.at = self.line_end();
            } else if starts_with_at(self.text, self.at, "/*") {
                self.block_comment();
            } else if space == 0 {
                return self.at < self.text.len() && self.problem.is_none();
            }
            if self.problem.is_some() {
                return false;
            }
        }
    }

    fn line_end(&self) -> usize {
        index_of(self.text, "\n", self.at).unwrap_or(self.text.len())
    }

    fn block_comment(&mut self) {
        match index_of(self.text, "*/", self.at + 2) {
            None => self.fail("unterminated comment"),
            Some(end) => self.at = end + 2,
        }
    }

    fn push(&mut self, token: JsToken) {
        self.previous = Some(token.clone());
        self.tokens.push(token);
    }

    fn next(&mut self, char: Option<u16>) {
        let start = self.at;
        if is(char, '\'') || is(char, '"') {
            return self.quoted(char);
        }
        if is(char, '`') {
            return self.template();
        }
        if let Some(length) = word_at(self.text, start) {
            return self.take(TokenForm::Word, length);
        }
        if let Some(length) = number_at(self.text, start) {
            return self.take(TokenForm::Number, length);
        }
        if is(char, '/') && self.regex_allowed() {
            return self.regex();
        }
        let length = if starts_with_at(self.text, start, "...") {
            3
        } else {
            1
        };
        self.take(TokenForm::Punct, length);
    }

    fn take(&mut self, form: TokenForm, length: usize) {
        let start = self.at;
        self.at += length;
        let text = text_of(slice(self.text, start, self.at));
        self.push(JsToken {
            form,
            text,
            start,
            end: self.at,
            value: None,
        });
    }

    fn regex_allowed(&self) -> bool {
        let Some(before) = &self.previous else {
            return true;
        };
        match before.form {
            TokenForm::Punct => !DIVISION_AFTER_PUNCT.contains(&before.text.as_str()),
            TokenForm::Word => REGEX_AFTER_WORD.contains(&before.text.as_str()),
            TokenForm::Number | TokenForm::String | TokenForm::Regex => false,
        }
    }

    fn regex(&mut self) {
        let start = self.at;
        let mut at = start + 1;
        let mut in_class = false;
        while at < self.text.len() {
            let char = self.text.get(at).copied();
            if is(char, '\n') {
                break;
            }
            if is(char, '\\') {
                at += 1;
            } else if is(char, '[') {
                in_class = true;
            } else if is(char, ']') {
                in_class = false;
            } else if is(char, '/') && !in_class {
                return self.finish_regex(start, at + 1);
            }
            at += 1;
        }
        self.take(TokenForm::Punct, 1);
    }

    fn finish_regex(&mut self, start: usize, body_end: usize) {
        let flags = word_at(self.text, body_end).unwrap_or(0);
        self.at = body_end + flags;
        let text = text_of(slice(self.text, start, self.at));
        self.push(JsToken {
            form: TokenForm::Regex,
            text,
            start,
            end: self.at,
            value: None,
        });
    }
}

/// Tokens of `text`, up to where `stop` says, or all of them; never throws. A template's `${...}` is
/// scanned by a scanner of its own, one stack frame per level: nesting past the stack is a problem,
/// as an unterminated template is (judgment, spec/judgments.md).
#[must_use]
pub fn tokenize(text: &str, stop: Option<TokenStop<'_>>) -> JsTokens {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut scanner = Scanner::new(&units, 0);
    scanner.run(false, stop);
    JsTokens {
        tokens: scanner.tokens,
        problem: scanner.problem,
    }
}

/// `tokenize` with the host's `stop`, called after each token as `run` calls it.
async fn tokenize_for_host(text: &str, stop: Option<Callback>) -> JsTokens {
    let Some(stop) = stop else {
        return tokenize(text, None);
    };
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut scanner = Scanner::new(&units, 0);
    let mut depth = 0;
    while scanner.step(false, &mut depth) {
        let tokens = to_js_value(&scanner.tokens).unwrap_or_default();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a brace depth is far below 2^53"
        )]
        let args = vec![tokens, JsValue::Number(depth as f64)];
        if stop.call::<JsValue>(args).await == JsValue::Bool(true) {
            break;
        }
    }
    JsTokens {
        tokens: scanner.tokens,
        problem: scanner.problem,
    }
}

crate::registry::exports! {
    "format/js-tokens" {
        "tokenize" => async ok |text: String, stop: Option<Callback>| tokenize_for_host(&text, stop).await;
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_NESTING, tokenize};

    /// `${` nested `depth` times around `x`.
    fn nested(depth: usize) -> String {
        format!("{}x{}", "`${".repeat(depth), "}`".repeat(depth))
    }

    #[test]
    fn templates_nested_past_the_limit_are_the_engines_stack_problem() {
        assert_eq!(tokenize(&nested(MAX_NESTING), None).problem, None);
        assert_eq!(
            tokenize(&nested(200_000), None).problem.as_deref(),
            Some("code nested too deep to scan (Maximum call stack size exceeded.)")
        );
    }
}
