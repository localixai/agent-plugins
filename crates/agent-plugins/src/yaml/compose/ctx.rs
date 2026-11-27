//! What every step of composing shares (the `ctx` of `compose/composer.js` with the `onError`
//! it passes down): where the node is, the document's directives, the errors and the node ids.

use crate::yaml::directives::Directives;
use crate::yaml::nodes::{Body, Node, YamlError};

/// What composing a node reads and writes: the document's directives, where it is, and the
/// errors so far.
pub(crate) struct Ctx<'a> {
    pub(crate) at_key: bool,
    pub(crate) at_root: bool,
    /// `options.strict`, which the reader leaves on.
    pub(crate) strict: bool,
    pub(crate) directives: &'a Directives,
    pub(super) errors: &'a mut Vec<YamlError>,
    pub(super) ids: &'a mut usize,
    pub(super) depth: usize,
}

impl Ctx<'_> {
    /// `onError` of an error (never a warning) at `offset`.
    pub(crate) fn error(&mut self, offset: usize, message: impl Into<String>) {
        self.errors.push(YamlError {
            offset,
            message: message.into(),
        });
    }

    /// A new node, with an id no other node of the parse has.
    pub(crate) fn node(&mut self, body: Body) -> Node {
        *self.ids += 1;
        let mut node = Node::new(body);
        node.id = *self.ids;
        node
    }
}
