//! The composer of `yaml` (`dist/compose/composer.js`, `dist/compose/compose-doc.js`, and
//! `parseDocument` in `dist/public-api.js`): the parser's tokens composed into documents, with
//! the comments around them and every error at the offset it starts. Warnings are dropped: the
//! reader logs at `error` level and never reads them.

mod block;
mod block_scalar;
mod ctx;
mod end;
mod flow;
mod flow_scalar;
mod node;
mod prelude;
mod props;

pub(crate) use self::ctx::Ctx;
use self::end::resolve_end;
use self::node::{compose_empty_node, compose_node};
use self::prelude::parse_prelude;
use self::props::{Next, Options, resolve_props};
use super::cst::{Kind, SourceToken, Token, dismantle};
use super::directives::Directives;
use super::nodes::{Body, Document, Key, YamlError, append};
use super::parser::{parse, quoted};
use super::units::{Units, string, units};

/// The composer: the stream's directives, the document being composed, and what came before it.
struct Composer {
    doc: Option<Document>,
    at_directives: bool,
    prelude: Vec<Units>,
    errors: Vec<YamlError>,
    directives: Directives,
    ids: usize,
    docs: Vec<Document>,
}

impl Composer {
    /// `decorate`.
    fn decorate(&mut self, doc: &mut Document, after_doc: bool) {
        let (comment, after_empty_line) = parse_prelude(&self.prelude);
        if !comment.is_empty() {
            if after_doc {
                append(&mut doc.comment, "\n", &comment);
            } else if after_empty_line || doc.directives.doc_start || doc.contents.is_none() {
                doc.comment_before = Some(comment);
            } else if let Some(contents) = doc.contents.as_mut() {
                let first = match &mut contents.body {
                    Body::Map { items, flow: false } => {
                        items.first_mut().and_then(|pair| match &mut pair.key {
                            Key::Node(key) => Some(&mut **key),
                            Key::Text(_) => None,
                        })
                    }
                    Body::Seq { items, flow: false } => items.first_mut(),
                    Body::Map { .. } | Body::Seq { .. } | Body::Scalar(_) | Body::Alias(_) => None,
                };
                let target = match first {
                    Some(first) => first,
                    None => contents,
                };
                target.comment_before = Some(match target.comment_before.take() {
                    Some(before) if !before.is_empty() => format!("{comment}\n{before}"),
                    Some(_) | None => comment,
                });
            }
        }
        if after_doc {
            doc.errors.append(&mut self.errors);
        } else {
            doc.errors = std::mem::take(&mut self.errors);
        }
        self.prelude.clear();
    }

    /// `next`: one token of the stream.
    fn next(&mut self, token: Token) {
        match token {
            Token::Directive { offset, source } => {
                let line = string(&source);
                let mut errors = Vec::new();
                self.directives.add(&line, &mut |at, message, warning| {
                    if !warning {
                        errors.push(YamlError {
                            offset: offset + at,
                            message,
                        });
                    }
                });
                self.errors.append(&mut errors);
                self.prelude.push(source);
                self.at_directives = true;
            }
            Token::Document {
                offset,
                start,
                value,
                end,
            } => {
                let mut doc = self.compose_doc(offset, &start, value.as_deref(), end.as_deref());
                if let Some(value) = value {
                    dismantle(*value);
                }
                if self.at_directives && !doc.directives.doc_start {
                    self.errors.push(YamlError {
                        offset,
                        message: "Missing directives-end/doc-start indicator line".to_owned(),
                    });
                }
                self.decorate(&mut doc, false);
                if let Some(previous) = self.doc.replace(doc) {
                    self.docs.push(previous);
                }
                self.at_directives = false;
            }
            Token::Source(SourceToken {
                kind: Kind::ByteOrderMark | Kind::Space,
                ..
            }) => {}
            Token::Source(SourceToken {
                kind: Kind::Comment | Kind::Newline,
                source,
                ..
            }) => {
                self.prelude.push(source);
            }
            Token::Error(error) => {
                let message = if error.source.is_empty() {
                    error.message
                } else {
                    format!("{}: {}", error.message, quoted(&error.source))
                };
                let error = YamlError {
                    offset: error.offset,
                    message,
                };
                match self.doc.as_mut() {
                    Some(doc) if !self.at_directives => doc.errors.push(error),
                    Some(_) | None => self.errors.push(error),
                }
            }
            Token::DocEnd {
                offset,
                source,
                end,
            } => self.doc_end(offset, &source, end.as_deref()),
            token @ Token::Source(_)
            | token @ Token::FlowScalar(_)
            | token @ Token::BlockScalar { .. }
            | token @ Token::BlockMap { .. }
            | token @ Token::BlockSeq { .. }
            | token @ Token::FlowCollection { .. } => self.errors.push(YamlError {
                offset: token.offset(),
                message: format!("Unsupported token {}", token.type_name()),
            }),
        }
    }

