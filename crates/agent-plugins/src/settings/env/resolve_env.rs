//! Port of `src/settings/env/resolve-env.ts`, switched.
//!
//! What the `env` blocks of the settings files come to for a session (settings-reference.md,
//! `env`: "How `env` values interact with your shell", "When Claude Code applies `env` values",
//! "Variables Claude Code ignores in `env`"; env-vars.md, "Set variables in settings files" and
//! "Precedence"). Pure over data: the host passes its environment and what it knows of trust, and
//! writes the result into the process it starts.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::{JsValue, in_key_order, property_order};
use crate::ports::BoxFuture;
use crate::protocol::is_record::is_record;
use crate::registry::{Callback, Port};
use crate::settings::env::ignored_variables::{
    IgnoreReason, env_text, ignored_because, turns_telemetry_off,
};
use crate::settings::key_spec::{REPOSITORY_FILES, admits};
use crate::settings::layers::{PolicyLayer, PolicyScope, ordered_layers};

/// What the session knows of trust.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct EnvTrust {
    /// An interactive session; `claude -p` and an SDK session never show the trust dialog.
    pub interactive: bool,
    /// The user accepted the trust dialog for the folder or a parent folder whose trust extends to it.
    pub trusted: bool,
}

/// Variables by name; a name set to `undefined` is unset.
pub type Variables = IndexMap<String, Option<String>>;

/// `EnvInput` without its `isSafeAtStartup`, which is a [`NameTest`] of its own.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvInput {
    /// The settings layers, read by `readSettingsLayer`.
    pub layers: Vec<PolicyLayer>,
    /// What the session knows of trust.
    pub trust: EnvTrust,
    /// The environment `claude` was started from.
    pub shell: Variables,
    /// The launch environment the Claude Desktop app or a self-hosted environment runner builds:
    /// it "takes precedence instead", so no settings file overwrites a variable it sets.
    #[serde(default)]
    pub launch_environment: Option<Variables>,
    /// The host's variable names are case-insensitive (Windows): the lists and the environments
    /// compare without case.
    #[serde(default)]
    pub windows: Option<bool>,
}

/// A question about a variable's name the host answers: `isSafeAtStartup`, the variables "Claude
/// Code classifies as safe, such as model selection, timeouts and limits, and feature toggles".
pub trait NameTest: Send + Sync {
    /// Whether it holds for `name`.
    fn holds<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool>;
}

impl<F: Fn(&str) -> bool + Send + Sync> NameTest for F {
    fn holds<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(std::future::ready(self(name)))
    }
}

/// When a file's values start to apply, by its scope.
pub trait ScopeTest: Send + Sync {
    /// Whether they apply now.
    fn holds(&self, scope: PolicyScope) -> BoxFuture<'_, bool>;
}

impl<F: Fn(PolicyScope) -> bool + Send + Sync> ScopeTest for F {
    fn holds(&self, scope: PolicyScope) -> BoxFuture<'_, bool> {
        Box::pin(std::future::ready(self(scope)))
    }
}

/// A method of the host's record, called with the record as `this`: `input.isSafeAtStartup(name)`
/// counts only an answer `=== true`.
pub struct HostMethod {
    /// The record.
    pub record: Port,
    /// The method's name.
    pub method: &'static str,
}

impl NameTest for HostMethod {
    fn holds<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self
            .record
            .method::<JsValue>(self.method, vec![name.into()]);
        Box::pin(async move { asked.await == JsValue::Bool(true) })
    }
}

/// Why a settings value did not reach the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NotApplied {
    /// A variable Claude Code ignores in `env`.
    Ignored(IgnoreReason),
    /// Another reason.
    Skipped(Skip),
}

/// The reasons beyond the ignored variables.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Skip {
    /// The launch environment sets it.
    LaunchEnvironment,
    /// The shell sets it, and a repository's off value does not replace it.
    ShellKeepsIt,
    /// A repository's file waits for the trust dialog.
    AwaitingTrust,
}

/// A value left out, with the reason; the debug log names each one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotAppliedVariable {
    /// The variable.
    pub name: String,
    /// The file it was set in.
    pub scope: PolicyScope,
    /// Why it did not apply.
    pub why: NotApplied,
}

/// What the files come to, and the environments of the session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvResolution {
    /// What the files come to, by variable: the highest file's value.
    pub settings: IndexMap<String, String>,
    /// The file each of `settings` came from.
    pub origin: IndexMap<String, PolicyScope>,
    /// The values left out, with the reason; the debug log names each one.
    pub not_applied: Vec<NotAppliedVariable>,
    /// The environment Claude Code's own process reads.
    pub process: IndexMap<String, String>,
    /// The environment every subprocess it starts inherits.
    pub subprocess: IndexMap<String, String>,
}

