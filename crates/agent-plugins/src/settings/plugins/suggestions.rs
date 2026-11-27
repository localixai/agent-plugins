//! Port of `src/settings/plugins/suggestions.ts`, switched.
//!
//! `pluginSuggestionMarketplaces`: the marketplaces whose plugins may surface as contextual install
//! suggestions (settings-reference.md, `pluginSuggestionMarketplaces`; plugins/relevance.md,
//! "Enable suggestions in managed settings"). The suggestions themselves come from each plugin's
//! `relevance` and render in the host's spinner tips and Discover tab; the kit decides which names
//! take effect.

use crate::js::JsValue;
use crate::settings::layers::PolicyScope;
use crate::settings::marketplaces::match_source::{ListKind, match_marketplace_source};
use crate::settings::marketplaces::registry::OFFICIAL_MARKETPLACE;
use crate::settings::marketplaces::source_validate::registration_problems;
use crate::settings::view::PolicyView;

/// Whether a source declared in the managed settings is the one the marketplace registered from.
fn same_source(declared: &JsValue, registered: &JsValue) -> bool {
    match_marketplace_source(ListKind::Strict, declared, registered)
}

/// The `extraKnownMarketplaces` entry of that name in the managed settings, "wherever" the highest
/// managed file that declares it is; an entry that cannot register a marketplace declares nothing.
async fn declared_source(view: &dyn PolicyView, name: &str) -> Option<JsValue> {
    let holders = view.each_from_highest("extraKnownMarketplaces").await;
    let holder = holders
        .iter()
        .find(|each| each.scope == PolicyScope::Managed && each.value.get(name).is_some())?;
    let source = holder.value.get(name)?.get("source")?;
    (!source.is_undefined() && registration_problems(name, source).is_empty())
        .then(|| source.clone())
}

/// The `strictKnownMarketplaces` list of the managed settings (a parent's is not "the same managed
/// settings").
async fn managed_allowlist(view: &dyn PolicyView) -> Vec<JsValue> {
    let holders = view.each_from_highest("strictKnownMarketplaces").await;
    match holders
        .into_iter()
        .find(|each| each.scope == PolicyScope::Managed)
        .map(|each| each.value)
    {
        Some(JsValue::Array(entries)) => entries,
        _ => Vec::new(),
    }
}

/// The names of `pluginSuggestionMarketplaces` that take effect, in the order the list gives them.
/// A name takes effect only when the marketplace is registered and its registered source is also
/// declared in the same managed settings: as the entry of that name in `extraKnownMarketplaces`, or
/// as an entry of `strictKnownMarketplaces` that admits it. A marketplace registered under the
/// name from another source is ignored. The official marketplace is exempt from the source
/// requirement: its name "can only register from the official Anthropic source". The key counts
/// from managed settings only (its **Scope** line). `registered` maps each registered name to its
/// source.
pub async fn suggestion_marketplaces(view: &dyn PolicyView, registered: &JsValue) -> Vec<String> {
    let mut listed: Vec<String> = Vec::new();
    if let Some(JsValue::Array(names)) = view.get("pluginSuggestionMarketplaces").await {
        for name in names.iter().filter_map(JsValue::as_str) {
            if !listed.iter().any(|known| known == name) {
                listed.push(name.to_owned());
            }
        }
    }
    let allowlist = managed_allowlist(view).await;
    let mut kept = Vec::new();
    for name in listed {
        let Some(source) = registered
            .get(&name)
            .filter(|source| !source.is_undefined())
        else {
            continue;
        };
        if name == OFFICIAL_MARKETPLACE {
            kept.push(name);
            continue;
        }
        let declared = declared_source(view, &name).await;
        if declared.is_some_and(|declared| same_source(&declared, source))
            || allowlist
                .iter()
                .any(|entry| match_marketplace_source(ListKind::Strict, entry, source))
        {
            kept.push(name);
        }
    }
    kept
}

crate::registry::exports! {
    "settings/plugins/suggestions" {
        "suggestionMarketplaces" => async ok |view: crate::registry::Port, registered: JsValue|
            suggestion_marketplaces(&view, &registered).await;
    }
}
