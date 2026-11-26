//! The parser's stream and documents (`stream`, `document`, `documentEnd` and `lineEnd` of
//! `Parser.prototype`): directives, document markers and what ends a line.

use super::{Parser, has_non_empty};
use crate::yaml::cst::{Kind, Token};

impl Parser {
    pub(super) fn stream(&mut self) {
        match self.kind {
            Kind::DirectiveLine => self.out.push(Token::Directive {
                offset: self.offset,
                source: self.source.clone(),
            }),
            Kind::ByteOrderMark | Kind::Space | Kind::Comment | Kind::Newline => {
                self.out.push(Token::Source(self.source_token()));
            }
            Kind::DocMode | Kind::DocStart => {
                let mut start = Vec::new();
                if self.kind == Kind::DocStart {
                    start.push(self.source_token());
                }
                self.stack.push(Token::Document {
                    offset: self.offset,
                    start,
                    value: None,
                    end: None,
                });
            }
            Kind::FlowErrorEnd
            | Kind::Scalar
            | Kind::DocEnd
            | Kind::SeqItemInd
            | Kind::ExplicitKeyInd
            | Kind::MapValueInd
            | Kind::FlowMapStart
            | Kind::FlowMapEnd
            | Kind::FlowSeqStart
            | Kind::FlowSeqEnd
            | Kind::Comma
            | Kind::Alias
            | Kind::Anchor
            | Kind::Tag
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => {
                let message = format!("Unexpected {} token in YAML stream", self.kind.name());
                self.out.push(self.error(message));
            }
        }
    }

    pub(super) fn document(&mut self) {
        let token = self.source_token();
        let Some(Token::Document { start, value, .. }) = self.stack.last_mut() else {
            return;
        };
        if value.is_some() {
            self.line_end();
            return;
        }
        match self.kind {
            Kind::DocStart if has_non_empty(start) => {
                self.pop(None);
                self.step();
                return;
            }
            Kind::DocStart
            | Kind::Anchor
            | Kind::Tag
            | Kind::Space
            | Kind::Comment
            | Kind::Newline => {
                start.push(token);
                return;
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::Scalar
            | Kind::DocEnd
            | Kind::SeqItemInd
            | Kind::ExplicitKeyInd
            | Kind::MapValueInd
            | Kind::FlowMapStart
            | Kind::FlowMapEnd
            | Kind::FlowSeqStart
            | Kind::FlowSeqEnd
            | Kind::Comma
            | Kind::DirectiveLine
            | Kind::Alias
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => {}
        }
        match self.start_block_value() {
            Some(value) => self.stack.push(value),
            None => {
                let message = format!("Unexpected {} token in YAML document", self.kind.name());
                self.out.push(self.error(message));
            }
        }
    }

    pub(super) fn document_end(&mut self) {
        if self.kind == Kind::DocMode {
            return;
        }
        let token = self.source_token();
        if let Some(Token::DocEnd { end, .. }) = self.stack.last_mut() {
            end.get_or_insert_with(Vec::new).push(token);
        }
        if self.kind == Kind::Newline {
            self.pop(None);
        }
    }

    pub(super) fn line_end(&mut self) {
        match self.kind {
            Kind::Comma
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::FlowSeqEnd
            | Kind::FlowMapEnd
            | Kind::MapValueInd => {
                self.pop(None);
                self.step();
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::Scalar
            | Kind::Newline
            | Kind::SeqItemInd
            | Kind::ExplicitKeyInd
            | Kind::FlowMapStart
            | Kind::FlowSeqStart
            | Kind::Space
            | Kind::Comment
            | Kind::DirectiveLine
            | Kind::Alias
            | Kind::Anchor
            | Kind::Tag
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => {
                if self.kind == Kind::Newline {
                    self.on_key_line = false;
                }
                // All other values are errors.
                let token = self.source_token();
                match self.stack.last_mut() {
                    Some(Token::FlowScalar(scalar)) => {
                        scalar.end.get_or_insert_with(Vec::new).push(token);
                    }
                    Some(Token::Document { end, .. } | Token::DocEnd { end, .. }) => {
                        end.get_or_insert_with(Vec::new).push(token);
                    }
                    Some(Token::FlowCollection { end, .. }) => end.push(token),
                    Some(_) | None => {}
                }
                if self.kind == Kind::Newline {
                    self.pop(None);
                }
            }
        }
    }
}
