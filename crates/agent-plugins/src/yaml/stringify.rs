//! The writer of `yaml` (`dist/stringify/stringify.js`, `Alias.prototype.toString` and
//! `anchorIsValid` of `dist/doc/anchors.js`): a document as text, with the options the
//! frontmatter writer passes (`lineWidth: 0`) and the package's defaults for the rest.
//!
//! The package's callbacks `onComment` and `onChompKeep` only set a flag in their caller, so a
//! call here returns the flags it would have set in [`Out`].

mod collection;
mod comment;
mod document;
mod fold;
mod pair;
mod string;

use std::collections::HashSet;

use self::collection::stringify_collection;
pub(crate) use self::comment::{indent_comment, line_comment, stringify_comment};
pub(crate) use self::document::stringify_document;
use self::pair::stringify_pair;
use self::string::stringify_string;
use super::nodes::{Body, Document, Node, Pair};
use super::tags::{CORE, Tag, stringify_int, stringify_number};
use crate::js::{JsValue, is_whitespace, to_exponential};

/// `createStringifyContext` and the members the writer adds to its copies. The anchors written
/// so far, which every copy shares (`ctx.anchors`), are passed beside it as `anchors`.
#[derive(Debug, Clone)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's context members, one each"
)]
pub(crate) struct Context<'a> {
    pub(crate) doc: &'a Document,
    pub(crate) indent: String,
    pub(crate) in_flow: bool,
    pub(crate) implicit_key: bool,
    pub(crate) all_null_values: bool,
    pub(crate) indent_at_start: Option<usize>,
    pub(crate) force_block_indent: bool,
    pub(crate) actual_string: bool,
    /// `options.lineWidth`: 0 from the frontmatter writer, the default 80 for a key's text.
    pub(crate) line_width: usize,
}

/// `indentStep`.
pub(crate) const INDENT_STEP: &str = "  ";

/// What a call's `onComment` and `onChompKeep` would have recorded.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Out {
    /// The item's comment is written: the caller does not write it again.
    pub(crate) commented: bool,
    /// A block scalar keeps its final line breaks.
    pub(crate) chomp_keep: bool,
}

