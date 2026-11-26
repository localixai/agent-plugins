//! Port of `src/marketplace/plugin-hints.ts`, switched.
//!
//! The `<claude-code-hint />` tag a CLI writes to stderr to recommend its plugin (plugins/cli-hints.md,
//! "Emit the hint", "Hint format", "Check when the prompt appears", "Preview what the user sees"): pure,
//! over output text and facts the host holds. The kit strips the lines and decides whether a prompt
//! shows; the host draws the dialog and installs at user scope.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::marketplace::marketplace_name_lists::OFFICIAL_NAMES;
use crate::js::{Pattern, SPACE_CLASS, is_whitespace, trim};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// A hint that parsed: `value` is `name@marketplace`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginHint {
    /// The format version.
    pub v: String,
    /// What it recommends.
    pub r#type: String,
    /// `name@marketplace`.
    pub value: String,
}

/// `/^\s*<claude-code-hint\s+([^<>]*?)\s*\/>\s*$/`.
static TAG: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"^[{SPACE_CLASS}]*<claude-code-hint[{SPACE_CLASS}]+([^<>]*?)[{SPACE_CLASS}]*/>[{SPACE_CLASS}]*$"
    ))
    .ok()
});
/// `/([A-Za-z][A-Za-z0-9_-]*)=(?:"([^"]*)"|([^\s"<>=]+))/g`.
static ATTRIBUTE: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r#"([A-Za-z][A-Za-z0-9_-]*)=(?:"([^"]*)"|([^{SPACE_CLASS}"<>=]+))"#
    ))
    .ok()
});

/// The three required attributes, double-quoted or unquoted (an unquoted value holds no whitespace).
fn parse_tag(body: &str) -> Option<PluginHint> {
    let attribute = ATTRIBUTE.as_ref()?;
    let mut found: HashMap<&str, &str> = HashMap::new();
    for captures in attribute.captures_iter(body) {
        let name = captures.get(1).map_or("", |name| name.as_str());
        let value = captures
            .get(2)
            .or_else(|| captures.get(3))
            .map_or("", |value| value.as_str());
        found.insert(name, value);
    }
    /* An unquoted value cannot hold whitespace, so a stray word is no attribute list (spec/judgments.md). */
    if !trim(&attribute.replace_all(body, "")).is_empty() {
        return None;
    }
    let v = found.get("v")?;
    let r#type = found.get("type")?;
    let value = found.get("value")?;
    Some(PluginHint {
        v: (*v).to_owned(),
        r#type: (*r#type).to_owned(),
        value: (*value).to_owned(),
    })
}

/// What `stripHints` leaves and finds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StrippedOutput {
    /// The output the model sees.
    pub output: String,
    /// Every hint line removed, in order; a line whose attributes are incomplete is removed but not listed.
    pub hints: Vec<PluginHint>,
}

/// "The tag must occupy its own line; Claude Code ignores a tag embedded mid-line." A tag on its own line
/// is removed from the output "even when `v` or `type` is unrecognized".
#[must_use]
pub fn strip_hints(output: &str) -> StrippedOutput {
    let mut kept: Vec<&str> = Vec::new();
    let mut hints: Vec<PluginHint> = Vec::new();
    for line in output.split('\n') {
        let own = line.strip_suffix('\r').unwrap_or(line);
        let body = TAG
            .as_ref()
            .and_then(|tag| tag.captures(own))
            .map(|tag| tag.get(1).map_or("", |body| body.as_str()));
        let Some(body) = body else {
            kept.push(line);
            continue;
        };
        if let Some(hint) = parse_tag(body) {
            hints.push(hint);
        }
    }
    StrippedOutput {
        output: kept.join("\n"),
        hints,
    }
}

/// What a supported hint names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HintPlugin {
    /// The plugin.
    pub plugin: String,
    /// Its marketplace.
    pub marketplace: String,
}

const SUPPORTED_VERSIONS: [&str; 1] = ["1"];
const SUPPORTED_TYPES: [&str; 1] = ["plugin"];

