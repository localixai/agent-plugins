//! Port of `src/hooks/select/model-switch.ts`, switched.

use crate::hooks::contracts::ModelCatalog;
use crate::registry::Port;

/// hooks.md, "PreModelSwitch": the matcher ignores "any `[1m]` suffix". (`/\[1m\]$/`)
const LONG_CONTEXT_SUFFIX: &str = "[1m]";

fn without_suffix(model: &str) -> String {
    model
        .strip_suffix(LONG_CONTEXT_SUFFIX)
        .unwrap_or(model)
        .to_owned()
}

/// The canonical name of the model a switch goes to, which `PreModelSwitch` and `PostModelSwitch`
/// matchers compare against: an alias, a dated id and a provider id all resolve to the one name
/// (the catalog's job, the kit holds no model list). `None` when no canonical name can be
/// determined, so every hook of the event runs, whatever its matcher.
pub async fn canonical_matcher_model(
    to_model: &str,
    catalog: Option<&impl ModelCatalog>,
) -> Option<String> {
    let canonical = match catalog {
        Some(catalog) => catalog.canonical_model(&without_suffix(to_model)).await,
        None => None,
    };
    canonical.map(|canonical| without_suffix(&canonical))
}

crate::registry::exports! {
    "hooks/select/model-switch" {
        "canonicalMatcherModel" => async ok |to_model: String, catalog: Option<Port>|
            canonical_matcher_model(&to_model, catalog.as_ref()).await;
    }
}
