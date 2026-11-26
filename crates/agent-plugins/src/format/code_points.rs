//! Port of `src/format/code-points.ts`, switched.
//! The length of a text in Unicode code points, the unit the specification counts a name or an
//! option label in (spec §3.4.2, §4.3): a character outside the Basic Multilingual Plane is one,
//! not the two UTF-16 units JavaScript's `String.length` gives.

/// How many code points `text` holds.
#[must_use]
pub fn code_point_length(text: &str) -> usize {
    text.chars().count()
}

crate::registry::exports! {
    "format/code-points" {
        "codePointLength" => ok |text: String| code_point_length(&text);
    }
}
