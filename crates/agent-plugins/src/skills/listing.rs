//! Port of `src/skills/listing.ts`, switched.
//!
//! What the model and the `/` menu are told about the skills (skills.md, "Skill descriptions are cut
//! short", "Angle brackets in a skill description", "Override skill visibility from settings";
//! settings-reference.md, `skillListingMaxDescChars` and `skillListingBudgetFraction`). Pure: the host
//! passes the settings, the usage counts and the model's context window in.

use serde::{Deserialize, Serialize};

use super::invocation::{Invocable, invocation_policy};
use super::visibility::{ModelListing, SkillOverrides};
use crate::format::skill_fields::escape_angle_brackets;
use crate::format::token_estimate::CHARS_PER_TOKEN;
use crate::js::code_unit_length;
use crate::registry::{Map, Set};

/// `skillListingMaxDescChars` default: "capped at 1,536 characters".
const DEFAULT_MAX_DESCRIPTION_CHARS: usize = 1536;
/// `skillListingBudgetFraction` default: "reserves 1% of the context window".
const DEFAULT_BUDGET_FRACTION: f64 = 0.01;
/// `SLASH_COMMAND_TOOL_CHAR_BUDGET`: "The budget scales dynamically at 1% of the context window, with a
/// fallback of 8,000 characters." The fallback applies when the host gives neither a window nor a count.
pub const FALLBACK_BUDGET_CHARS: f64 = 8000.0;

/// A skill as the listing reads it: who may invoke it, and what it says of itself.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedSkill {
    /// What decides who may invoke it.
    #[serde(flatten)]
    pub invocable: Invocable,
    /// `paths`: a skill that sets them is dormant until a touched file activates it.
    #[serde(default)]
    pub paths: Vec<String>,
    /// `description`.
    #[serde(default)]
    pub description: Option<String>,
    /// `when_to_use`.
    #[serde(default)]
    pub when_to_use: Option<String>,
    /// No `description` and no `when_to_use` were written.
    #[serde(default)]
    pub undescribed: Option<bool>,
    /// `argument-hint`.
    #[serde(default)]
    pub argument_hint: Option<String>,
}

/// The listing's budget.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingBudget {
    /// The model's context window in tokens.
    #[serde(default)]
    pub context_window: Option<f64>,
    /// `skillListingBudgetFraction`, greater than 0 and at most 1 (default 0.01).
    #[serde(default)]
    pub fraction: Option<f64>,
    /// A fixed character count (`SLASH_COMMAND_TOOL_CHAR_BUDGET`): it replaces the fraction of the window.
    #[serde(default)]
    pub chars: Option<f64>,
}

/// How the listing is made.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListingOptions {
    /// `skillOverrides`.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
    /// `skillListingMaxDescChars` (default 1536).
    #[serde(default)]
    pub max_desc_chars: Option<f64>,
    /// With neither a window nor a character count the budget is 8,000 characters.
    #[serde(default)]
    pub budget: Option<ListingBudget>,
    /// How often each skill (by command name) was invoked; absent counts as never.
    #[serde(default)]
    pub usage: Option<Map<String, f64>>,
    /// The command names of skills whose `paths` have activated them in this session.
    #[serde(default)]
    pub activated: Option<Set<String>>,
}

/// One line of the listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListingEntry {
    /// The command name.
    pub name: String,
    /// Absent for a name-only skill and for one whose description the budget dropped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A command listed ahead of the skills with its description as written: a plugin's workflow
/// (oracle 2.1.319, spec/judgments.md). The description is cut and escaped as a skill's is.
#[derive(Clone, Debug, Deserialize)]
pub struct LeadingCommand {
    /// Its name.
    pub name: String,
    /// Its description.
    pub description: String,
}

/// The model's listing.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelListingResult {
    /// The lines.
    pub entries: Vec<ListingEntry>,
    /// The skills that lost their description to the budget, in the order they lost it.
    pub descriptions_dropped: Vec<String>,
    /// The characters the listing takes.
    pub chars: f64,
    /// Still over the budget with every dropable description gone: names are never dropped.
    pub over_budget: bool,
}

