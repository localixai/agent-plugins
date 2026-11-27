//! Port of `src/settings/general/session-overrides.ts` at sha256:67f6ac2c941c.
//!
//! The ways to change one setting for one session without saving it (settings.md, "Change a
//! setting for one session"): a flag for that key and an environment variable paired with it.
//! Environment variables "aren't a level in this stack": which one applies is decided per pair
//! (env-vars.md, "Precedence"; model-config.md, "Setting your model"; settings-reference.md,
//! `effortLevel`). The table `SESSION_OVERRIDES` holds readers (functions), so it stays
//! TypeScript with the module: only `resolveSessionValue` is exported.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::model_id::model_id_names;
use crate::js::trim;

/// What the session knows of one key's override.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInput {
    /// The command-line flags, by name (`--model`), with the value given.
    pub flags: IndexMap<String, Option<String>>,
    /// The host's environment.
    pub env: IndexMap<String, Option<String>>,
    /// The key's value as the settings files combine it.
    pub settings_value: Option<String>,
    /// The `availableModels` list as the files combine it (`undefined` when none sets it). A flag
    /// such as `--model` "picks only from the models your organization allows" (settings.md,
    /// "Settings precedence"); `[]` blocks every named selection (model-config.md, "Default model
    /// behavior").
    #[serde(default)]
    pub available_models: Option<Vec<String>>,
}

