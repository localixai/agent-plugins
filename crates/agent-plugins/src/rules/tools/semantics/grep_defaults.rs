//! Port of `src/rules/tools/semantics/grep-defaults.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::js::{JsValue, slice_index};
use crate::rules::tools::inputs::grep::{GREP_OUTPUT_MODES, GrepOutputMode};

/// "Defaults to `files_with_matches`"; tools-reference.md: "file paths only, no line content. This is the default."
const DEFAULT_MODE: GrepOutputMode = GrepOutputMode::FilesWithMatches;

/// The output mode a Grep call runs in. A value that is not one of the three is read as absent here;
/// `parseToolInput` reports it (spec/judgments.md).
#[must_use]
pub fn effective_grep_output_mode(input: &JsValue) -> GrepOutputMode {
    let mode = input
        .as_object()
        .and_then(|members| members.get("output_mode"))
        .and_then(JsValue::as_str);
    GREP_OUTPUT_MODES
        .into_iter()
        .find(|known| mode.is_some_and(|text| mode_text(*known) == text))
        .unwrap_or(DEFAULT_MODE)
}

/// The mode's own spelling.
const fn mode_text(mode: GrepOutputMode) -> &'static str {
    match mode {
        GrepOutputMode::Content => "content",
        GrepOutputMode::FilesWithMatches => "files_with_matches",
        GrepOutputMode::Count => "count",
    }
}

/// tools-reference.md, "Grep tool behavior": "By default, patterns match within a single line.
/// Claude can set `multiline: true`"; the SDK's GrepInput says "Default: false". Only `true` turns
/// it on (spec/judgments.md).
#[must_use]
pub fn effective_grep_multiline(input: &JsValue) -> bool {
    input
        .as_object()
        .and_then(|members| members.get("multiline"))
        .is_some_and(|value| matches!(value, JsValue::Bool(true)))
}

/// The SDK's GrepInput: `head_limit` "Defaults to 250 when unspecified. Pass 0 for unlimited".
const DEFAULT_HEAD_LIMIT: f64 = 250.0;

/// What a Grep call asks of the listing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Deserialize)]
pub struct GrepWindowRequest {
    /// "Skip first N lines/entries before applying head_limit"; 0 when absent.
    #[serde(default)]
    pub offset: Option<f64>,
    /// "Limit output to first N lines/entries"; 250 when absent, 0 for no limit.
    #[serde(default)]
    pub head_limit: Option<f64>,
}

/// One file's match count.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GrepCount {
    /// The file.
    pub path: String,
    /// How many matches it holds.
    pub count: f64,
}

fn whole_or(value: Option<f64>, fallback: f64) -> f64 {
    value.map_or(fallback, |number| {
        if number.is_finite() && number.fract() == 0.0 && number >= 0.0 {
            number
        } else {
            fallback
        }
    })
}

/// `| tail -n +offset | head -head_limit` over the entries (SDK GrepInput).
fn slice_window<T: Clone>(entries: &[T], request: GrepWindowRequest) -> Vec<T> {
    let offset = whole_or(request.offset, 0.0);
    let limit = whole_or(request.head_limit, DEFAULT_HEAD_LIMIT);
    let start = slice_index(offset, entries.len());
    let end = if limit == 0.0 {
        entries.len()
    } else {
        slice_index(offset + limit, entries.len())
    };
    entries
        .get(start..end.max(start))
        .map(<[T]>::to_vec)
        .unwrap_or_default()
}

/// The lines of a `content` window and its notice.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrepWindow {
    /// The lines the window selects.
    pub lines: Vec<String>,
    /// "No entries at this offset" when the offset is past the last of a pattern that has matches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<&'static str>,
}

/// What a `content` window says when it starts past the last match.
pub const NO_ENTRIES_AT_OFFSET: &str = "No entries at this offset";

/// The lines Grep returns in `content` mode (tools-reference.md, "Grep tool behavior"): the window
/// `offset` and `head_limit` select. "When the tool's `offset` parameter points past the last match
/// for a pattern that has matches, Grep returns `No entries at this offset`"; a pattern with no
/// match has no such notice (spec/judgments.md).
#[must_use]
pub fn window_grep_content(matches: &[String], request: GrepWindowRequest) -> GrepWindow {
    let lines = slice_window(matches, request);
    #[expect(clippy::cast_precision_loss, reason = "a line count is far below 2^53")]
    let past_end = !matches.is_empty() && whole_or(request.offset, 0.0) >= matches.len() as f64;
    GrepWindow {
        lines,
        notice: past_end.then_some(NO_ENTRIES_AT_OFFSET),
    }
}

/// "A total across all matching files" that "covers every match" however the listing is cut.
#[must_use]
pub fn total_grep_count(counts: &[GrepCount]) -> f64 {
    counts.iter().fold(0.0, |sum, entry| sum + entry.count)
}

/// The per-file entries of `count` mode after `offset` and `head_limit`, with the whole total.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct GrepCountWindow {
    /// The entries the window selects.
    pub entries: Vec<GrepCount>,
    /// Every file's count added.
    pub total: f64,
}

/// The per-file entries of `count` mode after `offset` and `head_limit`, with the whole total.
#[must_use]
pub fn window_grep_counts(counts: &[GrepCount], request: GrepWindowRequest) -> GrepCountWindow {
    GrepCountWindow {
        entries: slice_window(counts, request),
        total: total_grep_count(counts),
    }
}

crate::registry::exports! {
    "rules/tools/semantics/grep-defaults" {
        "effectiveGrepOutputMode" => ok |input: JsValue| effective_grep_output_mode(&input);
        "effectiveGrepMultiline" => ok |input: JsValue| effective_grep_multiline(&input);
        "NO_ENTRIES_AT_OFFSET" => ok || NO_ENTRIES_AT_OFFSET;
        "windowGrepContent" => ok |matches: Vec<String>, request: GrepWindowRequest| window_grep_content(&matches, request);
        "totalGrepCount" => ok |counts: Vec<GrepCount>| total_grep_count(&counts);
        "windowGrepCounts" => ok |counts: Vec<GrepCount>, request: GrepWindowRequest| window_grep_counts(&counts, request);
    }
}
