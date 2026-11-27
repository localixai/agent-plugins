//! The characters of a JavaScript identifier as a regular expression with `u` names them:
//! `\p{ID_Start}` and `\p{ID_Continue}`, Unicode's identifier properties (the `regex` crate's
//! tables, of its Unicode version: docs/RUST.md, "Where the crate answers differently"). A pattern
//! such as `/[\p{ID_Start}_$][\p{ID_Continue}$]*/u` is these two over a text's code points.

use regex::Regex;

use super::{Pattern, regex_test};

/// `\p{ID_Start}`.
static ID_START: Pattern = Pattern::new(|| Regex::new(r"^\p{ID_Start}$").ok());
/// `\p{ID_Continue}`.
static ID_CONTINUE: Pattern = Pattern::new(|| Regex::new(r"^\p{ID_Continue}$").ok());

/// Whether `character` matches `\p{ID_Start}`: a letter, a letter number, or one of the few
/// others Unicode lets start an identifier.
#[must_use]
pub fn is_id_start(character: char) -> bool {
    if character.is_ascii() {
        return character.is_ascii_alphabetic();
    }
    regex_test(&ID_START, character.encode_utf8(&mut [0; 4]))
}

/// Whether `character` matches `\p{ID_Continue}`: `\p{ID_Start}`, a digit, a combining mark,
/// a connector such as `_`.
#[must_use]
pub fn is_id_continue(character: char) -> bool {
    if character.is_ascii() {
        return character.is_ascii_alphanumeric() || character == '_';
    }
    regex_test(&ID_CONTINUE, character.encode_utf8(&mut [0; 4]))
}

#[cfg(test)]
mod tests {
    use super::{is_id_continue, is_id_start};

    // bun -e 'console.log(["a","_","$","1","é","ℕ","Ⅻ","٣","́","‿","😀","-"].map(c =>
    //   [/\p{ID_Start}/u.test(c), /\p{ID_Continue}/u.test(c)]))' prints
    // a [true,true] _ [false,true] $ [false,false] 1 [false,true] é [true,true] ℕ [true,true]
    // Ⅻ [true,true] ٣ [false,true] U+0301 [false,true] ‿ [false,true] 😀 [false,false] - [false,false]
    #[test]
    fn identifier_characters_are_unicodes_properties() {
        let expected = [
            ('a', true, true),
            ('_', false, true),
            ('$', false, false),
            ('1', false, true),
            ('é', true, true),
            ('ℕ', true, true),
            ('Ⅻ', true, true),
            ('٣', false, true),
            ('\u{301}', false, true),
            ('‿', false, true),
            ('😀', false, false),
            ('-', false, false),
        ];
        for (character, start, rest) in expected {
            assert_eq!(
                (is_id_start(character), is_id_continue(character)),
                (start, rest),
                "{character}"
            );
        }
    }
}