/// Judgment: the docs give the content of a line, not its form.
#[must_use]
pub fn listing_line(entry: &ListingEntry) -> String {
    match &entry.description {
        None => format!("- {}", entry.name),
        Some(description) => format!("- {}: {description}", entry.name),
    }
}

fn line_cost(entry: &ListingEntry) -> usize {
    code_unit_length(&listing_line(entry)) + 1
}

fn positive_integer(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && value.fract() == 0.0 && *value > 0.0)
}

/// The first `limit` characters (code points, so a surrogate pair is never split).
fn first_characters(text: &str, limit: f64) -> String {
    text.chars()
        .enumerate()
        .take_while(|(at, _)| (*at as f64) < limit)
        .map(|(_, character)| character)
        .collect()
}

/// `description` with `when_to_use` appended, cut at the cap, then shown with `<` and `>` escaped.
/// Judgment, oracle 2.1.319: the two are joined by " - ", as Claude Code's listing shows them.
fn describe(skill: &ListedSkill, cap: f64) -> String {
    let parts: Vec<&str> = [&skill.description, &skill.when_to_use]
        .into_iter()
        .filter_map(|part| part.as_deref().filter(|part| !part.is_empty()))
        .collect();
    escape_angle_brackets(&first_characters(&parts.join(" - "), cap))
}

/// The character budget.
fn budget_chars(budget: Option<&ListingBudget>) -> f64 {
    let Some(budget) = budget else {
        return FALLBACK_BUDGET_CHARS;
    };
    if let Some(chars) = positive_integer(budget.chars) {
        return chars;
    }
    let Some(window) = positive_integer(budget.context_window) else {
        return FALLBACK_BUDGET_CHARS;
    };
    let fraction = budget.fraction.unwrap_or(DEFAULT_BUDGET_FRACTION);
    let valid = if fraction > 0.0 && fraction <= 1.0 {
        fraction
    } else {
        DEFAULT_BUDGET_FRACTION
    };
    (window * valid * CHARS_PER_TOKEN as f64).floor()
}

/// It sets `paths` and no touched file has activated it yet: the listing names it alone.
#[must_use]
pub fn is_dormant(skill: &ListedSkill, activated: Option<&Set<String>>) -> bool {
    let command = &skill.invocable.command;
    !skill.paths.is_empty() && !activated.is_some_and(|set| set.members.contains(command))
}

/// What the model's listing says of a skill before the budget: its description, its name alone (a
/// `name-only` override, a dormant `paths` skill), or nothing (`disable-model-invocation`, a hiding
/// override, no description: still invocable through the Skill tool, oracle 2.1.319).
#[must_use]
pub fn listed_as(skill: &ListedSkill, options: &ListingOptions) -> ModelListing {
    let overrides = options.overrides.clone().unwrap_or_default();
    let listed = invocation_policy(&skill.invocable, &overrides).listed_to_model;
    if listed == ModelListing::Hidden || skill.undescribed == Some(true) {
        return ModelListing::Hidden;
    }
    if is_dormant(skill, options.activated.as_ref()) {
        ModelListing::Name
    } else {
        listed
    }
}

fn uses(options: &ListingOptions, name: &str) -> f64 {
    options
        .usage
        .as_ref()
        .and_then(|usage| usage.entries.iter().find(|(key, _)| key == name))
        .map_or(0.0, |(_, count)| *count)
}

/// The skills the model reads about, with the description of the least used dropped first until the
/// listing fits the budget. Among equally used skills the one listed last loses it first (judgment).
#[must_use]
pub fn model_listing(
    skills: &[ListedSkill],
    options: &ListingOptions,
    leading: &[LeadingCommand],
) -> ModelListingResult {
    let cap =
        positive_integer(options.max_desc_chars).unwrap_or(DEFAULT_MAX_DESCRIPTION_CHARS as f64);
    let mut entries: Vec<ListingEntry> = Vec::new();
    let mut used: Vec<f64> = Vec::new();
    for row in leading {
        let description = escape_angle_brackets(&first_characters(&row.description, cap));
        entries.push(ListingEntry {
            name: row.name.clone(),
            description: Some(description),
        });
        used.push(uses(options, &row.name));
    }
    for skill in skills {
        let name = skill.invocable.command.clone();
        let entry = match listed_as(skill, options) {
            ModelListing::Hidden => continue,
            ModelListing::Name => ListingEntry {
                name,
                description: None,
            },
            ModelListing::Description => ListingEntry {
                name,
                description: Some(describe(skill, cap)),
            },
        };
        used.push(uses(options, &entry.name));
        entries.push(entry);
    }
    fit_budget(entries, &used, budget_chars(options.budget.as_ref()))
}