    /// The `doc-end` case of `next`.
    fn doc_end(&mut self, offset: usize, source: &[u16], end: Option<&[SourceToken]>) {
        let Some(mut doc) = self.doc.take() else {
            self.errors.push(YamlError {
                offset,
                message: "Unexpected doc-end without preceding document".to_owned(),
            });
            return;
        };
        doc.directives.doc_end = true;
        let (comment, end_offset) = {
            let mut ctx = self.ctx(&doc.directives);
            resolve_end(&mut ctx, end, offset + source.len(), true)
        };
        self.decorate(&mut doc, true);
        if !comment.is_empty() {
            append(&mut doc.comment, "\n", &comment);
        }
        doc.range[2] = end_offset;
        self.doc = Some(doc);
    }

    fn ctx<'a>(&'a mut self, directives: &'a Directives) -> Ctx<'a> {
        Ctx {
            at_key: false,
            at_root: true,
            strict: true,
            directives,
            errors: &mut self.errors,
            ids: &mut self.ids,
            depth: 0,
        }
    }

    /// `composeDoc`.
    fn compose_doc(
        &mut self,
        offset: usize,
        start: &[SourceToken],
        value: Option<&Token>,
        end: Option<&[SourceToken]>,
    ) -> Document {
        let mut directives = self.directives.at_document();
        let mut ctx = self.ctx(&directives);
        let props = resolve_props(
            &mut ctx,
            start,
            Options {
                flow: None,
                indicator: Kind::DocStart,
                next: Next::of(value).or_else(|| Next::of_source(end.and_then(<[_]>::first))),
                offset,
                parent_indent: 0,
                start_on_newline: true,
            },
        );
        let found = props.found.is_some();
        if found
            && matches!(value, Some(Token::BlockMap { .. } | Token::BlockSeq { .. }))
            && !props.has_newline
        {
            ctx.error(
                props.end,
                "Block collection cannot start on same line with directives-end marker",
            );
        }
        let contents = match value {
            Some(value) => compose_node(&mut ctx, value, &props),
            None => compose_empty_node(&mut ctx, props.end, Some(start), None, &props),
        };
        let content_end = contents.range[2];
        let (comment, end_offset) = resolve_end(&mut ctx, end, content_end, false);
        directives.doc_start = found;
        Document {
            contents: Some(contents),
            comment_before: None,
            comment: (!comment.is_empty()).then_some(comment),
            directives,
            errors: Vec::new(),
            range: [offset, content_end, end_offset],
        }
    }

    /// `end`, with `forceDoc`: the last document, or an empty one for a stream with none.
    fn end(&mut self, end_offset: usize) {
        if let Some(mut doc) = self.doc.take() {
            self.decorate(&mut doc, true);
            self.docs.push(doc);
            return;
        }
        let mut doc = Document {
            contents: None,
            comment_before: None,
            comment: None,
            directives: self.directives.at_document(),
            errors: Vec::new(),
            range: [0, end_offset, end_offset],
        };
        if self.at_directives {
            self.errors.push(YamlError {
                offset: end_offset,
                message: "Missing directives-end indicator line".to_owned(),
            });
        }
        self.decorate(&mut doc, false);
        self.docs.push(doc);
    }
}

/// `parseDocument(text, options)`: the first document of the text, with an error for any after
/// it; and where each line starts, as the package's line counter records them.
pub(crate) fn parse_document(text: &str) -> (Document, Vec<usize>) {
    let source = units(text);
    let parsed = parse(&source);
    let mut composer = Composer {
        doc: None,
        at_directives: false,
        prelude: Vec::new(),
        errors: Vec::new(),
        directives: Directives::new(),
        ids: 0,
        docs: Vec::new(),
    };
    for token in parsed.tokens {
        composer.next(token);
    }
    composer.end(source.len());
    let mut docs = composer.docs.into_iter();
    let mut first = docs.next().unwrap_or_else(|| Document {
        contents: None,
        comment_before: None,
        comment: None,
        directives: Directives::new(),
        errors: Vec::new(),
        range: [0, source.len(), source.len()],
    });
    if let Some(second) = docs.next() {
        first.errors.push(YamlError {
            offset: second.range[0],
            message: "Source contains multiple documents; please use YAML.parseAllDocuments()"
                .to_owned(),
        });
    }
    (first, parsed.line_starts)
}
