//! `String.prototype.normalize(form)`: the four Unicode normalization forms, by the
//! `unicode-normalization` crate. `tests/differential/js.rs` holds it to the answers
//! `scripts/rust/js-oracle.ts` recorded in Bun (JavaScriptCore, ICU).

use serde::Deserialize;
use unicode_normalization::UnicodeNormalization;

/// The form `normalize` is given.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum NormalForm {
    /// Canonical decomposition, then canonical composition: `normalize()` and `'NFC'`.
    #[serde(rename = "NFC")]
    Nfc,
    /// Canonical decomposition.
    #[serde(rename = "NFD")]
    Nfd,
    /// Compatibility decomposition, then canonical composition.
    #[serde(rename = "NFKC")]
    Nfkc,
    /// Compatibility decomposition.
    #[serde(rename = "NFKD")]
    Nfkd,
}

/// `text.normalize(form)`.
#[must_use]
pub fn normalize(text: &str, form: NormalForm) -> String {
    match form {
        NormalForm::Nfc => text.nfc().collect(),
        NormalForm::Nfd => text.nfd().collect(),
        NormalForm::Nfkc => text.nfkc().collect(),
        NormalForm::Nfkd => text.nfkd().collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{NormalForm, normalize};

    #[test]
    fn each_form_answers_as_javascript_does() {
        // 'Åﬁ'.normalize(form) in Bun.
        assert_eq!(
            normalize("A\u{30a}\u{fb01}", NormalForm::Nfc),
            "\u{c5}\u{fb01}"
        );
        assert_eq!(
            normalize("\u{c5}\u{fb01}", NormalForm::Nfd),
            "A\u{30a}\u{fb01}"
        );
        assert_eq!(normalize("A\u{30a}\u{fb01}", NormalForm::Nfkc), "\u{c5}fi");
        assert_eq!(normalize("\u{c5}\u{fb01}", NormalForm::Nfkd), "A\u{30a}fi");
    }
}
