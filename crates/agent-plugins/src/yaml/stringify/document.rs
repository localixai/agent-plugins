//! A whole document as text (`dist/stringify/stringifyDocument.js` and
//! `Document.prototype.toString`): its directives, its comments and its contents.

use std::collections::HashSet;

use super::{Item, Out, context, indent_comment, line_comment, stringify, stringify_comment};
use crate::yaml::nodes::{Body, Document, Key, Node, Pair};

/// The tags a document's nodes carry, for the `%TAG` lines.
fn used_tags(node: &Node, out: &mut Vec<String>) {
    if let Some(tag) = &node.tag {
        out.push(tag.clone());
    }
    match &node.body {
        Body::Map { items, .. } => {
            for Pair { key, value } in items {
                if let Key::Node(key) = key {
                    used_tags(key, out);
                }
                if let Some(value) = value {
                    used_tags(value, out);
                }
            }
        }
        Body::Seq { items, .. } => items.iter().for_each(|item| used_tags(item, out)),
        Body::Scalar(_) | Body::Alias(_) => {}
    }
}

/// `doc.toString({ lineWidth: 0 })`, through `stringifyDocument`.
///
/// # Errors
///
/// A document with errors, or an anchor or alias the writer refuses.
pub(crate) fn stringify_document(doc: &Document) -> Result<String, String> {
    if !doc.errors.is_empty() {
        return Err("Document with errors cannot be stringified".to_owned());
    }
    let mut lines: Vec<String> = Vec::new();
    let mut has_directives = false;
    let mut tags = Vec::new();
    if let Some(contents) = &doc.contents {
        used_tags(contents, &mut tags);
    }
    let directives = doc.directives.to_string(&tags);
    if !directives.is_empty() {
        lines.push(directives);
        has_directives = true;
    } else if doc.directives.doc_start {
        has_directives = true;
    }
    if has_directives {
        lines.push("---".to_owned());
    }
    let mut anchors = HashSet::new();
    let mut ctx = context(doc, 0);
    if let Some(before) = doc
        .comment_before
        .as_deref()
        .filter(|before| !before.is_empty())
    {
        if lines.len() != 1 {
            lines.insert(0, String::new());
        }
        lines.insert(0, indent_comment(&stringify_comment(before), ""));
    }
    let mut chomp_keep = false;
    let mut content_comment = None;
    if let Some(contents) = &doc.contents {
        if contents.space_before && has_directives {
            lines.push(String::new());
        }
        if let Some(before) = contents
            .comment_before
            .as_deref()
            .filter(|before| !before.is_empty())
        {
            lines.push(indent_comment(&stringify_comment(before), ""));
        }
        // Top-level block scalars need to be indented if followed by a comment.
        ctx.force_block_indent = doc
            .comment
            .as_deref()
            .is_some_and(|comment| !comment.is_empty());
        content_comment = contents
            .comment
            .clone()
            .filter(|comment| !comment.is_empty());
        let keeps_chomp = content_comment.is_none();
        let mut out = Out::default();
        let mut body = stringify(Item::Node(contents), &mut ctx, &mut anchors, &mut out)?;
        if out.commented {
            content_comment = None;
        }
        chomp_keep = keeps_chomp && out.chomp_keep;
        if let Some(comment) = &content_comment {
            body.push_str(&line_comment(&body, "", &stringify_comment(comment)));
        }
        if body.starts_with(['|', '>']) && lines.last().is_some_and(|last| last == "---") {
            // Top-level block scalars with a preceding doc marker ought to use the same line for
            // their header.
            if let Some(last) = lines.last_mut() {
                *last = format!("--- {body}");
            }
        } else {
            lines.push(body);
        }
    } else {
        lines.push(stringify(
            Item::Null,
            &mut ctx,
            &mut anchors,
            &mut Out::default(),
        )?);
    }
    push_document_comment(doc, &mut lines, chomp_keep, content_comment.is_some());
    Ok(lines.join("\n") + "\n")
}

/// The document's comment after its contents: on the `...` line, or after a blank line.
fn push_document_comment(
    doc: &Document,
    lines: &mut Vec<String>,
    chomp_keep: bool,
    content_comment: bool,
) {
    let comment = doc.comment.as_deref().filter(|comment| !comment.is_empty());
    if doc.directives.doc_end {
        match comment.map(stringify_comment) {
            Some(text) if text.contains('\n') => {
                lines.push("...".to_owned());
                lines.push(indent_comment(&text, ""));
            }
            Some(text) => lines.push(format!("... {text}")),
            None => lines.push("...".to_owned()),
        }
    } else {
        let comment = comment.map(|comment| {
            if chomp_keep {
                comment.trim_start_matches('\n')
            } else {
                comment
            }
        });
        if let Some(comment) = comment.filter(|comment| !comment.is_empty()) {
            if (!chomp_keep || content_comment) && lines.last().is_some_and(|last| !last.is_empty())
            {
                lines.push(String::new());
            }
            lines.push(indent_comment(&stringify_comment(comment), ""));
        }
    }
}
