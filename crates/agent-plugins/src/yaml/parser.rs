//! The concrete syntax tree parser of `yaml` (`dist/parse/parser.js`): turns the lexer's tokens
//! into documents, directives and the source tokens between them, each yielded when it is done.
//! Errors are not thrown but yielded as error tokens, or set where a node would be.
//!
//! The line counter's line starts are recorded here as the package's `onNewLine` callback records
//! them: at each line break token and at each line break inside a scalar.

mod block;
mod block_map;
mod document;
mod flow;
mod scalar;

use super::cst::{ErrorToken, Kind, SourceToken, Token, token_type};
use super::lexer::{Lexeme, lex};
use super::units::{Units, unit};

/// The parser's output: the tokens of the stream, and where each line starts.
#[derive(Debug)]
pub(crate) struct Parsed {
    pub(crate) tokens: Vec<Token>,
    pub(crate) line_starts: Vec<usize>,
}

#[derive(Debug)]
struct Parser {
    /// Spaces and sequence indicators count as indentation.
    at_new_line: bool,
    /// The next token is a scalar value.
    at_scalar: bool,
    indent: usize,
    offset: usize,
    /// On the same line as a block map key.
    on_key_line: bool,
    /// The top is the node being built.
    stack: Vec<Token>,
    source: Units,
    kind: Kind,
    line_starts: Vec<usize>,
    out: Vec<Token>,
}

/// `includesToken`.
fn includes(list: &[SourceToken], kind: Kind) -> bool {
    list.iter().any(|token| token.kind == kind)
}

/// `findNonEmptyIndex`: whether the list holds anything but spaces, comments and line breaks.
fn has_non_empty(list: &[SourceToken]) -> bool {
    list.iter()
        .any(|token| !matches!(token.kind, Kind::Space | Kind::Comment | Kind::Newline))
}

/// `getPrevProps`: the tokens a new key's properties are taken from.
fn prev_props(parent: &mut Token) -> Option<&mut Vec<SourceToken>> {
    match parent {
        Token::Document { start, .. } => Some(start),
        Token::BlockMap { items, .. } => items
            .last_mut()
            .map(|item| item.sep.as_mut().unwrap_or(&mut item.start)),
        Token::BlockSeq { items, .. } => items.last_mut().map(|item| &mut item.start),
        Token::Source(_)
        | Token::Error(_)
        | Token::Directive { .. }
        | Token::DocEnd { .. }
        | Token::FlowScalar(_)
        | Token::BlockScalar { .. }
        | Token::FlowCollection { .. } => None,
    }
}

/// `getFirstKeyStartProps`: the tokens after the last indicator or line break (its spaces
/// skipped), taken out of `prev`.
fn first_key_start_props(prev: &mut Vec<SourceToken>) -> Vec<SourceToken> {
    let mut index = prev.len();
    while index > 0 {
        let kind = prev.get(index - 1).map(|token| token.kind);
        if matches!(
            kind,
            Some(
                Kind::DocStart
                    | Kind::ExplicitKeyInd
                    | Kind::MapValueInd
                    | Kind::SeqItemInd
                    | Kind::Newline
            )
        ) {
            break;
        }
        index -= 1;
    }
    // `index` is one past the indicator found (or 0); spaces after it stay in `prev`.
    while prev
        .get(index)
        .is_some_and(|token| token.kind == Kind::Space)
    {
        index += 1;
    }
    prev.split_off(index.min(prev.len()))
}

/// The tokens of a whole source.
pub(crate) fn parse(source: &Units) -> Parsed {
    let mut parser = Parser {
        at_new_line: true,
        at_scalar: false,
        indent: 0,
        offset: 0,
        on_key_line: false,
        stack: Vec::new(),
        source: Vec::new(),
        kind: Kind::Newline,
        line_starts: vec![0],
        out: Vec::new(),
    };
    for lexeme in lex(source) {
        let (text, kind) = match lexeme {
            Lexeme::Document => (vec![0x02], Some(Kind::DocMode)),
            Lexeme::FlowEnd => (vec![0x18], Some(Kind::FlowErrorEnd)),
            Lexeme::Scalar => (vec![0x1F], Some(Kind::Scalar)),
            Lexeme::Text(start, end) => {
                let text = source.get(start..end).unwrap_or_default().to_vec();
                let kind = token_type(&text);
                (text, kind)
            }
        };
        parser.next(text, kind);
    }
    while !parser.stack.is_empty() {
        parser.pop(None);
    }
    Parsed {
        tokens: parser.out,
        line_starts: parser.line_starts,
    }
}

