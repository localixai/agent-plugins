//! The core's counterpart of `src/format/utf8.ts` at sha256:1cc69c689dc0, which stays TypeScript: a
//! pass over host text of any size, which crossing the boundary would copy twice (docs/RUST.md, "What stays TypeScript").
//! Sizes in UTF-8 bytes, for every limit the docs give in bytes: one byte count and one cut that
//! never splits a character.

/// The UTF-8 bytes of `text`.
#[must_use]
pub const fn utf8_length(text: &str) -> usize {
    text.len()
}

/// The longest prefix of `text` of at most `limit` UTF-8 bytes that ends between two characters.
#[must_use]
pub fn prefix_within_bytes(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    // `TextDecoder.decode` drops one byte order mark at the start of what it decodes (`ignoreBOM`
    // is false), so a cut that begins with U+FEFF loses it; an uncut text is returned as it is.
    let cut = text.get(..end).unwrap_or_default();
    cut.strip_prefix('\u{FEFF}').unwrap_or(cut)
}

#[cfg(test)]
mod tests {
    use super::*;

    // bun -e 'import { prefixWithinBytes as p } from "./src/format/utf8.ts";
    //   console.log(JSON.stringify([p("\ufeff\u00e9\u200b\uffff", 7), p("\ufeffabc", 4),
    //   p("\ufeffabc", 3), p("\ufeff\ufeffabc", 7), p("\ufeffabc", 100), p("a\ufeffbc", 4)]))'
    // prints ["\u00e9","a","","\ufeffa","\ufeffabc","a\ufeff"]
    #[test]
    fn a_cut_loses_a_leading_byte_order_mark_as_the_decoder_drops_it() {
        assert_eq!(
            prefix_within_bytes("\u{FEFF}\u{E9}\u{200B}\u{FFFF}", 7),
            "\u{E9}"
        );
        assert_eq!(prefix_within_bytes("\u{FEFF}abc", 4), "a");
        assert_eq!(prefix_within_bytes("\u{FEFF}abc", 3), "");
        assert_eq!(prefix_within_bytes("\u{FEFF}\u{FEFF}abc", 7), "\u{FEFF}a");
        assert_eq!(prefix_within_bytes("\u{FEFF}abc", 100), "\u{FEFF}abc");
        assert_eq!(prefix_within_bytes("a\u{FEFF}bc", 4), "a\u{FEFF}");
    }
}
