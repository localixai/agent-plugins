//! Port of `src/convert/types.ts`, switched.
//!
//! What a conversion between the two plugin forms gives (Agent Plugins Extended §18.4): the files to
//! write beside the source, as data, and a note for everything it could not carry over or carried
//! over with a difference (§18.4.1 item 1). The kit writes nothing: a host writes `files` into the
//! plugin root, which makes a dual package (§18.4.1 item 4), and shows `notes`. The core holds the
//! one constant; the shapes stay in the TypeScript.

/// `stopped`: nothing is written (a name the target refuses, a file already there, output the target
/// would not accept); `not-converted`: a field, entry, file or behaviour the target form does not
/// get; `reported`: carried over, but it acts differently or the target warns about it.
pub const CONVERSION_OUTCOMES: &[&str] = &["stopped", "not-converted", "reported"];

crate::registry::exports! {
    "convert/types" {
        "CONVERSION_OUTCOMES" => ok || CONVERSION_OUTCOMES;
    }
}
