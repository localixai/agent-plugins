//! `yaml`'s document model (`dist/nodes/`, `dist/doc/Document.js`): scalars, maps, sequences,
//! aliases and pairs, each with the comments, blank lines and properties the writer keeps.

use super::directives::Directives;
use crate::js::JsValue;

/// How a scalar was written (`Scalar.BLOCK_FOLDED` and the others).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScalarType {
    BlockFolded,
    BlockLiteral,
    Plain,
    QuoteDouble,
    QuoteSingle,
}

/// The format of a number's tag: octal, hexadecimal or with an exponent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Format {
    Oct,
    Hex,
    Exp,
}

/// A scalar node's own fields.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Scalar {
    /// Its value: `null`, a boolean, a number or a string.
    pub(crate) value: JsValue,
    /// The text it was read from (after unescaping and folding), for the writer.
    pub(crate) source: Option<String>,
    pub(crate) kind: Option<ScalarType>,
    pub(crate) format: Option<Format>,
    /// The decimals a float read as `1.50` is written back with.
    pub(crate) min_fraction_digits: usize,
}

/// What a node holds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Body {
    Scalar(Scalar),
    /// A map's pairs; `flow` when it is written `{...}`.
    Map {
        items: Vec<Pair>,
        flow: bool,
    },
    /// A sequence's items; `flow` when it is written `[...]`.
    Seq {
        items: Vec<Node>,
        flow: bool,
    },
    /// An alias of the anchor named `source`.
    Alias(String),
}

/// A node of a document.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Node {
    /// Which node it is, for an alias's anchor (JavaScript compares the objects).
    pub(crate) id: usize,
    /// Where it starts, where its value ends, and where the node ends with its comment.
    pub(crate) range: [usize; 3],
    pub(crate) comment: Option<String>,
    pub(crate) comment_before: Option<String>,
    pub(crate) space_before: bool,
    pub(crate) anchor: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) body: Body,
}

/// A pair's key: a node, or the text `map.set` was given.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Key {
    Node(Box<Node>),
    Text(String),
}

/// A map's pair; a missing value is `null`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Pair {
    pub(crate) key: Key,
    pub(crate) value: Option<Node>,
}

/// A YAML problem: what the package's `YAMLParseError` holds that the reader reads, its message
/// and where it starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct YamlError {
    pub(crate) offset: usize,
    pub(crate) message: String,
}

/// A document.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Document {
    pub(crate) contents: Option<Node>,
    pub(crate) comment_before: Option<String>,
    pub(crate) comment: Option<String>,
    pub(crate) directives: Directives,
    pub(crate) errors: Vec<YamlError>,
    pub(crate) range: [usize; 3],
}

impl Document {
    /// `new Document(value)`: a document of these contents, with the stream's first directives.
    pub(crate) fn new(contents: Option<Node>) -> Self {
        Self {
            contents,
            comment_before: None,
            comment: None,
            directives: Directives::new(),
            errors: Vec::new(),
            range: [0; 3],
        }
    }
}

/// `a ? a + sep + b : b` for a comment: `b` appended to `a` after `sep`, or alone.
pub(crate) fn append(comment: &mut Option<String>, sep: &str, more: &str) {
    match comment {
        Some(text) if !text.is_empty() => {
            text.push_str(sep);
            text.push_str(more);
        }
        Some(_) | None => *comment = Some(more.to_owned()),
    }
}

impl Node {
    /// A node of this body with no properties, comments or place.
    pub(crate) const fn new(body: Body) -> Self {
        Self {
            id: 0,
            range: [0; 3],
            comment: None,
            comment_before: None,
            space_before: false,
            anchor: None,
            tag: None,
            body,
        }
    }

    /// A scalar node of a value.
    pub(crate) const fn scalar(value: JsValue) -> Self {
        Self::new(Body::Scalar(Scalar {
            value,
            source: None,
            kind: None,
            format: None,
            min_fraction_digits: 0,
        }))
    }

    pub(crate) const fn as_scalar(&self) -> Option<&Scalar> {
        match &self.body {
            Body::Scalar(scalar) => Some(scalar),
            Body::Map { .. } | Body::Seq { .. } | Body::Alias(_) => None,
        }
    }

    /// `hasAnchor`: a scalar or a collection with an anchor.
    pub(crate) fn has_anchor(&self) -> bool {
        !matches!(self.body, Body::Alias(_)) && self.anchor.as_ref().is_some_and(|a| !a.is_empty())
    }
}

impl Pair {
    /// `findPair`'s test: whether the pair's key is `key` (a scalar key by its value).
    fn has_key(&self, key: &str) -> bool {
        match &self.key {
            Key::Text(text) => text == key,
            Key::Node(node) => node
                .as_scalar()
                .is_some_and(|scalar| scalar.value.as_str() == Some(key)),
        }
    }
}

/// `YAMLMap.prototype.set(key, value)`: a key the map has gets the new value in its place, a new
/// one goes last.
pub(crate) fn map_set(items: &mut Vec<Pair>, key: &str, value: Node) {
    match items.iter_mut().find(|pair| pair.has_key(key)) {
        Some(pair) => pair.value = Some(value),
        None => items.push(Pair {
            key: Key::Text(key.to_owned()),
            value: Some(value),
        }),
    }
}