/// "`NO_COLOR` and `FORCE_COLOR` set here reach only subprocesses": the interface colors of Claude
/// Code itself are chosen from the shell it was launched from.
pub const SUBPROCESS_ONLY: [&str; 2] = ["NO_COLOR", "FORCE_COLOR"];

fn defined(variables: &Variables) -> Vec<(String, String)> {
    in_key_order(variables.clone())
        .into_iter()
        .filter_map(|(name, value)| value.map(|value| (name, value)))
        .collect()
}

/// Whether the files of `scope` take effect at startup in this session, or only after the folder is trusted.
#[must_use]
pub fn applies_at_startup(scope: PolicyScope, trust: EnvTrust) -> bool {
    !REPOSITORY_FILES.contains(&scope) || trust.trusted || !trust.interactive
}

/// The `env` entries of one layer the key's row lets its scope carry (settings-reference.md, `env`:
/// "Any file"; an `--add-dir` directory carries two keys only, permissions.md).
fn entries_of(layer: &PolicyLayer) -> Vec<(String, JsValue)> {
    match layer.settings.get("env") {
        Some(env) if is_record(env) && admits("env", layer.scope, env) => env
            .as_object()
            .map(property_order)
            .unwrap_or_default()
            .into_iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

/// Whether `variables` sets `name`: on a Windows host whatever the case of the name.
fn sets(variables: Option<&Variables>, name: &str, windows: bool) -> bool {
    let Some(variables) = variables else {
        return false;
    };
    if !windows {
        return variables.get(name).is_some_and(Option::is_some);
    }
    let wanted = name.to_uppercase();
    variables
        .iter()
        .any(|(set, value)| value.is_some() && set.to_uppercase() == wanted)
}

/// What a resolution compares against: `EnvInput` without `layers` and `trust`.
pub struct RulesInput<'a> {
    /// The environment `claude` was started from.
    pub shell: &'a Variables,
    /// The launch environment, when one was built.
    pub launch_environment: Option<&'a Variables>,
    /// The variables applied at startup from every settings file.
    pub is_safe_at_startup: Option<&'a dyn NameTest>,
    /// The host's variable names are case-insensitive.
    pub windows: Option<bool>,
}

/// How one resolution decides: when a file's values start to apply, and against which environments.
pub struct Rules<'a> {
    /// Whether a file's values apply now.
    pub starts_now: &'a dyn ScopeTest,
    /// The environments.
    pub input: RulesInput<'a>,
}

/// Why a candidate is left out of the session, when it is.
async fn reason_to_skip(
    name: &str,
    value: &str,
    scope: PolicyScope,
    rules: &Rules<'_>,
) -> Option<NotApplied> {
    let input = &rules.input;
    let windows = input.windows.unwrap_or(false);
    let text = JsValue::from(value);
    if let Some(dropped) = ignored_because(scope, name, &text, windows) {
        return Some(NotApplied::Ignored(dropped));
    }
    if sets(input.launch_environment, name, windows) {
        return Some(NotApplied::Skipped(Skip::LaunchEnvironment));
    }
    // "Such a value overrides the same variable in your user settings, but not one that the
    // environment you start Claude Code from ... sets".
    if REPOSITORY_FILES.contains(&scope)
        && turns_telemetry_off(name, &text, windows)
        && sets(Some(input.shell), name, windows)
    {
        return Some(NotApplied::Skipped(Skip::ShellKeepsIt));
    }
    if rules.starts_now.holds(scope).await {
        return None;
    }
    match input.is_safe_at_startup {
        Some(test) if test.holds(name).await => None,
        _ => Some(NotApplied::Skipped(Skip::AwaitingTrust)),
    }
}

/// What the `env` blocks of the files come to, without the environments.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesResolution {
    /// What the files come to, by variable.
    pub settings: IndexMap<String, String>,
    /// The file each of `settings` came from.
    pub origin: IndexMap<String, PolicyScope>,
    /// The values left out, with the reason.
    pub not_applied: Vec<NotAppliedVariable>,
}