/// `v` `1` and `type` `plugin` are "the only supported value"; `value` is `name@marketplace`.
#[must_use]
pub fn supported_hint(hint: &PluginHint) -> Option<HintPlugin> {
    if !SUPPORTED_VERSIONS.contains(&hint.v.as_str())
        || !SUPPORTED_TYPES.contains(&hint.r#type.as_str())
    {
        return None;
    }
    let (plugin, marketplace) = hint.value.split_once('@')?;
    if plugin.is_empty() || marketplace.is_empty() || marketplace.contains('@') {
        return None;
    }
    Some(HintPlugin {
        plugin: plugin.to_owned(),
        marketplace: marketplace.to_owned(),
    })
}

/// "Emit the tag only when `CLAUDECODE` or `CLAUDE_CODE_CHILD_SESSION` is set": what a CLI's gate reads.
pub async fn should_emit_hint(env: &dyn Environment) -> bool {
    env.get("CLAUDECODE").await.unwrap_or_default() != ""
        || env
            .get("CLAUDE_CODE_CHILD_SESSION")
            .await
            .unwrap_or_default()
            != ""
}

/// The one-line tag for a plugin in an official marketplace.
#[must_use]
pub fn hint_line(value: &str) -> String {
    format!("<claude-code-hint v=\"1\" type=\"plugin\" value=\"{value}\" />")
}

/// "One prompt ever per plugin ... and none once 100 plugins have been prompted for on that machine."
pub const HINT_PROMPT_LIMIT: usize = 100;

/// What the host knows when a hint arrives.
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's interface: one flag per fact the host reads"
)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptFacts {
    /// An interactive terminal session; `claude -p`, a subagent run and hook command output are not.
    pub interactive_terminal: bool,
    /// The workspace is local and the session is attended (not `--cloud`, Remote Control or a teammate).
    pub local_attended: bool,
    /// The plugin is in the local copy of an official marketplace, not installed, and no policy blocks it.
    pub installable: bool,
    /// Claude Code's analytics are on.
    pub analytics_on: bool,
    /// A prompt already showed in this session.
    pub prompted_this_session: bool,
    /// The plugins already prompted for on this machine.
    pub prompted_plugins: Vec<String>,
    /// The user chose "No, and don't show plugin installation hints again".
    pub hints_turned_off: bool,
}

/// Why no prompt shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum PromptRefusal {
    /// `unsupported`.
    #[serde(rename = "unsupported")]
    Unsupported,
    /// `not-official`.
    #[serde(rename = "not-official")]
    NotOfficial,
    /// `not-interactive`.
    #[serde(rename = "not-interactive")]
    NotInteractive,
    /// `not-installable`.
    #[serde(rename = "not-installable")]
    NotInstallable,
    /// `analytics-off`.
    #[serde(rename = "analytics-off")]
    AnalyticsOff,
    /// `already-prompted-this-session`.
    #[serde(rename = "already-prompted-this-session")]
    AlreadyPromptedThisSession,
    /// `already-prompted-for-plugin`.
    #[serde(rename = "already-prompted-for-plugin")]
    AlreadyPromptedForPlugin,
    /// `prompt-limit`.
    #[serde(rename = "prompt-limit")]
    PromptLimit,
    /// `hints-turned-off`.
    #[serde(rename = "hints-turned-off")]
    HintsTurnedOff,
    /// `not-local-attended`.
    #[serde(rename = "not-local-attended")]
    NotLocalAttended,
}

/// Whether the dialog shows, and for what.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum PromptVerdict {
    /// `{ show: true, plugin, marketplace }`.
    Show {
        /// Always true.
        show: bool,
        /// The plugin.
        plugin: String,
        /// Its marketplace.
        marketplace: String,
    },
    /// `{ show: false, reason }`.
    Hidden {
        /// Always false.
        show: bool,
        /// Why not.
        reason: PromptRefusal,
    },
}

type Check = fn(&HintPlugin, &PromptFacts) -> Option<PromptRefusal>;

