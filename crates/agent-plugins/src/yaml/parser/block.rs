//! The parser's block sequences, and how a finished token is set into its parent (`pop`).

use super::{Parser, has_non_empty, includes};
use crate::yaml::cst::{Item, Kind, SourceToken, Token};

/// Whether the token ends its parent's last item with only blank lines and comments that belong
/// after it: those are moved out to the parent (`pop`'s last step).
fn split_trailing(token: &mut Token) -> Option<Vec<SourceToken>> {
    let (Token::BlockMap { indent, items, .. } | Token::BlockSeq { indent, items, .. }) = token
    else {
        return None;
    };
    let indent = *indent;
    let last = items.last()?;
    let moves = last.sep.is_none()
        && last.value.is_none()
        && !last.start.is_empty()
        && !has_non_empty(&last.start)
        && (indent == 0
            || last
                .start
                .iter()
                .all(|token| token.kind != Kind::Comment || token.indent < indent));
    if moves {
        items.pop().map(|item| item.start)
    } else {
        None
    }
}

/// An item holding only these leading tokens.
pub(super) fn starting(start: Vec<SourceToken>) -> Item {
    Item {
        start,
        ..Item::default()
    }
}

/// An item holding `key` and an empty separator.
pub(super) fn keyed(start: Vec<SourceToken>, key: Token) -> Item {
    Item {
        start,
        key: Some(Box::new(key)),
        sep: Some(Vec::new()),
        ..Item::default()
    }
}

impl Parser {
    /// Sets a finished token into the node on top of the stack.
    pub(super) fn attach(&mut self, mut token: Token) {
        let Some(top) = self.stack.last_mut() else {
            return;
        };
        let top_indent = top.indent().unwrap_or(0);
        let top_is_document = matches!(top, Token::Document { .. });
        match &mut token {
            // Block scalars use their parent's indent rather than the header's.
            Token::BlockScalar { indent, .. } => *indent = top_indent,
            Token::FlowCollection { indent, .. } => {
                // A top-level flow collection's indent is ignored.
                if top_is_document {
                    *indent = 0;
                }
                super::flow::fix_flow_seq_items(&mut token);
            }
            Token::Source(_)
            | Token::Error(_)
            | Token::Directive { .. }
            | Token::Document { .. }
            | Token::DocEnd { .. }
            | Token::FlowScalar(_)
            | Token::BlockMap { .. }
            | Token::BlockSeq { .. } => {}
        }
        match top {
            Token::Document { value, end, .. } => {
                let trailing = split_trailing(&mut token);
                *value = Some(Box::new(token));
                if let Some(start) = trailing {
                    *end = Some(start);
                }
            }
            // An error.
            Token::BlockScalar { props, .. } => props.push(token),
            Token::BlockMap { items, .. } => {
                let Some(item) = items.last_mut() else {
                    return;
                };
                if item.value.is_some() {
                    items.push(keyed(Vec::new(), token));
                    self.on_key_line = true;
                } else if item.sep.is_some() {
                    let trailing = split_trailing(&mut token);
                    item.value = Some(Box::new(token));
                    if let Some(start) = trailing {
                        items.push(starting(start));
                    }
                } else {
                    item.key = Some(Box::new(token));
                    item.sep = Some(Vec::new());
                    self.on_key_line = !item.explicit_key;
                }
            }
            Token::BlockSeq { items, .. } => {
                let trailing = split_trailing(&mut token);
                match items.last_mut() {
                    Some(item) if item.value.is_none() => item.value = Some(Box::new(token)),
                    Some(_) | None => items.push(Item {
                        value: Some(Box::new(token)),
                        ..Item::default()
                    }),
                }
                if let Some(start) = trailing {
                    items.push(starting(start));
                }
            }
            Token::FlowCollection { items, .. } => match items.last_mut() {
                Some(item) if item.value.is_none() => {
                    if item.sep.is_some() {
                        item.value = Some(Box::new(token));
                    } else {
                        item.key = Some(Box::new(token));
                        item.sep = Some(Vec::new());
                    }
                }
                Some(_) | None => items.push(keyed(Vec::new(), token)),
            },
            Token::Source(_)
            | Token::Error(_)
            | Token::Directive { .. }
            | Token::DocEnd { .. }
            | Token::FlowScalar(_) => {
                self.pop(None);
                self.pop(Some(token));
            }
        }
    }

    /// An indented comment after an item's value goes to the end of that value, taking the
    /// current item's tokens with it; whether it did.
    pub(super) fn onto_previous_end(items: &mut Vec<Item>, token: SourceToken) -> bool {
        let count = items.len();
        let Some(current) = items.last().map(|item| item.start.clone()) else {
            return false;
        };
        let previous = count.checked_sub(2).and_then(|at| items.get_mut(at));
        let Some(end) = previous
            .and_then(|item| item.value.as_mut())
            .and_then(|value| value.end_mut())
        else {
            return false;
        };
        end.extend(current);
        end.push(token);
        items.pop();
        true
    }

    pub(super) fn block_sequence(&mut self) {
        let token = self.source_token();
        let Some(Token::BlockSeq { indent, items, .. }) = self.stack.last_mut() else {
            return;
        };
        let seq_indent = *indent;
        let Some(item) = items.last_mut() else {
            return;
        };
        match self.kind {
            Kind::Newline => {
                if let Some(value) = &mut item.value {
                    match value.end_mut() {
                        Some(end) if end.last().is_some_and(|last| last.kind == Kind::Comment) => {
                            end.push(token);
                        }
                        Some(_) | None => items.push(starting(vec![token])),
                    }
                } else {
                    item.start.push(token);
                }
                return;
            }
            Kind::Space | Kind::Comment => {
                if item.value.is_some() {
                    items.push(starting(vec![token]));
                } else {
                    let indented = self.kind == Kind::Comment
                        && self.indent > seq_indent
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
            Kind::Anchor | Kind::Tag if item.value.is_none() && self.indent > seq_indent => {
                item.start.push(token);
                return;
            }
            Kind::SeqItemInd if self.indent == seq_indent => {
                if item.value.is_some() || includes(&item.start, Kind::SeqItemInd) {
                    items.push(starting(vec![token]));
                } else {
                    item.start.push(token);
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
        if self.indent > seq_indent
            && let Some(value) = self.start_block_value()
        {
            self.stack.push(value);
            return;
        }
        self.pop(None);
        self.step();
    }
}