/// What the `env` blocks of the layers come to, the highest file's value winning for a variable.
pub async fn resolve_env_blocks(layers: &[PolicyLayer], rules: &Rules<'_>) -> FilesResolution {
    // Maps, so a JSON-parsed `__proto__` name stays a name.
    let mut settings = IndexMap::new();
    let mut origin = IndexMap::new();
    let mut not_applied = Vec::new();
    // Lowest precedence first, so a higher file's value replaces a lower one's.
    for layer in ordered_layers(layers).iter().rev() {
        let scope = layer.scope;
        for (name, one) in entries_of(layer) {
            // A layer a host built without `readSettingsLayer` gets the same text the reader gives.
            let value = env_text(&one);
            match reason_to_skip(&name, &value, scope, rules).await {
                Some(why) => not_applied.push(NotAppliedVariable { name, scope, why }),
                None => {
                    settings.insert(name.clone(), value);
                    origin.insert(name, scope);
                }
            }
        }
    }
    FilesResolution {
        settings: in_key_order(settings),
        origin: in_key_order(origin),
        not_applied,
    }
}

/// `{ ...a, ...b }` of two records.
fn spread<V: Clone>(
    first: &IndexMap<String, V>,
    second: &IndexMap<String, V>,
) -> IndexMap<String, V> {
    let mut merged = first.clone();
    for (name, value) in second {
        merged.insert(name.clone(), value.clone());
    }
    in_key_order(merged)
}

/// The process environments a session has once `settings` are written over `base`.
fn compose_environments(
    base: &Variables,
    settings: &IndexMap<String, String>,
) -> (IndexMap<String, String>, IndexMap<String, String>) {
    let inherited: IndexMap<String, String> = defined(base).into_iter().collect();
    let own: IndexMap<String, String> = settings
        .iter()
        .filter(|(name, _)| !SUBPROCESS_ONLY.contains(&name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    (spread(&inherited, &own), spread(&inherited, settings))
}

/// The environment of a session: the shell's, with the `env` of the settings files written over
/// it, "the highest-precedence one" for a variable more than one file sets.
pub async fn resolve_env(
    input: &EnvInput,
    is_safe_at_startup: Option<&dyn NameTest>,
) -> EnvResolution {
    let trust = input.trust;
    let starts_now = move |scope| applies_at_startup(scope, trust);
    let rules = Rules {
        starts_now: &starts_now,
        input: RulesInput {
            shell: &input.shell,
            launch_environment: input.launch_environment.as_ref(),
            is_safe_at_startup,
            windows: input.windows,
        },
    };
    let files = resolve_env_blocks(&input.layers, &rules).await;
    let base = input.launch_environment.as_ref().map_or_else(
        || input.shell.clone(),
        |launch| spread(&input.shell, launch),
    );
    let (process, subprocess) = compose_environments(&base, &files.settings);
    EnvResolution {
        settings: files.settings,
        origin: files.origin,
        not_applied: files.not_applied,
        process,
        subprocess,
    }
}

/// Whether a variable counts as unset: "To cancel a shell export, set the variable to `""`. Claude
/// Code treats an empty value as unset for provider selection, and subprocesses inherit the empty
/// value."
#[must_use]
pub fn is_unset_for_provider(environment: &Variables, name: &str) -> bool {
    environment
        .get(name)
        .and_then(Option::as_deref)
        .is_none_or(str::is_empty)
}

/// `resolveEnv`'s input as the host passes it: a record that holds `isSafeAtStartup` crosses as
/// the host's own value, its data members read one by one.
#[derive(Deserialize)]
#[serde(untagged)]
enum EnvInputArg {
    Host(Port),
    Data(EnvInput),
}

async fn resolve_env_of(input: EnvInputArg) -> EnvResolution {
    match input {
        EnvInputArg::Data(input) => resolve_env(&input, None).await,
        EnvInputArg::Host(record) => {
            let read = EnvInput {
                layers: record.member("layers").await,
                trust: record.member("trust").await,
                shell: record.member("shell").await,
                launch_environment: record.member("launchEnvironment").await,
                windows: record.member("windows").await,
            };
            let given: Option<Callback> = record.member("isSafeAtStartup").await;
            let test = HostMethod {
                record,
                method: "isSafeAtStartup",
            };
            let safe = given.map(|_| &test as &dyn NameTest);
            resolve_env(&read, safe).await
        }
    }
}

crate::registry::exports! {
    "settings/env/resolve-env" {
        "SUBPROCESS_ONLY" => ok || SUBPROCESS_ONLY;
        "appliesAtStartup" => ok |scope: PolicyScope, trust: EnvTrust| applies_at_startup(scope, trust);
        "resolveEnv" => async ok |input: EnvInputArg| resolve_env_of(input).await;
        "isUnsetForProvider" => ok |environment: Variables, name: String|
            is_unset_for_provider(&environment, &name);
    }
}
