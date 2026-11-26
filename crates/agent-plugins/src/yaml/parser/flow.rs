//! The parser's flow collections and the start of every block value.

use super::{Parser, first_key_start_props, includes, prev_props};
use crate::yaml::cst::{FlowScalar, Item, Kind, SourceToken, Token};

/// `fixFlowSeqItems`: in a flow sequence, an item with a separator but no `?` or `:` holds a
/// value, not a key.
pub(super) fn fix_flow_seq_items(token: &mut Token) {
    let Token::FlowCollection { start, items, .. } = token else {
        return;
    };
    if start.kind != Kind::FlowSeqStart {
        return;
    }
    for item in items {
        let Some(sep) = &item.sep else {
            continue;
        };
        if item.value.is_some()
            || includes(&item.start, Kind::ExplicitKeyInd)
            || includes(sep, Kind::MapValueInd)
        {
            continue;
        }
        let sep = item.sep.take().unwrap_or_default();
        if item.key.is_some() {
            item.value = item.key.take();
        }
        match item.value.as_deref_mut() {
            Some(Token::FlowScalar(FlowScalar { end, .. })) => {
                end.get_or_insert_with(Vec::new).extend(sep);
            }
            Some(Token::FlowCollection { end, .. }) => end.extend(sep),
            Some(_) | None => item.start.extend(sep),
        }
    }
}

/// An item holding only `start`.
fn starting(start: Vec<SourceToken>) -> Item {
    Item {
        start,
        ..Item::default()
    }
}

impl Parser {
    pub(super) fn flow_collection(&mut self) {
        let token = self.source_token();
        if self.kind == Kind::FlowErrorEnd {
            loop {
                self.pop(None);
                if !matches!(self.stack.last(), Some(Token::FlowCollection { .. })) {
                    break;
                }
            }
            return;
        }
        let Some(Token::FlowCollection { end, .. }) = self.stack.last() else {
            return;
        };
        if end.is_empty() {
            self.flow_item(token);
        } else {
            self.after_flow_collection(token);
        }
    }