impl Parser {
    /// Advances by one lexical token.
    fn next(&mut self, source: Units, kind: Option<Kind>) {
        let length = source.len();
        self.source = source;
        if self.at_scalar {
            self.at_scalar = false;
            self.step();
            self.offset += length;
            return;
        }
        let Some(kind) = kind else {
            let message = format!(
                "Not a YAML token: {}",
                String::from_utf16_lossy(&self.source)
            );
            self.pop(Some(self.error(message)));
            self.offset += length;
            return;
        };
        if kind == Kind::Scalar {
            self.at_new_line = false;
            self.at_scalar = true;
            self.kind = Kind::Scalar;
            return;
        }
        self.kind = kind;
        self.step();
        match kind {
            Kind::Newline => {
                self.at_new_line = true;
                self.indent = 0;
                self.line_starts.push(self.offset + length);
            }
            Kind::Space => {
                if self.at_new_line && self.source.first() == Some(&unit(' ')) {
                    self.indent += length;
                }
            }
            Kind::ExplicitKeyInd | Kind::MapValueInd | Kind::SeqItemInd => {
                if self.at_new_line {
                    self.indent += length;
                }
            }
            Kind::DocMode | Kind::FlowErrorEnd => return,
            Kind::ByteOrderMark
            | Kind::Scalar
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::FlowMapStart
            | Kind::FlowMapEnd
            | Kind::FlowSeqStart
            | Kind::FlowSeqEnd
            | Kind::Comma
            | Kind::Comment
            | Kind::DirectiveLine
            | Kind::Alias
            | Kind::Anchor
            | Kind::Tag
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => self.at_new_line = false,
        }
        self.offset += length;
    }

    fn source_token(&self) -> SourceToken {
        SourceToken {
            kind: self.kind,
            offset: self.offset,
            indent: self.indent,
            source: self.source.clone(),
        }
    }

    fn error(&self, message: String) -> Token {
        Token::Error(ErrorToken {
            offset: self.offset,
            message,
            source: self.source.clone(),
        })
    }

    fn step(&mut self) {
        let top_is_doc_end = matches!(self.stack.last(), Some(Token::DocEnd { .. }));
        if self.kind == Kind::DocEnd && !top_is_doc_end {
            while !self.stack.is_empty() {
                self.pop(None);
            }
            self.stack.push(Token::DocEnd {
                offset: self.offset,
                source: self.source.clone(),
                end: None,
            });
            return;
        }
        match self.stack.last() {
            None => self.stream(),
            Some(Token::Document { .. }) => self.document(),
            Some(Token::FlowScalar(_)) => self.scalar(),
            Some(Token::BlockScalar { .. }) => self.block_scalar(),
            Some(Token::BlockMap { .. }) => self.block_map(),
            Some(Token::BlockSeq { .. }) => self.block_sequence(),
            Some(Token::FlowCollection { .. }) => self.flow_collection(),
            Some(Token::DocEnd { .. }) => self.document_end(),
            Some(Token::Source(_) | Token::Error(_) | Token::Directive { .. }) => self.pop(None),
        }
    }

    fn pop(&mut self, error: Option<Token>) {
        let token = match error {
            Some(token) => token,
            None => match self.stack.pop() {
                Some(token) => token,
                None => {
                    let message = "Tried to pop an empty stack".to_owned();
                    self.out.push(Token::Error(ErrorToken {
                        offset: self.offset,
                        message,
                        source: Vec::new(),
                    }));
                    return;
                }
            },
        };
        if self.stack.is_empty() {
            self.out.push(token);
        } else {
            self.attach(token);
        }
    }

    /// The `onNewLine` calls for the line breaks inside the current token's source.
    fn note_line_starts(&mut self) {
        for (index, &character) in self.source.iter().enumerate() {
            if character == unit('\n') {
                self.line_starts.push(self.offset + index + 1);
            }
        }
    }
}

/// The text of a message's quoted source: `JSON.stringify(source)`.
pub(crate) fn quoted(source: &[u16]) -> String {
    let mut out = String::new();
    crate::js::value::write_json_string(&String::from_utf16_lossy(source), &mut out);
    out
}
