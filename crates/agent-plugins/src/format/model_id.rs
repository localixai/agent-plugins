//! Port of `src/format/model-id.ts`, switched.
//!
//! Whether a model ID names a model name, family alias or version prefix: the name is the whole
//! ID, a prefix of it, or starts one of its dash-separated parts (`opus` in `claude-opus-5-5` and
//! in `my-gateway/claude-opus-5-5`). One reading for `availableModels` entries ("`opus` allows
//! any opus version", "a custom ID that embeds a family name ... counts as a specific entry for
//! that family") and for the family of a model ID (model-config.md, "Restrict model selection",
//! "Add a custom model option"). Judgment where the docs give only examples
//! (spec/judgments.md).

use crate::js::trim;

/// Whether `model` is named by `name`, as above.
#[must_use]
pub fn model_id_names(model: &str, name: &str) -> bool {
    let id = trim(model).to_lowercase();
    let named = trim(name).to_lowercase();
    !named.is_empty() && (id.starts_with(&named) || id.contains(&format!("-{named}")))
}

crate::registry::exports! {
    "format/model-id" {
        "modelIdNames" => ok |model: String, name: String| model_id_names(&model, &name);
    }
}
