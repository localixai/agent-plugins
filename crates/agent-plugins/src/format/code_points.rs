//! Ported from `src/format/code-points.ts`; only the core uses it now, so it has no wrapper.
//! The length of a text in Unicode code points, the unit the specification counts a name or an
//! option label in (spec §3.4.2, §4.3): a character outside the Basic Multilingual Plane is one,
//! not the two UTF-16 units JavaScript's `String.length` gives.

/// How many code points `text` holds.
#[must_use]
pub fn code_point_length(text: &str) -> usize {
    text.chars().count()
}