    /// A token inside a flow collection that is not closed yet.
    fn flow_item(&mut self, token: SourceToken) {
        let kind = self.kind;
        let scalar = matches!(
            kind,
            Kind::Alias | Kind::Scalar | Kind::SingleQuotedScalar | Kind::DoubleQuotedScalar
        )
        .then(|| self.flow_scalar());
        let Some(Token::FlowCollection { items, end, .. }) = self.stack.last_mut() else {
            return;
        };
        let open = items.last().filter(|item| item.value.is_none());
        let has_sep = open.is_some_and(|item| item.sep.is_some());
        let open = open.is_some();
        match kind {
            Kind::Comma | Kind::ExplicitKeyInd => {
                match items.last_mut() {
                    Some(item) if item.sep.is_none() => item.start.push(token),
                    Some(_) | None => items.push(starting(vec![token])),
                }
                return;
            }
            Kind::MapValueInd => {
                match items.last_mut() {
                    Some(item) if open && has_sep => {
                        if let Some(sep) = &mut item.sep {
                            sep.push(token);
                        }
                    }
                    Some(item) if open => {
                        item.key = None;
                        item.sep = Some(vec![token]);
                    }
                    Some(_) | None => items.push(Item {
                        sep: Some(vec![token]),
                        ..Item::default()
                    }),
                }
                return;
            }
            Kind::Space | Kind::Comment | Kind::Newline | Kind::Anchor | Kind::Tag => {
                match items.last_mut() {
                    Some(item) if open && has_sep => {
                        if let Some(sep) = &mut item.sep {
                            sep.push(token);
                        }
                    }
                    Some(item) if open => item.start.push(token),
                    Some(_) | None => items.push(starting(vec![token])),
                }
                return;
            }
            Kind::FlowMapEnd | Kind::FlowSeqEnd => {
                end.push(token);
                return;
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::Scalar
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::SeqItemInd
            | Kind::FlowMapStart
            | Kind::FlowSeqStart
            | Kind::DirectiveLine
            | Kind::Alias
            | Kind::SingleQuotedScalar
            | Kind::DoubleQuotedScalar
            | Kind::BlockScalarHeader => {}
        }
        if let Some(scalar) = scalar {
            match items.last_mut() {
                Some(_) if open && has_sep => self.stack.push(scalar),
                Some(item) if open => {
                    item.key = Some(Box::new(scalar));
                    item.sep = Some(Vec::new());
                }
                Some(_) | None => items.push(Item {
                    key: Some(Box::new(scalar)),
                    sep: Some(Vec::new()),
                    ..Item::default()
                }),
            }
            return;
        }
        match self.start_block_value() {
            Some(value) => self.stack.push(value),
            None => {
                self.pop(None);
                self.step();
            }
        }
    }

    /// A token after the flow collection's closing bracket.
    fn after_flow_collection(&mut self, token: SourceToken) {
        let kind = self.kind;
        let count = self.stack.len();
        let parent = count.checked_sub(2).and_then(|at| self.stack.get(at));
        let fc_indent = self.stack.last().and_then(Token::indent).unwrap_or(0);
        let ends_in_map = match parent {
            Some(Token::BlockMap { indent, items, .. }) => {
                (kind == Kind::MapValueInd && *indent == fc_indent)
                    || (kind == Kind::Newline
                        && items.last().is_some_and(|item| item.sep.is_none()))
            }
            Some(_) | None => false,
        };
        if ends_in_map {
            self.pop(None);
            self.step();
            return;
        }
        let in_flow = matches!(parent, Some(Token::FlowCollection { .. }));
        if kind != Kind::MapValueInd || parent.is_none() || in_flow {
            self.line_end();
            return;
        }
        let start = count
            .checked_sub(2)
            .and_then(|at| self.stack.get_mut(at))
            .and_then(prev_props)
            .map(first_key_start_props)
            .unwrap_or_default();
        let Some(mut collection) = self.stack.pop() else {
            return;
        };
        fix_flow_seq_items(&mut collection);
        let Token::FlowCollection {
            offset,
            indent,
            end,
            ..
        } = &mut collection
        else {
            return;
        };
        let (offset, indent) = (*offset, *indent);
        let mut sep = if end.is_empty() {
            Vec::new()
        } else {
            end.split_off(1)
        };
        sep.push(token);
        self.on_key_line = true;
        self.stack.push(Token::BlockMap {
            offset,
            indent,
            items: vec![Item {
                start,
                key: Some(Box::new(collection)),
                sep: Some(sep),
                ..Item::default()
            }],
        });
    }

    /// A scalar or an alias token at the current position, its line breaks noted.
    pub(super) fn flow_scalar(&mut self) -> Token {
        self.note_line_starts();
        Token::FlowScalar(FlowScalar {
            kind: self.kind,
            offset: self.offset,
            indent: self.indent,
            source: self.source.clone(),
            end: None,
        })
    }

    /// `startBlockValue`: the node the current token starts, its properties taken from the node
    /// on top of the stack; `None` when it starts none.
    pub(super) fn start_block_value(&mut self) -> Option<Token> {
        let (offset, indent) = (self.offset, self.indent);
        let token = self.source_token();
        match self.kind {
            Kind::Alias | Kind::Scalar | Kind::SingleQuotedScalar | Kind::DoubleQuotedScalar => {
                Some(self.flow_scalar())
            }
            Kind::BlockScalarHeader => Some(Token::BlockScalar {
                offset,
                indent,
                props: vec![Token::Source(token)],
                source: Vec::new(),
            }),
            Kind::FlowMapStart | Kind::FlowSeqStart => Some(Token::FlowCollection {
                offset,
                indent,
                start: token,
                items: Vec::new(),
                end: Vec::new(),
            }),
            Kind::SeqItemInd => Some(Token::BlockSeq {
                offset,
                indent,
                items: vec![starting(vec![token])],
            }),
            Kind::ExplicitKeyInd | Kind::MapValueInd => {
                self.on_key_line = true;
                let mut start = self
                    .stack
                    .last_mut()
                    .and_then(prev_props)
                    .map(first_key_start_props)
                    .unwrap_or_default();
                let item = if self.kind == Kind::ExplicitKeyInd {
                    start.push(token);
                    Item {
                        start,
                        explicit_key: true,
                        ..Item::default()
                    }
                } else {
                    Item {
                        start,
                        sep: Some(vec![token]),
                        ..Item::default()
                    }
                };
                Some(Token::BlockMap {
                    offset,
                    indent,
                    items: vec![item],
                })
            }
            Kind::ByteOrderMark
            | Kind::DocMode
            | Kind::FlowErrorEnd
            | Kind::DocStart
            | Kind::DocEnd
            | Kind::Newline
            | Kind::FlowMapEnd
            | Kind::FlowSeqEnd
            | Kind::Comma
            | Kind::Space
            | Kind::Comment
            | Kind::DirectiveLine
            | Kind::Anchor
            | Kind::Tag => None,
        }
    }
}
