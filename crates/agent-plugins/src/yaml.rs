//! YAML as the npm package `yaml` 2.9.1 reads and writes it, which the package's frontmatter
//! reader and writer use (`src/format/frontmatter.ts`, `src/format/frontmatter-write.ts`): the
//! lexer, parser and composer, the YAML 1.2 core schema, conversion to a value, and the writer.
//! It is ported by hand because no Rust YAML crate gives the package's errors, keeps comments
//! for an edit, or writes what `stringify` writes (docs/RUST.md, RS8).
//!
//! Only what the frontmatter modules ask of the package is here: one document parsed with
//! `{ prettyErrors: false, uniqueKeys: false, logLevel: 'error' }` and turned into a value, and
//! a document written with `{ lineWidth: 0 }`. Positions are UTF-16 code units, as JavaScript
//! counts them. Where the core answers differently is in docs/RUST.md, "Where the crate answers
//! differently".
//!
//! The package is ISC licensed; this port keeps its notice:
//!
//! Copyright Eemeli Aro <eemeli@gmail.com>
//!
//! Permission to use, copy, modify, and/or distribute this software for any purpose with or
//! without fee is hereby granted, provided that the above copyright notice and this permission
//! notice appear in all copies.
//!
//! THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD TO THIS
//! SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE
//! AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
//! WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
//! NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
//! PERFORMANCE OF THIS SOFTWARE.

mod compose;
mod create;
mod cst;
mod directives;
mod lexer;
pub(crate) mod nodes;
mod parser;
mod stringify;
mod tags;
mod to_js;
mod units;

pub(crate) use compose::parse_document;
pub(crate) use create::create_node;
pub(crate) use stringify::stringify_document;
pub(crate) use to_js::to_js;

/// How deep collections may nest, read or written. The package stops where JavaScript's stack
/// does (`Maximum call stack size exceeded.`, past 5,000 levels in Bun); the core stops at a fixed
/// depth with the same message, so its own stack is never at risk: a level costs about 1.4 KiB
/// of WebAssembly's 1 MiB stack and about 8 KiB of a debug build's, whose test threads have
/// 2 MiB. No frontmatter comes near it (docs/RUST.md, "Where the crate answers differently").
const MAX_DEPTH: usize = 128;

/// What the package reports when nesting has used up JavaScript's stack.
const STACK_EXCEEDED: &str = "Maximum call stack size exceeded.";

#[cfg(test)]
mod tests {
    use super::{MAX_DEPTH, parse_document};

    /// The flow sequences of `x:` nested `depth` deep.
    fn nested(depth: usize) -> String {
        format!("x:\n  {}{}\n", "[".repeat(depth), "]".repeat(depth))
    }

    /// Past its stack the package gives one error, `bun -e "import { parseDocument } from 'yaml';
    /// const n = 100000; console.log(parseDocument('x:\n  ' + '['.repeat(n) + ']'.repeat(n),
    /// { prettyErrors: false, uniqueKeys: false, logLevel: 'error' }).errors.map((e) =>
    /// e.message))"`: `[ "Maximum call stack size exceeded.", "Maximum call stack size
    /// exceeded." ]`, the second where the stack ran out a second time. The core gives the
    /// first past `MAX_DEPTH`, in time linear in the text, and its stack holds (the tree is
    /// dropped without recursing).
    #[test]
    fn nesting_past_the_cap_is_the_packages_stack_error() {
        let (deep, _) = parse_document(&nested(100_000));
        let first = deep.errors.first().map(|error| error.message.as_str());
        assert_eq!(first, Some("Maximum call stack size exceeded."));
        let (within, _) = parse_document(&nested(MAX_DEPTH - 1));
        assert!(within.errors.is_empty());
        assert!(super::to_js(&within).is_ok());
    }

    /// The writer refuses a value nested past `MAX_DEPTH` with the same message, and writes one
    /// within it (and reads it back) on a debug build's test stack.
    #[test]
    fn the_writer_stops_at_the_cap_too() {
        use crate::js::{JsObject, JsValue};
        let mut value = JsValue::Null;
        for _ in 0..MAX_DEPTH - 1 {
            value = JsValue::Array(vec![value]);
        }
        let mut fields = JsObject::new();
        fields.insert("x".to_owned(), value.clone());
        let fits = super::create_node(&JsValue::Object(fields.clone()));
        let text = fits
            .and_then(|node| super::stringify_document(&super::nodes::Document::new(Some(node))));
        let read = text.map(|text| parse_document(&text).0.errors.is_empty());
        assert_eq!(read, Ok(true));
        fields.insert("x".to_owned(), JsValue::Array(vec![value]));
        let deep = super::create_node(&JsValue::Object(fields)).map(|_| ());
        assert_eq!(deep, Err("Maximum call stack size exceeded.".to_owned()));
    }
}
