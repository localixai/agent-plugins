//! Port of `src/skills/visibility.ts`, switched.
//!
//! The `skillOverrides` setting (skills.md, "Override skill visibility from settings"): each key is
//! a skill name and each value one of four states that decides who is told about the skill. Pure:
//! the host reads the settings and passes the map in. The invocation policy (`invocation.ts`), the
//! model listing and the `/` menu (`listing.ts`) all read the states through this one registry.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::skill_types::origin_traits;

/// `skillOverrides` as the SDK types it: skill name to one of four states.
pub type SkillOverrides = IndexMap<String, OverrideState>;

/// One of the four states.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OverrideState {
    /// Listed and in the menu.
    #[serde(rename = "on")]
    On,
    /// Listed by name only.
    #[serde(rename = "name-only")]
    NameOnly,
    /// Only the user can invoke it.
    #[serde(rename = "user-invocable-only")]
    UserInvocableOnly,
    /// Hidden from both.
    #[serde(rename = "off")]
    Off,
}

/// The four states, in the order the docs list them.
pub const OVERRIDE_STATES: [OverrideState; 4] = [
    OverrideState::On,
    OverrideState::NameOnly,
    OverrideState::UserInvocableOnly,
    OverrideState::Off,
];

/// What the model is told about a skill: name and description, name only, or nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelListing {
    /// Name and description.
    Description,
    /// Name only.
    Name,
    /// Nothing.
    Hidden,
}

/// What one state leaves visible: "Listed to Claude" and "In `/` menu" of the docs table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Exposure {
    /// What the model is told.
    pub model: ModelListing,
    /// Whether the `/` menu lists it.
    pub user: bool,
}

/// The docs table, a row per state: the registry the policy, the listing and the menu read.
#[must_use]
pub const fn override_exposure(state: OverrideState) -> Exposure {
    match state {
        OverrideState::On => Exposure {
            model: ModelListing::Description,
            user: true,
        },
        OverrideState::NameOnly => Exposure {
            model: ModelListing::Name,
            user: true,
        },
        OverrideState::UserInvocableOnly => Exposure {
            model: ModelListing::Hidden,
            user: true,
        },
        OverrideState::Off => Exposure {
            model: ModelListing::Hidden,
            user: false,
        },
    }
}

/// Where a skill came from: its kind.
#[derive(Clone, Debug, Deserialize)]
pub struct Origin {
    /// The origin's kind.
    pub kind: String,
}

/// What the state of a skill reads of it (`Pick<Skill | Command, 'name' | 'origin'>`).
#[derive(Clone, Debug, Deserialize)]
pub struct Overridable {
    /// The skill's name.
    pub name: String,
    /// Where it came from.
    pub origin: Origin,
}

/// The state in force for a skill: absent from `skillOverrides` is `on`, and a plugin skill is never overridden.
#[must_use]
pub fn override_state(skill: &Overridable, overrides: &SkillOverrides) -> OverrideState {
    let honours = origin_traits()
        .get(skill.origin.kind.as_str())
        .is_some_and(|traits| traits.honours_skill_overrides);
    if !honours {
        return OverrideState::On;
    }
    overrides
        .get(&skill.name)
        .copied()
        .unwrap_or(OverrideState::On)
}

/// What the skill's state leaves visible to Claude and to the user.
#[must_use]
pub fn exposure_of(skill: &Overridable, overrides: &SkillOverrides) -> Exposure {
    override_exposure(override_state(skill, overrides))
}

/// What the `/skills` menu shows for a state: "The `/skills` menu labels the `user-invocable-only` state `user-only`".
#[must_use]
pub const fn override_menu_label(state: OverrideState) -> &'static str {
    match state {
        OverrideState::On => "on",
        OverrideState::NameOnly => "name-only",
        OverrideState::UserInvocableOnly => "user-only",
        OverrideState::Off => "off",
    }
}

crate::registry::exports! {
    "skills/visibility" {
        "OVERRIDE_STATES" => ok || OVERRIDE_STATES;
        "OVERRIDE_EXPOSURE" => ok || OVERRIDE_STATES
            .map(|state| (state, override_exposure(state)))
            .into_iter()
            .collect::<IndexMap<_, _>>();
        "overrideState" => ok |skill: Overridable, overrides: Option<SkillOverrides>|
            override_state(&skill, &overrides.unwrap_or_default());
        "exposureOf" => ok |skill: Overridable, overrides: Option<SkillOverrides>|
            exposure_of(&skill, &overrides.unwrap_or_default());
        "overrideMenuLabel" => ok |state: OverrideState| override_menu_label(state);
    }
}