/// What `stringify` is given: a node, a pair, or a value a node is created for.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Item<'n> {
    Node(&'n Node),
    Pair(&'n Pair),
    Text(&'n str),
    Null,
}

/// `anchorIsValid`.
fn anchor_is_valid(anchor: &str) -> Result<(), String> {
    let bad = anchor.chars().any(|character| {
        character <= '\u{19}' || is_whitespace(character) || ",[]{}".contains(character)
    });
    if bad {
        return Err(format!(
            "Anchor must not contain whitespace or control characters: {}",
            JsValue::String(anchor.to_owned()).to_json_text()
        ));
    }
    Ok(())
}

/// `getTagObject`: the tag a node is written with.
fn get_tag_object(node: &Node) -> Tag {
    let scalar = node.as_scalar();
    if let Some(tag) = &node.tag {
        let mut named = CORE.into_iter().filter(|candidate| candidate.tag() == tag);
        let format = scalar.and_then(|scalar| scalar.format);
        let first = named.clone().next();
        if let Some(first) = first {
            return named
                .find(|candidate| candidate.format() == format)
                .unwrap_or(first);
        }
    }
    match (&node.body, scalar) {
        (_, Some(scalar)) => {
            let matching = || {
                CORE.into_iter()
                    .filter(|candidate| candidate.identify(&scalar.value))
            };
            let tested: Vec<Tag> = matching()
                .filter(|candidate| candidate.has_test())
                .collect();
            let all: Vec<Tag> = matching().collect();
            let candidates = if all.len() > 1 && !tested.is_empty() {
                tested
            } else {
                all
            };
            candidates
                .iter()
                .find(|candidate| candidate.format() == scalar.format)
                .or_else(|| {
                    candidates
                        .iter()
                        .find(|candidate| candidate.format().is_none())
                })
                .copied()
                .unwrap_or(Tag::Str)
        }
        (Body::Map { .. }, None) => Tag::Map,
        (Body::Seq { .. } | Body::Alias(_) | Body::Scalar(_), None) => Tag::Seq,
    }
}

/// `stringifyProps`: the node's anchor and tag.
fn stringify_props(
    node: &Node,
    ctx: &Context<'_>,
    anchors: &mut HashSet<String>,
) -> Result<String, String> {
    let mut props = Vec::new();
    if let Some(anchor) = node
        .anchor
        .as_ref()
        .filter(|anchor| !anchor.is_empty() && !matches!(node.body, Body::Alias(_)))
    {
        anchor_is_valid(anchor)?;
        anchors.insert(anchor.clone());
        props.push(format!("&{anchor}"));
    }
    if let Some(tag) = &node.tag {
        props.push(ctx.doc.directives.tag_string(tag));
    }
    Ok(props.join(" "))
}

/// `Alias.prototype.toString`.
fn alias_to_string(
    source: &str,
    ctx: &Context<'_>,
    anchors: &HashSet<String>,
) -> Result<String, String> {
    anchor_is_valid(source)?;
    if !anchors.contains(source) {
        return Err(format!(
            "Unresolved alias (the anchor must be set before the alias): {source}"
        ));
    }
    Ok(if ctx.implicit_key {
        format!("*{source} ")
    } else {
        format!("*{source}")
    })
}

/// `stringify`.
pub(crate) fn stringify(
    item: Item<'_>,
    ctx: &mut Context<'_>,
    anchors: &mut HashSet<String>,
    out: &mut Out,
) -> Result<String, String> {
    let created;
    let node = match item {
        Item::Pair(pair) => return stringify_pair(pair, ctx, anchors, out),
        Item::Node(node) => node,
        Item::Text(text) => {
            created = Node::scalar(JsValue::String(text.to_owned()));
            &created
        }
        Item::Null => {
            created = Node::scalar(JsValue::Null);
            &created
        }
    };
    if let Body::Alias(source) = &node.body {
        return alias_to_string(source, ctx, anchors);
    }
    let tag = get_tag_object(node);
    let props = stringify_props(node, ctx, anchors)?;
    if !props.is_empty() {
        ctx.indent_at_start = Some(ctx.indent_at_start.unwrap_or(0) + props.len() + 1);
    }
    let text = match (&node.body, node.as_scalar()) {
        (_, Some(scalar)) => match tag {
            Tag::Str => {
                let mut string_ctx = ctx.clone();
                string_ctx.actual_string = true;
                stringify_string(scalar, node.comment.as_deref(), &mut string_ctx, out)
            }
            Tag::Map | Tag::Seq => stringify_string(scalar, node.comment.as_deref(), ctx, out),
            Tag::Null => match &scalar.source {
                Some(source) if Tag::Null.test(source) => source.clone(),
                Some(_) | None => "null".to_owned(),
            },
            Tag::Bool => stringify_bool(scalar.source.as_deref(), &scalar.value),
            Tag::IntOct => stringify_int(scalar, node.tag.as_deref(), 8, "0o"),
            Tag::IntHex => stringify_int(scalar, node.tag.as_deref(), 16, "0x"),
            Tag::FloatExp => {
                let number = super::tags::to_number(&scalar.value);
                if number.is_finite() {
                    to_exponential(number)
                } else {
                    stringify_number(scalar, node.tag.as_deref())
                }
            }
            Tag::Int | Tag::FloatNaN | Tag::Float => stringify_number(scalar, node.tag.as_deref()),
        },
        (_, None) => stringify_collection(node, ctx, anchors, out)?,
    };
    if props.is_empty() {
        return Ok(text);
    }
    Ok(
        if node.as_scalar().is_some() || text.starts_with(['{', '[']) {
            format!("{props} {text}")
        } else {
            format!("{props}\n{}{text}", ctx.indent)
        },
    )
}

/// The bool tag's `stringify`: the text it was read from when that still says its value.
fn stringify_bool(source: Option<&str>, value: &JsValue) -> String {
    if let Some(source) = source.filter(|source| Tag::Bool.test(source)) {
        let said = source.starts_with(['t', 'T']);
        if value.as_bool() == Some(said) {
            return source.to_owned();
        }
    }
    if value_truthy(value) { "true" } else { "false" }.to_owned()
}

/// JavaScript's truthiness of a scalar's value.
fn value_truthy(value: &JsValue) -> bool {
    match value {
        JsValue::Undefined | JsValue::Null => false,
        JsValue::Bool(flag) => *flag,
        JsValue::Number(number) => *number != 0.0 && !number.is_nan(),
        JsValue::String(text) => !text.is_empty(),
        JsValue::Array(_) | JsValue::Object(_) => true,
    }
}

/// A new context of a document.
fn context(doc: &Document, line_width: usize) -> Context<'_> {
    Context {
        doc,
        indent: String::new(),
        in_flow: false,
        implicit_key: false,
        all_null_values: false,
        indent_at_start: None,
        force_block_indent: false,
        actual_string: false,
        line_width,
    }
}

/// `stringifyKey`'s text of a collection key: the key in flow style, with the anchors converted
/// so far as the ones written.
pub(crate) fn stringify_key_node(
    doc: &Document,
    key: &Node,
    anchors: HashSet<String>,
) -> Result<String, String> {
    let mut anchors = anchors;
    let mut ctx = context(doc, 80);
    ctx.in_flow = true;
    stringify(Item::Node(key), &mut ctx, &mut anchors, &mut Out::default())
}
