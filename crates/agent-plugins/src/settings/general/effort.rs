//! Port of `src/settings/general/effort.ts`, switched.
//!
//! The effort level a settings file saves for one model (settings-reference.md, `modelSettings`
//! and `effortLevel`): "A model's `effortLevel` here takes precedence over the top-level
//! `effortLevel` in the same settings file. Across files, Claude Code resolves each model
//! separately: the highest-precedence settings file that sets either an `effortLevel` for that
//! model or a top-level `effortLevel` that applies to that model decides". `modelSettings` is so no
//! merged object: the entry of a model in a lower file is never combined with a higher file's. A
//! file saves only the levels `SAVED_EFFORT_LEVELS` lists: `max` is valid for `maxEffortLevel`
//! alone.

use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::protocol::effort::{EFFORT_LEVELS, SAVED_EFFORT_LEVELS};
use crate::protocol::is_record::is_record;
use crate::settings::general::exceptions::{LOWEST_CAP, strictest_of};
use crate::settings::key_spec::admits;
use crate::settings::layers::{PolicyLayer, PolicyScope, ordered_layers};

/// The level a file saves for a model, and the file.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ModelEffort {
    /// One of `SAVED_EFFORT_LEVELS`.
    pub level: &'static str,
    /// The file's scope.
    pub from: PolicyScope,
}

/// What the host says about the model.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelEffortFacts {
    /// The model's canonical name (`claude-opus-5-5`): the host maps the alias, dated, `[1m]` and
    /// provider spellings to it.
    pub model: String,
    /// The top-level `effortLevel` of the user file applies to this model: in user settings it
    /// "keeps applying where it applied before, on Opus 5, Fable 5.1, and earlier models"; in the
    /// other files it applies to every model. Which models are "earlier" is the host's to say.
    pub user_level_applies: bool,
}

fn saved_level(value: Option<&JsValue>) -> Option<&'static str> {
    let text = value?.as_str()?;
    SAVED_EFFORT_LEVELS
        .iter()
        .copied()
        .find(|level| *level == text)
}

fn effort_of(settings: Option<&JsValue>) -> Option<&'static str> {
    let settings = settings.filter(|settings| is_record(settings))?;
    saved_level(settings.get("effortLevel"))
}

fn model_entry<'a>(layer: &'a PolicyLayer, model: &str) -> Option<&'a JsValue> {
    let model_settings = layer.settings.get("modelSettings")?;
    if is_record(model_settings) {
        model_settings.get(model)
    } else {
        None
    }
}

/// What one file says about the model: its saved level, else the top-level one where that
/// applies.
fn file_level(layer: &PolicyLayer, facts: &ModelEffortFacts) -> Option<&'static str> {
    if let Some(saved) = effort_of(model_entry(layer, &facts.model)) {
        return Some(saved);
    }
    let top_level_applies = layer.scope != PolicyScope::User || facts.user_level_applies;
    if top_level_applies {
        effort_of(Some(&layer.settings))
    } else {
        None
    }
}

/// The level the files give a model, or `None` when none does.
#[must_use]
pub fn effort_level_for(layers: &[PolicyLayer], facts: &ModelEffortFacts) -> Option<ModelEffort> {
    ordered_layers(layers).iter().find_map(|layer| {
        file_level(layer, facts).map(|level| ModelEffort {
            level,
            from: layer.scope,
        })
    })
}

fn level_of(value: Option<&JsValue>) -> Option<&'static str> {
    let text = value?.as_str()?;
    EFFORT_LEVELS.iter().copied().find(|level| *level == text)
}

fn rank_of(level: &str) -> Option<usize> {
    EFFORT_LEVELS.iter().position(|one| *one == level)
}

/// The cap one file sets for a model (settings-reference.md, `maxEffortLevel`, "Per-model caps"):
/// the model's own `maxEffortLevel` "replaces this key for the model only within the settings
/// source that sets both", so it stands in for the file's top-level cap, `"max"` among them (no
/// cap).
fn file_cap(layer: &PolicyLayer, model: &str) -> Option<&'static str> {
    let entry = model_entry(layer, model).filter(|entry| is_record(entry));
    let own = level_of(entry.and_then(|entry| entry.get("maxEffortLevel")));
    let cap = own.or_else(|| level_of(layer.settings.get("maxEffortLevel")))?;
    admits("maxEffortLevel", layer.scope, &JsValue::from(cap)).then_some(cap)
}

/// The cap that applies to a model: "When several scopes set a cap, the lowest applies, so a cap
/// set in one scope can't be raised from another", and a file's `"max"` exempts the model from
/// that file's cap only, "Claude Code still applies caps from other sources". `None` when no file
/// caps the model (a `"max"` sets no cap). An organization's own effort limit for a model is not
/// in the files: the host takes the lower of the two with `capEffort`.
#[must_use]
pub fn effective_cap(layers: &[PolicyLayer], model: &str) -> Option<String> {
    let caps: Vec<JsValue> = layers
        .iter()
        .filter_map(|layer| file_cap(layer, model).map(JsValue::from))
        .collect();
    let lowest = strictest_of(LOWEST_CAP.ladder.clone(), &caps);
    lowest
        .as_str()
        .filter(|level| *level != "max")
        .map(str::to_owned)
}

/// The level a session runs at: `level`, or the cap when `level` is higher ("Any higher level runs
/// at the cap instead").
#[must_use]
pub fn cap_effort(level: String, cap: Option<String>) -> String {
    match cap {
        Some(cap) if rank_of(&level) > rank_of(&cap) => cap,
        _ => level,
    }
}

crate::registry::exports! {
    "settings/general/effort" {
        "effortLevelFor" => ok |layers: Vec<PolicyLayer>, facts: ModelEffortFacts| effort_level_for(&layers, &facts);
        "effectiveCap" => ok |layers: Vec<PolicyLayer>, model: String| effective_cap(&layers, &model);
        "capEffort" => ok |level: String, cap: Option<String>| cap_effort(level, cap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_higher_level_runs_at_the_cap() {
        assert_eq!(
            cap_effort("max".to_owned(), Some("high".to_owned())),
            "high"
        );
        assert_eq!(cap_effort("low".to_owned(), Some("high".to_owned())), "low");
        assert_eq!(cap_effort("low".to_owned(), None), "low");
    }
}
