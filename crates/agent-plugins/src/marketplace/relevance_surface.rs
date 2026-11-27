//! Port of `src/marketplace/relevance-surface.ts`, switched.
//!
//! Where a matching plugin is suggested and how often (plugins/relevance.md, "Understand how plugin
//! relevance works", "Preview what the user sees"): the spinner tip, the session-start notification
//! and the `/plugin` Discover tab. Pure over what the host recorded; the texts are the docs'. The
//! counting of "sessions" and the annotations the docs do not quote are in spec/judgments.md.

use serde::Deserialize;

use super::relevance_match::SignalName;
use crate::format::marketplace::relevance::{TopicSource, effective_topic};
use crate::format::marketplace::types::Relevance;

/// A plugin a signal matched.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct SuggestedPlugin {
    /// Its name.
    pub name: String,
    /// Its marketplace.
    pub marketplace: String,
    /// Its `relevance` block.
    #[serde(default)]
    pub relevance: Option<Relevance>,
}

fn install_command(plugin: &SuggestedPlugin) -> String {
    format!("/plugin install {}@{}", plugin.name, plugin.marketplace)
}

/// "Working with Terraform? Install the terraform-helpers plugin:" and the install command on the next line.
#[must_use]
pub fn spinner_tip_text(plugin: &SuggestedPlugin) -> String {
    let topic = effective_topic(&TopicSource {
        name: plugin.name.clone(),
        relevance: plugin.relevance.clone(),
    });
    format!(
        "Working with {topic}? Install the {} plugin:\n{}",
        plugin.name,
        install_command(plugin)
    )
}

/// The one-line notification of a `cwd` match at session start.
#[must_use]
pub fn session_start_notice_text(plugin: &SuggestedPlugin) -> String {
    format!(
        "plugin suggestion: {}@{} · /plugin",
        plugin.name, plugin.marketplace
    )
}

/// What the host recorded about one plugin's suggestions.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionHistory {
    /// Sessions started since the spinner tip or the notification last showed it; `undefined` when never shown.
    #[serde(default)]
    pub sessions_since_shown: Option<f64>,
    /// Times the spinner tip and the notification together have shown it.
    pub times_shown: f64,
    /// The Discover tab already pinned it once (`~/.claude.json`).
    pub discover_pinned: bool,
}

/// What decides whether a plugin is suggested now.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionContext {
    /// It is installed.
    pub installed: bool,
    /// The signals that match now (`matchingSignals`).
    pub matched: Vec<SignalName>,
    /// What was shown before.
    pub history: SuggestionHistory,
    /// `spinnerTipsEnabled`; unset is on.
    #[serde(default)]
    pub spinner_tips_enabled: Option<bool>,
    /// A `spinnerTipsOverride` with `excludeDefault` replaces the built-in tips.
    #[serde(default)]
    pub tips_override_excludes_default: Option<bool>,
}

/// "At most once every three sessions across the spinner tip and the session-start notification combined."
const SESSIONS_BETWEEN_SUGGESTIONS: f64 = 3.0;
/// "Stops appearing once the spinner tip and the notification have shown the plugin a combined total of two times."
const NOTICE_TOTAL_LIMIT: f64 = 2.0;

fn tips_on(context: &SuggestionContext) -> bool {
    context.spinner_tips_enabled != Some(false)
        && context.tips_override_excludes_default != Some(true)
}

fn spaced(history: &SuggestionHistory) -> bool {
    history
        .sessions_since_shown
        .is_none_or(|sessions| sessions >= SESSIONS_BETWEEN_SUGGESTIONS)
}

fn eligible(context: &SuggestionContext) -> bool {
    !context.installed && !context.matched.is_empty()
}

/// Whether the spinner tip shows the plugin now.
#[must_use]
pub fn show_spinner_tip(context: &SuggestionContext) -> bool {
    eligible(context) && tips_on(context) && spaced(&context.history)
}

/// Whether the session-start notification shows the plugin: a `cwd` match, the spacing, and at most two showings.
#[must_use]
pub fn show_session_start_notice(context: &SuggestionContext) -> bool {
    eligible(context)
        && context.matched.contains(&SignalName::Cwd)
        && tips_on(context)
        && spaced(&context.history)
        && context.history.times_shown < NOTICE_TOTAL_LIMIT
}

/// The Discover tab pins the plugin the first time the tab opens while a signal matches; neither tip setting affects it.
#[must_use]
pub fn pin_in_discover(context: &SuggestionContext) -> bool {
    eligible(context) && !context.history.discover_pinned
}

/// The annotation of each signal; `command` is the matching command name of a `cli` signal.
fn annotation(signal: SignalName, command: Option<&str>) -> String {
    match signal {
        SignalName::Cwd => "suggested for this directory".to_owned(),
        SignalName::Cli => format!("suggested for {} commands", command.unwrap_or("these")),
        SignalName::Hosts => "suggested for the hosts you reach".to_owned(),
        SignalName::FilesRead => "suggested for the files you read".to_owned(),
        SignalName::ManifestDeps => "suggested for your project dependencies".to_owned(),
    }
}

/// The annotation that names the first matching signal; `command` is the matching command name of a `cli` signal.
#[must_use]
pub fn discover_annotation(matched: &[SignalName], command: Option<&str>) -> Option<String> {
    let first = matched.first()?;
    Some(annotation(*first, command))
}

crate::registry::exports! {
    "marketplace/relevance-surface" {
        "spinnerTipText" => ok |plugin: SuggestedPlugin| spinner_tip_text(&plugin);
        "sessionStartNoticeText" => ok |plugin: SuggestedPlugin| session_start_notice_text(&plugin);
        "showSpinnerTip" => ok |context: SuggestionContext| show_spinner_tip(&context);
        "showSessionStartNotice" => ok |context: SuggestionContext| show_session_start_notice(&context);
        "pinInDiscover" => ok |context: SuggestionContext| pin_in_discover(&context);
        "discoverAnnotation" => ok |matched: Vec<SignalName>, command: Option<String>| discover_annotation(&matched, command.as_deref());
    }
}
