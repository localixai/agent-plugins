//! The comment before a document (`parsePrelude` of `compose/composer.js`).

use crate::yaml::units::{Units, string, unit};

/// `parsePrelude`: the comment of the lines before a document, and whether a blank line follows
/// it.
pub(super) fn parse_prelude(prelude: &[Units]) -> (String, bool) {
    let mut comment = String::new();
    let mut at_comment = false;
    let mut after_empty_line = false;
    let mut index = 0;
    while let Some(source) = prelude.get(index) {
        match source.first().copied() {
            Some(first) if first == unit('#') => {
                if !comment.is_empty() {
                    comment.push_str(if after_empty_line { "\n\n" } else { "\n" });
                }
                let text = string(source.get(1..).unwrap_or_default());
                comment.push_str(if text.is_empty() { " " } else { &text });
                at_comment = true;
                after_empty_line = false;
            }
            Some(first) if first == unit('%') => {
                if prelude.get(index + 1).and_then(|next| next.first()) != Some(&unit('#')) {
                    index += 1;
                }
                at_comment = false;
            }
            _ => {
                // This may be wrong after doc-end, but in that case it doesn't matter.
                if !at_comment {
                    after_empty_line = true;
                }
                at_comment = false;
            }
        }
        index += 1;
    }
    (comment, after_empty_line)
}