fn fit_budget(mut entries: Vec<ListingEntry>, used: &[f64], limit: f64) -> ModelListingResult {
    let mut order: Vec<(usize, f64)> = entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.description.is_some())
        .map(|(at, _)| (at, used.get(at).copied().unwrap_or(0.0)))
        .collect();
    order.sort_by(|left, right| left.1.total_cmp(&right.1).then(right.0.cmp(&left.0)));
    let mut chars: usize = entries.iter().map(line_cost).sum();
    let mut dropped = Vec::new();
    for (at, _) in order {
        if chars as f64 <= limit {
            break;
        }
        let Some(entry) = entries.get_mut(at) else {
            continue;
        };
        let bare = ListingEntry {
            name: entry.name.clone(),
            description: None,
        };
        chars = chars + line_cost(&bare) - line_cost(entry);
        dropped.push(entry.name.clone());
        *entry = bare;
    }
    ModelListingResult {
        entries,
        descriptions_dropped: dropped,
        chars: chars as f64,
        over_budget: chars as f64 > limit,
    }
}

/// The options of the `/` menu.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct MenuOptions {
    /// `skillOverrides`.
    #[serde(default)]
    pub overrides: Option<SkillOverrides>,
}

/// One row of the `/` menu, in the official `SlashCommand` shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuRow {
    /// The command name.
    pub name: String,
    /// The description, `<` and `>` escaped.
    pub description: String,
    /// `argument-hint` as written.
    pub argument_hint: String,
}

/// The rows of the `/` menu, in the official `SlashCommand` shape: the skills a user may type
/// (`user-invocable: false` and an `off` override are hidden). The row shows the description with
/// `<` and `>` escaped; judgment: not `when_to_use`, which the docs append only "in the skill
/// listing", and not capped. `argument-hint` is shown as written.
#[must_use]
pub fn menu_entries(
    skills: &[ListedSkill],
    options: &MenuOptions,
    leading: &[LeadingCommand],
) -> Vec<MenuRow> {
    let overrides = options.overrides.clone().unwrap_or_default();
    let first = leading.iter().map(|row| MenuRow {
        name: row.name.clone(),
        description: escape_angle_brackets(&row.description),
        argument_hint: String::new(),
    });
    let rest = skills
        .iter()
        .filter(|skill| invocation_policy(&skill.invocable, &overrides).in_menu)
        .map(|skill| MenuRow {
            name: skill.invocable.command.clone(),
            description: escape_angle_brackets(skill.description.as_deref().unwrap_or_default()),
            argument_hint: skill.argument_hint.clone().unwrap_or_default(),
        });
    first.chain(rest).collect()
}

crate::registry::exports! {
    "skills/listing" {
        "FALLBACK_BUDGET_CHARS" => ok || FALLBACK_BUDGET_CHARS;
        "listingLine" => ok |entry: ListingEntry| listing_line(&entry);
        "isDormant" => ok |skill: ListedSkill, activated: Option<Set<String>>| is_dormant(&skill, activated.as_ref());
        "listedAs" => ok |skill: ListedSkill, options: Option<ListingOptions>| listed_as(&skill, &options.unwrap_or_default());
        "modelListing" => ok |skills: Vec<ListedSkill>, options: Option<ListingOptions>, leading: Option<Vec<LeadingCommand>>|
            model_listing(&skills, &options.unwrap_or_default(), &leading.unwrap_or_default());
        "menuEntries" => ok |skills: Vec<ListedSkill>, options: Option<MenuOptions>, leading: Option<Vec<LeadingCommand>>|
            menu_entries(&skills, &options.unwrap_or_default(), &leading.unwrap_or_default());
    }
}
