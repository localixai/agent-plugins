//! The parser's scalars (`scalar` and `blockScalar` of `Parser.prototype`): a scalar that may
//! turn out to be a map's implicit key, and a block scalar's header and body.

use super::{Parser, first_key_start_props, prev_props};
use crate::yaml::cst::{Item, Kind, Token};

impl Parser {
    pub(super) fn scalar(&mut self) {
        if self.kind != Kind::MapValueInd {
            self.line_end();
            return;
        }
        let token = self.source_token();
        let parent = self.stack.len().checked_sub(2);
        let start = parent
            .and_then(|at| self.stack.get_mut(at))
            .and_then(prev_props)
            .map(first_key_start_props)
            .unwrap_or_default();
        let Some(Token::FlowScalar(mut scalar)) = self.stack.pop() else {
            return;
        };
        let mut sep = scalar.end.take().unwrap_or_default();
        sep.push(token);
        let map = Token::BlockMap {
            offset: scalar.offset,
            indent: scalar.indent,
            items: vec![Item {
                start,
                key: Some(Box::new(Token::FlowScalar(scalar))),
                sep: Some(sep),
                ..Item::default()
            }],
        };
        self.on_key_line = true;
        self.stack.push(map);
    }

    pub(super) fn block_scalar(&mut self) {
        let token = self.source_token();
        let Some(Token::BlockScalar { props, source, .. }) = self.stack.last_mut() else {
            return;
        };
        match self.kind {
            Kind::Space | Kind::Comment | Kind::Newline => props.push(Token::Source(token)),
            Kind::Scalar => {
                source.clone_from(&self.source);
                // A block scalar's source includes its trailing line break.
                self.at_new_line = true;
                self.indent = 0;
                self.note_line_starts();
                self.pop(None);
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::DocStart
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
            | Kind::Anchor
            | Kind::Tag
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => {
                self.pop(None);
                self.step();
            }
        }
    }
}
