//! What follows a block map's key (the value branches of `Parser.prototype.blockMap`): an
//! explicit map, a scalar or a block value set as the item's key or value, and the `:`.

use crate::yaml::cst::{Item, Kind, SourceToken, Token};
use crate::yaml::parser::block::{keyed, starting};
use crate::yaml::parser::{Parser, first_key_start_props, includes};

impl Parser {
    pub(super) fn explicit_map(&self, token: SourceToken) -> Token {
        Token::BlockMap {
            offset: self.offset,
            indent: self.indent,
            items: vec![Item {
                start: vec![token],
                explicit_key: true,
                ..Item::default()
            }],
        }
    }

    pub(super) fn scalar_in_map(
        &mut self,
        at_next_item: bool,
        start: Vec<SourceToken>,
        scalar: Token,
    ) {
        let Some(Token::BlockMap { items, .. }) = self.stack.last_mut() else {
            return;
        };
        let Some(item) = items.last_mut() else {
            return;
        };
        if at_next_item || item.value.is_some() {
            items.push(keyed(start, scalar));
            self.on_key_line = true;
        } else if item.sep.is_some() {
            self.stack.push(scalar);
        } else {
            item.key = Some(Box::new(scalar));
            item.sep = Some(Vec::new());
            self.on_key_line = true;
        }
    }

    pub(super) fn block_value_in_map(
        &mut self,
        at_map_indent: bool,
        start: Vec<SourceToken>,
    ) -> bool {
        let Some(value) = self.start_block_value() else {
            return false;
        };
        let Some(Token::BlockMap { items, .. }) = self.stack.last_mut() else {
            return false;
        };
        if matches!(value, Token::BlockSeq { .. }) {
            let same_line = items.last().is_some_and(|item| {
                !item.explicit_key
                    && item
                        .sep
                        .as_ref()
                        .is_some_and(|sep| !includes(sep, Kind::Newline))
            });
            if same_line {
                let error = self.error("Unexpected block-seq-ind on same line with key".to_owned());
                self.pop(Some(error));
                return true;
            }
        } else if at_map_indent {
            items.push(starting(start));
        }
        self.stack.push(value);
        true
    }

    pub(super) fn map_value(
        &mut self,
        at_next_item: bool,
        start: Vec<SourceToken>,
        token: SourceToken,
    ) {
        let (offset, indent) = (self.offset, self.indent);
        let Some(Token::BlockMap { items, .. }) = self.stack.last_mut() else {
            return;
        };
        let Some(item) = items.last_mut() else {
            return;
        };
        let new_map = |start: Vec<SourceToken>, key: Option<Box<Token>>, sep: Vec<SourceToken>| {
            Token::BlockMap {
                offset,
                indent,
                items: vec![Item {
                    start,
                    key,
                    sep: Some(sep),
                    ..Item::default()
                }],
            }
        };
        let empty_key = |start: Vec<SourceToken>, token: SourceToken| Item {
            start,
            sep: Some(vec![token]),
            ..Item::default()
        };
        let pushed = if item.explicit_key {
            match &mut item.sep {
                None if includes(&item.start, Kind::Newline) => {
                    item.key = None;
                    item.sep = Some(vec![token]);
                    None
                }
                None => Some(new_map(
                    first_key_start_props(&mut item.start),
                    None,
                    vec![token],
                )),
                Some(_) if item.value.is_some() => {
                    items.push(empty_key(Vec::new(), token));
                    None
                }
                Some(sep) if includes(sep, Kind::MapValueInd) => {
                    Some(new_map(start, None, vec![token]))
                }
                Some(sep)
                    if item.key.as_ref().is_some_and(|key| key.is_flow())
                        && !includes(sep, Kind::Newline) =>
                {
                    let start = first_key_start_props(&mut item.start);
                    let key = item.key.take();
                    let mut sep = item.sep.take().unwrap_or_default();
                    sep.push(token);
                    Some(new_map(start, key, sep))
                }
                // Not actually at the next item.
                Some(sep) => {
                    sep.extend(start);
                    sep.push(token);
                    None
                }
            }
        } else {
            match &mut item.sep {
                None => {
                    item.key = None;
                    item.sep = Some(vec![token]);
                    None
                }
                Some(_) if item.value.is_some() || at_next_item => {
                    items.push(empty_key(start, token));
                    None
                }
                Some(sep) if includes(sep, Kind::MapValueInd) => {
                    Some(new_map(Vec::new(), None, vec![token]))
                }
                Some(sep) => {
                    sep.push(token);
                    None
                }
            }
        };
        if let Some(map) = pushed {
            self.stack.push(map);
        }
    }
}
