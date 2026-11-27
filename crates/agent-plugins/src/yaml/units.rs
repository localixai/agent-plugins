//! Text as the `yaml` package indexes it: UTF-16 code units, so every offset, length and column
//! the reader works with is JavaScript's (`source.length`, `source[i]`, `indexOf`), and a
//! problem's line and column are the ones the package reports.

/// A text as UTF-16 code units.
pub(crate) type Units = Vec<u16>;

/// The code units of a text.
pub(crate) fn units(text: &str) -> Units {
    text.encode_utf16().collect()
}

/// The text of code units (a lone surrogate, which a Rust string cannot hold, reads as U+FFFD:
/// docs/RUST.md, "Where the crate answers differently").
pub(crate) fn string(text: &[u16]) -> String {
    String::from_utf16_lossy(text)
}

/// The code unit of an ASCII character.
pub(crate) const fn unit(character: char) -> u16 {
    character as u16
}

/// `text[index]`: `None` (JavaScript's `undefined`) before the start or past the end.
pub(crate) fn at(text: &[u16], index: isize) -> Option<u16> {
    usize::try_from(index)
        .ok()
        .and_then(|index| text.get(index).copied())
}

/// Whether `text[index]` is the ASCII character `character`.
pub(crate) fn is(text: &[u16], index: isize, character: char) -> bool {
    at(text, index) == Some(unit(character))
}

/// `text.indexOf(character, from)`, `-1` when it is not there.
pub(crate) fn index_of(text: &[u16], character: char, from: isize) -> isize {
    let start = usize::try_from(from.max(0)).unwrap_or(0);
    text.get(start..)
        .and_then(|rest| rest.iter().position(|&found| found == unit(character)))
        .and_then(|found| isize::try_from(start + found).ok())
        .unwrap_or(-1)
}

/// The length as a signed index, for the reader's arithmetic that may step before the start.
pub(crate) fn signed(length: usize) -> isize {
    isize::try_from(length).unwrap_or(isize::MAX)
}

/// An index known not to be negative, as an unsigned one (a negative one is 0).
pub(crate) fn unsigned(index: isize) -> usize {
    usize::try_from(index).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::{at, index_of, units};

    #[test]
    fn indexes_are_javascripts() {
        let text = units("a\u{1F600}b\n");
        assert_eq!(text.len(), 5);
        assert_eq!(at(&text, 3), Some(u16::from(b'b')));
        assert_eq!(at(&text, -1), None);
        assert_eq!(index_of(&text, '\n', 0), 4);
        assert_eq!(index_of(&text, '\n', 5), -1);
    }
}