/// Where a source reads its value.
#[derive(Clone, Copy, Debug)]
pub enum Reader {
    /// A command-line flag.
    Flag(&'static str),
    /// An environment variable.
    Variable(&'static str),
    /// The settings files.
    Files,
}

/// Whether a value a source read counts.
type Admit = fn(&str, &SessionInput) -> bool;

/// One place a key's value can come from, as a reader of the session input.
#[derive(Clone, Copy, Debug)]
pub struct OverrideSource {
    /// Its name.
    pub name: &'static str,
    /// How it reads.
    pub read: Reader,
    /// A value the source reads counts only when this says so; absent, any value counts.
    pub admit: Option<Admit>,
}

impl OverrideSource {
    /// What the source reads of the input.
    fn read(&self, input: &SessionInput) -> Option<String> {
        match self.read {
            Reader::Flag(name) => input.flags.get(name).cloned().flatten(),
            Reader::Variable(name) => input.env.get(name).cloned().flatten(),
            Reader::Files => input.settings_value.clone(),
        }
    }
}

const fn flag(name: &'static str, admit: Option<Admit>) -> OverrideSource {
    OverrideSource {
        name,
        read: Reader::Flag(name),
        admit,
    }
}
const fn variable(name: &'static str) -> OverrideSource {
    OverrideSource {
        name,
        read: Reader::Variable(name),
        admit: None,
    }
}
const fn files(admit: Option<Admit>) -> OverrideSource {
    OverrideSource {
        name: "settings files",
        read: Reader::Files,
        admit,
    }
}

/// Whether `availableModels` lets a model be selected. The SDK types the entries: "family aliases
/// (`opus` allows any opus version), version prefixes (`opus-4-5` allows that version and any model
/// ID that extends it, so `claude-opus-5` also allows `claude-opus-5-5`), and full model IDs". The
/// kit reads that as: the entry is the whole name, a prefix of it, or starts one of its
/// dash-separated parts (spec/judgments.md).
fn model_allowed(value: &str, input: &SessionInput) -> bool {
    match &input.available_models {
        None => true,
        Some(models) => models.iter().any(|entry| model_id_names(value, entry)),
    }
}

/// A usable `CLAUDE_CODE_AUTO_COMPACT_WINDOW`: "from `100000` to `1000000` tokens", written as a
/// number, or with `k` or `M` ("`200k`", "`1M`"). A number with no suffix is read as that many
/// tokens. Any other value is one "Claude Code ignores" (model-config.md, "Values and warnings").
/// The grammar of the suffix beyond those two examples is a judgment (spec/judgments.md).
/// (`/^(\d+(?:\.\d+)?)([kKmM]?)$/`)
fn usable_window(value: &str, _input: &SessionInput) -> bool {
    let text = trim(value);
    let (number, unit) = match text.char_indices().last() {
        Some((at, last)) if matches!(last, 'k' | 'K' | 'm' | 'M') => (
            text.get(..at).unwrap_or_default(),
            last.to_ascii_lowercase(),
        ),
        Some(_) | None => (text, ' '),
    };
    if !is_decimal(number) {
        return false;
    }
    let multiplier = match unit {
        'k' => 1000.0,
        'm' => 1_000_000.0,
        _ => 1.0,
    };
    let tokens = number.parse::<f64>().unwrap_or(f64::NAN) * multiplier;
    (100_000.0..=1_000_000.0).contains(&tokens)
}

/// `^\d+(?:\.\d+)?$` with `\d` ASCII (RS6).
fn is_decimal(text: &str) -> bool {
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    match text.split_once('.') {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(text),
    }
}

fn variable_when(name: &'static str, admit: Admit) -> OverrideSource {
    OverrideSource {
        admit: Some(admit),
        ..variable(name)
    }
}

/// The keys with a per-session override.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverriddenKey {
    /// `agent`.
    #[serde(rename = "agent")]
    Agent,
    /// `teammateMode`.
    #[serde(rename = "teammateMode")]
    TeammateMode,
    /// `processWrapper`.
    #[serde(rename = "processWrapper")]
    ProcessWrapper,
    /// `model`.
    #[serde(rename = "model")]
    Model,
    /// `effortLevel`.
    #[serde(rename = "effortLevel")]
    EffortLevel,
    /// `modelSettings`.
    #[serde(rename = "modelSettings")]
    ModelSettings,
    /// `permissions.defaultMode`.
    #[serde(rename = "permissions.defaultMode")]
    PermissionsDefaultMode,
    /// `autoCompactWindow`.
    #[serde(rename = "autoCompactWindow")]
    AutoCompactWindow,
    /// `autoConnectIde`.
    #[serde(rename = "autoConnectIde")]
    AutoConnectIde,
}

/// `--effort` for the level, with `CLAUDE_CODE_EFFORT_LEVEL` over both and the files under.
fn effort() -> Vec<OverrideSource> {
    vec![
        variable("CLAUDE_CODE_EFFORT_LEVEL"),
        flag("--effort", None),
        files(None),
    ]
}

/// The sources of a key, highest first.
#[must_use]
pub fn session_overrides(key: OverriddenKey) -> Vec<OverrideSource> {
    match key {
        OverriddenKey::Agent => vec![flag("--agent", None), files(None)],
        OverriddenKey::TeammateMode => vec![flag("--teammate-mode", None), files(None)],
        OverriddenKey::ProcessWrapper => {
            vec![variable("CLAUDE_CODE_PROCESS_WRAPPER"), files(None)]
        }
        // `--model` overrides `ANTHROPIC_MODEL`, which overrides the `model` key;
        // `ANTHROPIC_DEFAULT_MODEL` "applies only when no file sets `model`".
        // A flag and a file value are gated by `availableModels`; the variable is not named there.
        OverriddenKey::Model => vec![
            flag("--model", Some(model_allowed)),
            variable("ANTHROPIC_MODEL"),
            files(Some(model_allowed)),
            variable("ANTHROPIC_DEFAULT_MODEL"),
        ],
        // "`--effort` takes precedence over this key for one session, and
        // `CLAUDE_CODE_EFFORT_LEVEL` takes precedence over both".
        OverriddenKey::EffortLevel => effort(),
        // "`--effort` for `effortLevel` and `modelSettings`": the settings value is the level saved
        // for the model.
        OverriddenKey::ModelSettings => effort(),
        // permission-modes.md, "Which mode a session starts in": "The `--permission-mode` flag"
        // comes first, then `permissions.defaultMode`.
        OverriddenKey::PermissionsDefaultMode => {
            vec![flag("--permission-mode", None), files(None)]
        }
        // "`--autocompact` takes precedence over this key for one session, and
        // `CLAUDE_CODE_AUTO_COMPACT_WINDOW` takes precedence over both unless Claude Code ignores
        // the variable's value" (settings-reference.md).
        OverriddenKey::AutoCompactWindow => vec![
            variable_when("CLAUDE_CODE_AUTO_COMPACT_WINDOW", usable_window),
            flag("--autocompact", None),
            files(None),
        ],
        // "Claude Code reads the variable first and uses the `autoConnectIde` setting only when the
        // variable is unset."
        OverriddenKey::AutoConnectIde => {
            vec![variable("CLAUDE_CODE_AUTO_CONNECT_IDE"), files(None)]
        }
    }
}

/// The value a session uses and the source that supplied it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionValue {
    /// The value.
    pub value: String,
    /// Which source supplied it.
    pub from: String,
}

/// The value a session uses: the first source that has one.
#[must_use]
pub fn resolve_session_value(key: OverriddenKey, input: &SessionInput) -> Option<SessionValue> {
    for source in session_overrides(key) {
        let Some(value) = source.read(input).filter(|value| !value.is_empty()) else {
            continue;
        };
        if source.admit.is_none_or(|admit| admit(&value, input)) {
            return Some(SessionValue {
                value,
                from: source.name.to_owned(),
            });
        }
    }
    None
}

crate::registry::exports! {
    "settings/general/session-overrides" {
        "resolveSessionValue" => ok |key: OverriddenKey, input: SessionInput| resolve_session_value(key, &input);
    }
}
