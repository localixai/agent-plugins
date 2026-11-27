//! Port of `src/hooks/output/text-cap.ts`, switched.
//!
//! The cap on a hook's text (hooks.md, "JSON output"): "A hook's `additionalContext`,
//! `systemMessage`, and `initialUserMessage` strings, and its plain stdout, are capped at 10,000
//! characters". Each string is measured on its own; over the limit, "Claude Code saves the output to
//! a file in the session directory. It replaces the output with the file path and a preview of up to
//! the first 2,000 characters." Except for PreCompact stdout, which is cut at the cap with a marker.

use serde::Serialize;

use crate::format::issues::{PluginIssue, warning};
use crate::ports::Signal;
use crate::ports::scratch_files::ScratchFiles;
use crate::registry::{Port, catch_host};

/// The cap on a hook's text, in characters.
pub const TEXT_CAP: usize = 10_000;
/// How much of a spilled text the model is shown.
pub const PREVIEW_LENGTH: usize = 2_000;

/// A text cut at the cap and the marker that says so (judgment: the docs give no marker text).
fn cut_marker() -> String {
    format!("\n[output cut at {TEXT_CAP} characters]")
}

/// The first `count` characters (code points), so a cut never splits a surrogate pair.
fn first_characters(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

fn length_of(text: &str) -> usize {
    let units = text.encode_utf16().count();
    if units <= TEXT_CAP {
        units
    } else {
        text.chars().count()
    }
}

fn exceeds_cap(text: &str) -> bool {
    length_of(text) > TEXT_CAP
}

/// What the model is told in place of a text that is saved to a file (judgment: the docs give no
/// wording).
fn spilled_text(path: &str, total: usize, preview: &str) -> String {
    format!(
        "Output too large ({total} characters). Full output saved to: {path}\n\nPreview (first {PREVIEW_LENGTH} characters):\n{preview}"
    )
}

/// A text as the model gets it, and what was found on the way.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Limited {
    /// The text.
    pub text: String,
    /// Why it was cut.
    pub issues: Vec<PluginIssue>,
}

/// `text` as the model gets it: as is under the cap, else a pointer to the saved file with a
/// preview. Without a `ScratchFiles` port, or when the write fails, the text is cut at the cap with
/// a warning, so a host that cannot save a file never injects more than the cap.
pub async fn limit_text(
    name: &str,
    text: &str,
    scratch: Option<&Port>,
    signal: &Signal,
) -> Limited {
    if !exceeds_cap(text) {
        return Limited {
            text: text.to_owned(),
            issues: Vec::new(),
        };
    }
    let total = text.chars().count();
    let cut = first_characters(text, TEXT_CAP) + &cut_marker();
    let Some(scratch) = scratch else {
        return Limited {
            text: cut,
            issues: vec![warning(
                name,
                &format!(
                    "{total} characters exceed the {TEXT_CAP} cap and no ScratchFiles port can save them: cut"
                ),
            )],
        };
    };
    match catch_host(async { scratch.write(name, text, Some(signal)).await }).await {
        Ok(path) => Limited {
            text: spilled_text(&path, total, &first_characters(text, PREVIEW_LENGTH)),
            issues: Vec::new(),
        },
        Err(thrown) => Limited {
            text: cut,
            issues: vec![warning(
                name,
                &format!(
                    "{total} characters exceed the {TEXT_CAP} cap and could not be saved ({}): cut",
                    thrown.message
                ),
            )],
        },
    }
}

/// PreCompact stdout: "Claude Code cuts longer output at the cap and appends a marker."
#[must_use]
pub fn cut_text(text: &str) -> String {
    if exceeds_cap(text) {
        first_characters(text, TEXT_CAP) + &cut_marker()
    } else {
        text.to_owned()
    }
}

crate::registry::exports! {
    "hooks/output/text-cap" {
        "TEXT_CAP" => ok || TEXT_CAP;
        "PREVIEW_LENGTH" => ok || PREVIEW_LENGTH;
        "limitText" => async ok |name: String, text: String, scratch: Option<Port>, signal: Signal| limit_text(&name, &text, scratch.as_ref(), &signal).await;
    }
}
