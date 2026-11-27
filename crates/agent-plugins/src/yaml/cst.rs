//! The concrete syntax tree of `yaml` (`dist/parse/cst.js` and its token shapes): what the parser
//! builds from the lexer's tokens and the composer reads.

use super::units::Units;

/// The type of a source token, named as the package names it (its error messages print it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    ByteOrderMark,
    DocMode,
    FlowErrorEnd,
    Scalar,
    DocStart,
    DocEnd,
    Newline,
    SeqItemInd,
    ExplicitKeyInd,
    MapValueInd,
    FlowMapStart,
    FlowMapEnd,
    FlowSeqStart,
    FlowSeqEnd,
    Comma,
    Space,
    Comment,
    DirectiveLine,
    Alias,
    Anchor,
    Tag,
    SingleQuotedScalar,
    DoubleQuotedScalar,
    BlockScalarHeader,
}

impl Kind {
    /// The package's name of the type.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::ByteOrderMark => "byte-order-mark",
            Self::DocMode => "doc-mode",
            Self::FlowErrorEnd => "flow-error-end",
            Self::Scalar => "scalar",
            Self::DocStart => "doc-start",
            Self::DocEnd => "doc-end",
            Self::Newline => "newline",
            Self::SeqItemInd => "seq-item-ind",
            Self::ExplicitKeyInd => "explicit-key-ind",
            Self::MapValueInd => "map-value-ind",
            Self::FlowMapStart => "flow-map-start",
            Self::FlowMapEnd => "flow-map-end",
            Self::FlowSeqStart => "flow-seq-start",
            Self::FlowSeqEnd => "flow-seq-end",
            Self::Comma => "comma",
            Self::Space => "space",
            Self::Comment => "comment",
            Self::DirectiveLine => "directive-line",
            Self::Alias => "alias",
            Self::Anchor => "anchor",
            Self::Tag => "tag",
            Self::SingleQuotedScalar => "single-quoted-scalar",
            Self::DoubleQuotedScalar => "double-quoted-scalar",
            Self::BlockScalarHeader => "block-scalar-header",
        }
    }
}

/// `tokenType`: the type of a lexical token's text; `None` for text that is no YAML token.
pub(crate) fn token_type(source: &[u16]) -> Option<Kind> {
    let whole = match String::from_utf16_lossy(source).as_str() {
        "\u{FEFF}" => Some(Kind::ByteOrderMark),
        "---" => Some(Kind::DocStart),
        "..." => Some(Kind::DocEnd),
        "" | "\n" | "\r\n" => Some(Kind::Newline),
        "-" => Some(Kind::SeqItemInd),
        "?" => Some(Kind::ExplicitKeyInd),
        ":" => Some(Kind::MapValueInd),
        "{" => Some(Kind::FlowMapStart),
        "}" => Some(Kind::FlowMapEnd),
        "[" => Some(Kind::FlowSeqStart),
        "]" => Some(Kind::FlowSeqEnd),
        "," => Some(Kind::Comma),
        _ => None,
    };
    whole.or_else(|| {
        let first = char::from_u32(u32::from(*source.first()?))?;
        match first {
            ' ' | '\t' => Some(Kind::Space),
            '#' => Some(Kind::Comment),
            '%' => Some(Kind::DirectiveLine),
            '*' => Some(Kind::Alias),
            '&' => Some(Kind::Anchor),
            '!' => Some(Kind::Tag),
            '\'' => Some(Kind::SingleQuotedScalar),
            '"' => Some(Kind::DoubleQuotedScalar),
            '|' | '>' => Some(Kind::BlockScalarHeader),
            _ => None,
        }
    })
}

/// A token as the lexer gave it, with where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceToken {
    pub(crate) kind: Kind,
    pub(crate) offset: usize,
    pub(crate) indent: usize,
    pub(crate) source: Units,
}

/// An error the parser met: `{ type: 'error', offset, message, source }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ErrorToken {
    pub(crate) offset: usize,
    pub(crate) message: String,
    pub(crate) source: Units,
}

/// An item of a collection: its leading tokens, and the key, separator and value it holds.
/// `sep` is there (even empty) once the item has a key or a `:`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Item {
    pub(crate) start: Vec<SourceToken>,
    pub(crate) key: Option<Box<Token>>,
    pub(crate) sep: Option<Vec<SourceToken>>,
    pub(crate) value: Option<Box<Token>>,
    pub(crate) explicit_key: bool,
}

/// A scalar on one or more lines, or an alias: `alias`, `scalar`, `single-quoted-scalar` or
/// `double-quoted-scalar`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowScalar {
    pub(crate) kind: Kind,
    pub(crate) offset: usize,
    pub(crate) indent: usize,
    pub(crate) source: Units,
    pub(crate) end: Option<Vec<SourceToken>>,
}

