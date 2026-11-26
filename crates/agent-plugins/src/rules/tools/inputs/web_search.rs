//! Port of `src/rules/tools/inputs/web-search.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;

/// hooks.md ("WebSearch"): `"standard"` requests a lighter search and `"extended"` the full search.
pub const WEB_SEARCH_MODES: [WebSearchMode; 2] = [WebSearchMode::Standard, WebSearchMode::Extended];

/// One of `WEB_SEARCH_MODES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebSearchMode {
    /// `standard`.
    Standard,
    /// `extended`.
    Extended,
}

impl WebSearchMode {
    /// Its name, as the input spells it.
    const fn name(self) -> &'static str {
        match self {
            Self::Standard => "standard",
            Self::Extended => "extended",
        }
    }
}

/// "Some sessions don't send `mode`, and a call without it runs the full search."
const DEFAULT_MODE: WebSearchMode = WebSearchMode::Extended;

/// The search a call runs. A `mode` that is neither of the two is read as absent here;
/// `parseToolInput` reports it.
#[must_use]
pub fn effective_search_mode(input: &JsValue) -> WebSearchMode {
    let mode = input.as_object().and_then(|members| members.get("mode"));
    WEB_SEARCH_MODES
        .into_iter()
        .find(|known| mode.and_then(JsValue::as_str) == Some(known.name()))
        .unwrap_or(DEFAULT_MODE)
}

crate::registry::exports! {
    "rules/tools/inputs/web-search" {
        "WEB_SEARCH_MODES" => ok || WEB_SEARCH_MODES;
        "effectiveSearchMode" => ok |input: JsValue| effective_search_mode(&input);
    }
}