/// The checks, in the docs' order; every one must pass.
const CHECKS: [Check; 9] = [
    |context, _| {
        if OFFICIAL_NAMES.contains(&context.marketplace.as_str()) {
            None
        } else {
            Some(PromptRefusal::NotOfficial)
        }
    },
    |_, facts| (!facts.interactive_terminal).then_some(PromptRefusal::NotInteractive),
    |_, facts| (!facts.installable).then_some(PromptRefusal::NotInstallable),
    |_, facts| (!facts.analytics_on).then_some(PromptRefusal::AnalyticsOff),
    |_, facts| {
        facts
            .prompted_this_session
            .then_some(PromptRefusal::AlreadyPromptedThisSession)
    },
    |context, facts| {
        facts
            .prompted_plugins
            .contains(&format!("{}@{}", context.plugin, context.marketplace))
            .then_some(PromptRefusal::AlreadyPromptedForPlugin)
    },
    |_, facts| {
        (facts.prompted_plugins.len() >= HINT_PROMPT_LIMIT).then_some(PromptRefusal::PromptLimit)
    },
    |_, facts| {
        facts
            .hints_turned_off
            .then_some(PromptRefusal::HintsTurnedOff)
    },
    |_, facts| (!facts.local_attended).then_some(PromptRefusal::NotLocalAttended),
];

/// Whether the **Plugin recommendation** dialog shows for a hint.
#[must_use]
pub fn prompt_verdict(hint: &PluginHint, facts: &PromptFacts) -> PromptVerdict {
    let Some(parsed) = supported_hint(hint) else {
        return PromptVerdict::Hidden {
            show: false,
            reason: PromptRefusal::Unsupported,
        };
    };
    for check in CHECKS {
        if let Some(reason) = check(&parsed, facts) {
            return PromptVerdict::Hidden {
                show: false,
                reason,
            };
        }
    }
    PromptVerdict::Show {
        show: true,
        plugin: parsed.plugin,
        marketplace: parsed.marketplace,
    }
}

/// The three answers of the dialog.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HintAnswer {
    /// Yes, install.
    Install,
    /// No.
    No,
    /// No, and don't show hints again.
    Never,
    /// No answer in time.
    Timeout,
}

/// What an answer does.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HintEffect {
    /// Install the plugin at user scope.
    pub install: bool,
    /// Turn off future hint prompts for the user.
    pub turn_off: bool,
}

/// "No answer for 30 seconds: counts as No."
pub const HINT_ANSWER_TIMEOUT_MS: u32 = 30_000;

/// What an answer does.
#[must_use]
pub const fn answer_effect(answer: HintAnswer) -> HintEffect {
    match answer {
        HintAnswer::Install => HintEffect {
            install: true,
            turn_off: false,
        },
        HintAnswer::No | HintAnswer::Timeout => HintEffect {
            install: false,
            turn_off: false,
        },
        HintAnswer::Never => HintEffect {
            install: false,
            turn_off: true,
        },
    }
}

/// The dialog's text: the command's first word, the plugin and the marketplace.
#[must_use]
pub fn dialog_lines(
    command: &str,
    plugin: &str,
    marketplace: &str,
    description: Option<&str>,
) -> Vec<String> {
    let first = trim(command).split(is_whitespace).next().unwrap_or("");
    let mut lines = vec![
        "Plugin recommendation".to_owned(),
        format!("The {first} command suggests installing a plugin."),
        format!("Plugin: {plugin}"),
        format!("Marketplace: {marketplace}"),
    ];
    if let Some(description) = description {
        lines.push(format!("Description: {description}"));
    }
    lines.extend([
        "Would you like to install it?".to_owned(),
        "1. Yes, install".to_owned(),
        "2. No".to_owned(),
        "3. No, and don't show plugin installation hints again".to_owned(),
    ]);
    lines
}

crate::registry::exports! {
    "marketplace/plugin-hints" {
        "stripHints" => ok |output: String| strip_hints(&output);
        "supportedHint" => ok |hint: PluginHint| supported_hint(&hint);
        "shouldEmitHint" => async ok |env: Port| should_emit_hint(&env).await;
        "hintLine" => ok |value: String| hint_line(&value);
        "HINT_PROMPT_LIMIT" => ok || HINT_PROMPT_LIMIT;
        "promptVerdict" => ok |hint: PluginHint, facts: PromptFacts| prompt_verdict(&hint, &facts);
        "HINT_ANSWER_TIMEOUT_MS" => ok || HINT_ANSWER_TIMEOUT_MS;
        "answerEffect" => ok |answer: HintAnswer| answer_effect(answer);
        "dialogLines" => ok |command: String, plugin: String, marketplace: String, description: Option<String>|
            dialog_lines(&command, &plugin, &marketplace, description.as_deref());
    }
}
