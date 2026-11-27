//! The parser's block maps (`Parser.prototype.blockMap`): where each token goes in a map's
//! items, a key, its `:`, its value, or the start of the next item.

mod value;

use super::Parser;
use super::block::starting;
use crate::yaml::cst::{Item, Kind, SourceToken, Token};

impl Parser {
    pub(super) fn block_map(&mut self) {
        let token = self.source_token();
        let Some(Token::BlockMap { indent, items, .. }) = self.stack.last_mut() else {
            return;
        };
        let map_indent = *indent;
        let Some(item) = items.last_mut() else {
            return;
        };
        match self.kind {
            Kind::Newline => {
                self.on_key_line = false;
                if let Some(value) = &mut item.value {
                    match value.end_mut() {
                        Some(end) if end.last().is_some_and(|last| last.kind == Kind::Comment) => {
                            end.push(token);
                        }
                        Some(_) | None => items.push(starting(vec![token])),
                    }
                } else if let Some(sep) = &mut item.sep {
                    sep.push(token);
                } else {
                    item.start.push(token);
                }
                return;
            }
            Kind::Space | Kind::Comment => {
                if item.value.is_some() {
                    items.push(starting(vec![token]));
                } else if let Some(sep) = &mut item.sep {
                    sep.push(token);
                } else {
                    let indented = self.kind == Kind::Comment
                        && self.indent > map_indent
                        && item
                            .start
                            .iter()
                            .all(|st| matches!(st.kind, Kind::Newline | Kind::Space));
                    if !(indented && Self::onto_previous_end(items, token.clone()))
                        && let Some(item) = items.last_mut()
                    {
                        item.start.push(token);
                    }
                }
                return;
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::Scalar
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
            | Kind::BlockScalarHeader => {}
        }
        if self.indent >= map_indent && self.block_map_item(map_indent, token) {
            return;
        }
        self.pop(None);
        self.step();
    }

    /// The empty-node tokens of an item at the next item's indent: those after its second line
    /// break move to the next item (`blockMap`'s `start`).
    pub(super) fn next_item_start(
        sep: &mut Vec<SourceToken>,
        map_indent: usize,
    ) -> Vec<SourceToken> {
        let mut breaks: Vec<usize> = Vec::new();
        for (index, token) in sep.iter().enumerate() {
            match token.kind {
                Kind::Newline => breaks.push(index),
                Kind::Space => {}
                Kind::Comment if token.indent <= map_indent => {}
                Kind::ByteOrderMark
                | Kind::DocMode
                | Kind::FlowErrorEnd
                | Kind::Scalar
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
                | Kind::Comment
                | Kind::DirectiveLine
                | Kind::Alias
                | Kind::Anchor
                | Kind::Tag
                | Kind::SingleQuotedScalar
                | Kind::DoubleQuotedScalar
                | Kind::BlockScalarHeader => breaks.clear(),
            }
        }
        match breaks.get(1) {
            Some(&at) => sep.split_off(at),
            None => Vec::new(),
        }
    }

    /// A token at or past the map's indent that is no space, comment or line break; whether it
    /// was taken.
    pub(super) fn block_map_item(&mut self, map_indent: usize, token: SourceToken) -> bool {
        let at_map_indent = !self.on_key_line && self.indent == map_indent;
        let kind = self.kind;
        let Some(Token::BlockMap { items, .. }) = self.stack.last_mut() else {
            return false;
        };
        let Some(item) = items.last_mut() else {
            return false;
        };
        let at_next_item =
            at_map_indent && (item.sep.is_some() || item.explicit_key) && kind != Kind::SeqItemInd;
        let mut start = Vec::new();
        if at_next_item
            && item.value.is_none()
            && let Some(sep) = &mut item.sep
        {
            start = Self::next_item_start(sep, map_indent);
        }
        match kind {
            Kind::Anchor | Kind::Tag => {
                if at_next_item || item.value.is_some() {
                    start.push(token);
                    items.push(starting(start));
                    self.on_key_line = true;
                } else if let Some(sep) = &mut item.sep {
                    sep.push(token);
                } else {
                    item.start.push(token);
                }
                true
            }
            Kind::ExplicitKeyInd => {
                if item.sep.is_none() && !item.explicit_key {
                    item.start.push(token);
                    item.explicit_key = true;
                } else if at_next_item || item.value.is_some() {
                    start.push(token);
                    items.push(Item {
                        start,
                        explicit_key: true,
                        ..Item::default()
                    });
                } else {
                    let map = self.explicit_map(token);
                    self.stack.push(map);
                }
                self.on_key_line = true;
                true
            }
            Kind::MapValueInd => {
                self.map_value(at_next_item, start, token);
                self.on_key_line = true;
                true
            }
            Kind::Alias | Kind::Scalar | Kind::SingleQuotedScalar | Kind::DoubleQuotedScalar => {
                let scalar = self.flow_scalar();
                self.scalar_in_map(at_next_item, start, scalar);
                true
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::Newline
            | Kind::SeqItemInd
            | Kind::FlowMapStart
            | Kind::FlowMapEnd
            | Kind::FlowSeqStart
            | Kind::FlowSeqEnd
            | Kind::Comma
            | Kind::Space
            | Kind::Comment
            | Kind::DirectiveLine
            | Kind::BlockScalarHeader => self.block_value_in_map(at_map_indent, start),
        }
    }
}