/// A token of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    /// A source token outside any node (or a block scalar's header and what follows it).
    Source(SourceToken),
    Error(ErrorToken),
    Directive {
        offset: usize,
        source: Units,
    },
    Document {
        offset: usize,
        start: Vec<SourceToken>,
        value: Option<Box<Token>>,
        end: Option<Vec<SourceToken>>,
    },
    DocEnd {
        offset: usize,
        source: Units,
        end: Option<Vec<SourceToken>>,
    },
    FlowScalar(FlowScalar),
    BlockScalar {
        offset: usize,
        indent: usize,
        props: Vec<Token>,
        source: Units,
    },
    BlockMap {
        offset: usize,
        indent: usize,
        items: Vec<Item>,
    },
    BlockSeq {
        offset: usize,
        indent: usize,
        items: Vec<Item>,
    },
    FlowCollection {
        offset: usize,
        indent: usize,
        start: SourceToken,
        items: Vec<Item>,
        end: Vec<SourceToken>,
    },
}

/// Drops a token tree without recursing: each collection's keys and values are taken out onto a
/// list and dropped from there. Dropped whole, a tree recurses once per level, and a document
/// nested a few thousand levels deep (which the parser builds, and the composer refuses past
/// `MAX_DEPTH`) would exhaust WebAssembly's stack (RS4).
pub(crate) fn dismantle(token: Token) {
    let mut pending = vec![token];
    while let Some(mut token) = pending.pop() {
        match &mut token {
            Token::Document { value, .. } => pending.extend(value.take().map(|value| *value)),
            Token::BlockMap { items, .. }
            | Token::BlockSeq { items, .. }
            | Token::FlowCollection { items, .. } => {
                for item in items.drain(..) {
                    pending.extend(item.key.map(|key| *key));
                    pending.extend(item.value.map(|value| *value));
                }
            }
            Token::Source(_)
            | Token::Error(_)
            | Token::Directive { .. }
            | Token::DocEnd { .. }
            | Token::FlowScalar(_)
            | Token::BlockScalar { .. } => {}
        }
    }
}

impl Token {
    /// The package's `type` of the token.
    pub(crate) const fn type_name(&self) -> &'static str {
        match self {
            Self::Source(token) => token.kind.name(),
            Self::Error(_) => "error",
            Self::Directive { .. } => "directive",
            Self::Document { .. } => "document",
            Self::DocEnd { .. } => "doc-end",
            Self::FlowScalar(scalar) => scalar.kind.name(),
            Self::BlockScalar { .. } => "block-scalar",
            Self::BlockMap { .. } => "block-map",
            Self::BlockSeq { .. } => "block-seq",
            Self::FlowCollection { .. } => "flow-collection",
        }
    }

    pub(crate) const fn offset(&self) -> usize {
        match self {
            Self::Source(SourceToken { offset, .. })
            | Self::Error(ErrorToken { offset, .. })
            | Self::Directive { offset, .. }
            | Self::Document { offset, .. }
            | Self::DocEnd { offset, .. }
            | Self::FlowScalar(FlowScalar { offset, .. })
            | Self::BlockScalar { offset, .. }
            | Self::BlockMap { offset, .. }
            | Self::BlockSeq { offset, .. }
            | Self::FlowCollection { offset, .. } => *offset,
        }
    }

    /// `token.source` when it is a text: compound tokens have none.
    pub(crate) fn source(&self) -> Option<&Units> {
        match self {
            Self::Source(SourceToken { source, .. })
            | Self::Error(ErrorToken { source, .. })
            | Self::Directive { source, .. }
            | Self::DocEnd { source, .. }
            | Self::FlowScalar(FlowScalar { source, .. })
            | Self::BlockScalar { source, .. } => Some(source),
            Self::Document { .. }
            | Self::BlockMap { .. }
            | Self::BlockSeq { .. }
            | Self::FlowCollection { .. } => None,
        }
    }

    /// `'indent' in token ? token.indent : undefined`.
    pub(crate) const fn indent(&self) -> Option<usize> {
        match self {
            Self::Source(SourceToken { indent, .. })
            | Self::FlowScalar(FlowScalar { indent, .. })
            | Self::BlockScalar { indent, .. }
            | Self::BlockMap { indent, .. }
            | Self::BlockSeq { indent, .. }
            | Self::FlowCollection { indent, .. } => Some(*indent),
            Self::Error(_)
            | Self::Directive { .. }
            | Self::Document { .. }
            | Self::DocEnd { .. } => None,
        }
    }

    /// The token's `end` array when it has one (`Array.isArray(token.end)`).
    pub(crate) const fn end_mut(&mut self) -> Option<&mut Vec<SourceToken>> {
        match self {
            Self::FlowScalar(FlowScalar { end: Some(end), .. })
            | Self::Document { end: Some(end), .. }
            | Self::DocEnd { end: Some(end), .. }
            | Self::FlowCollection { end, .. } => Some(end),
            Self::FlowScalar(FlowScalar { end: None, .. })
            | Self::Document { end: None, .. }
            | Self::DocEnd { end: None, .. }
            | Self::Source(_)
            | Self::Error(_)
            | Self::Directive { .. }
            | Self::BlockScalar { .. }
            | Self::BlockMap { .. }
            | Self::BlockSeq { .. } => None,
        }
    }

    /// Whether it is a flow token (`isFlowToken`): an alias, a flow scalar or a flow collection.
    pub(crate) const fn is_flow(&self) -> bool {
        matches!(self, Self::FlowScalar(_) | Self::FlowCollection { .. })
    }
}
