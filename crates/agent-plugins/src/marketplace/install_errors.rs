//! Port of `src/marketplace/install-errors.ts`, switched.
//!
//! What an install says when it stops, and what the Errors tab of `/plugin` shows when a plugin or
//! its marketplace cannot be found, read or trusted (plugins/troubleshooting.md, "Install a plugin",
//! "Plugin installed but not working"; plugins/loading.md, "Check which stage a plugin reached",
//! "Find where a plugin is enabled"). Pure texts; a text the docs quote is quoted verbatim, and the
//! joins the docs do not show are rows in spec/judgments.md.

use std::collections::HashSet;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use super::install_scopes::{InstallScope, collaborator_install_command, install_command};
use crate::js::{JsValue, string_of};
use crate::registry::Function;

/// An entry of the `/plugin` **Errors** tab: "Each entry pairs a message with a guidance line."
/// (`ErrorsTabEntry` of `install-messages.ts`, a module not ported yet.)
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ErrorsTabEntry {
    /// The message.
    pub message: String,
    /// The guidance line beneath it.
    pub guidance: String,
}

// ---- not found ----

/// A plugin that is not in its marketplace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct PluginNotFound {
    /// What is wrong.
    pub message: String,
    /// Present when the catalog may be stale or could not be refreshed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

/// Whether the marketplace was refreshed before the lookup, and why it was not.
#[derive(Clone, Debug, Deserialize)]
pub struct Refresh {
    /// It was.
    pub done: bool,
    /// Why the attempt failed.
    #[serde(default)]
    pub failure: Option<String>,
}

/// `Plugin "<name>" not found in marketplace "<marketplace>"`, with a hint when Claude Code did not
/// refresh the marketplace before the lookup: `Your local copy may be out of date — try claude
/// plugin marketplace update <marketplace>`, or `The marketplace couldn't be refreshed (...)` when
/// it tried and failed; with no hint the name is the likeliest problem. The message and the hint
/// are separate values, as the docs print them as two statements.
#[must_use]
pub fn plugin_not_found(plugin: &str, marketplace: &str, refresh: &Refresh) -> PluginNotFound {
    let message = format!("Plugin \"{plugin}\" not found in marketplace \"{marketplace}\"");
    if refresh.done {
        return PluginNotFound {
            message,
            hint: None,
        };
    }
    let hint = match &refresh.failure {
        None => format!(
            "Your local copy may be out of date — try claude plugin marketplace update {marketplace}"
        ),
        Some(failure) => format!("The marketplace couldn't be refreshed ({failure})"),
    };
    PluginNotFound {
        message,
        hint: Some(hint),
    }
}

/// Where an install was typed: `/plugin install` in a session, or `claude plugin install` in a shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallSurface {
    /// A session.
    Session,
    /// A shell.
    Shell,
}

/// `Plugin "<name>" not found in any marketplace` in a session; `claude plugin install` says `any configured marketplace`.
#[must_use]
pub fn plugin_not_found_anywhere(name: &str, surface: InstallSurface) -> String {
    match surface {
        InstallSurface::Session => format!("Plugin \"{name}\" not found in any marketplace"),
        InstallSurface::Shell => {
            format!("Plugin \"{name}\" not found in any configured marketplace")
        }
    }
}

/// The fix for the lookup that found nothing: name the marketplace, so Claude Code refreshes it first.
#[must_use]
pub fn name_the_marketplace(name: &str) -> String {
    format!("/plugin install {name}@<marketplace>")
}

// ---- already installed ----

/// A plugin installed at user scope or by managed settings is "already installed globally".
#[must_use]
pub fn is_installed_globally(scopes: &[InstallScope]) -> bool {
    scopes.iter().any(|scope| match scope {
        InstallScope::User | InstallScope::Managed => true,
        InstallScope::Project | InstallScope::Local => false,
    })
}

/// `/plugin install` refuses a plugin already installed globally: `Plugin '<name>@<marketplace>' is
/// already installed globally. Use '/plugin' to manage existing plugins.`; typed without
/// `@<marketplace>`, the message omits `globally`.
#[must_use]
pub fn already_installed_globally(plugin: &str, with_marketplace: bool) -> String {
    let globally = if with_marketplace { " globally" } else { "" };
    format!(
        "Plugin '{plugin}' is already installed{globally}. Use '/plugin' to manage existing plugins."
    )
}

/// What `claude plugin install` prints for a plugin already installed at the target scope (exit 0).
#[must_use]
pub fn already_installed_at_scope(id: &str, scope: InstallScope) -> String {
    format!(
        "Plugin \"{id}\" is already installed (scope: {})",
        scope.as_str()
    )
}

// ---- one summary of failures ----

/// A plugin that did not install, and why.
#[derive(Clone, Debug, Deserialize)]
pub struct InstallFailure {
    /// The plugin.
    pub plugin: String,
    /// Why.
    pub reason: String,
}

const FULL_ERROR: &str =
    "Installing a plugin from its details (Enter) in /plugin shows its full error.";

/// The summary the `/plugin` menu closes with when none of the selected plugins installed: `Failed
/// to install: <plugin> (<reason>)`. A reason shows only its first line, and when one was
/// shortened the summary ends with `Installing a plugin from its details (Enter) in /plugin shows
/// its full error.` Judgment: several failures are joined by `, `.
#[must_use]
pub fn failed_to_install(failures: &[InstallFailure]) -> String {
    let first_lines: Vec<(&str, &str, bool)> = failures
        .iter()
        .map(|one| {
            (
                one.plugin.as_str(),
                one.reason.split('\n').next().unwrap_or_default(),
                one.reason.contains('\n'),
            )
        })
        .collect();
    let listed: Vec<String> = first_lines
        .iter()
        .map(|(plugin, reason, _)| format!("{plugin} ({reason})"))
        .collect();
    let summary = format!("Failed to install: {}", listed.join(", "));
    if first_lines.iter().any(|(_, _, shortened)| *shortened) {
        format!("{summary} {FULL_ERROR}")
    } else {
        summary
    }
}

// ---- reload pending ----

/// The line an install summary ends with when the plugin is not yet active.
pub const RELOAD_TO_APPLY: &str = "Run /reload-plugins to apply.";
/// The line an install summary ends with when the plugin is active.
pub const PLUGIN_ACTIVE: &str = "Plugin is now active.";
/// The notice above the prompt that can appear at the same time.
pub const PLUGINS_CHANGED: &str = "Plugins changed. Run /reload-plugins to activate.";

/// What decides whether an install activated the plugin.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallActivationFacts {
    /// Activating the plugin now would invalidate the prompt cache.
    pub invalidates_prompt_cache: bool,
    /// The attempt to activate the plugin during the install failed.
    pub activation_failed: bool,
}

/// Whether an install activated the plugin, and the line its summary ends with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct InstallActivation {
    /// It did.
    pub activated: bool,
    /// The line.
    pub ending: String,
}

/// Whether an install activated the plugin, and the line its summary ends with: `Plugin is now
/// active.` when it did, else `Run /reload-plugins to apply.` "Claude Code didn't activate the
/// plugin during the install, either because activating it would invalidate the prompt cache or
/// because the activation attempt failed" (troubleshooting.md).
#[must_use]
pub fn install_activation(facts: InstallActivationFacts) -> InstallActivation {
    let activated = !facts.invalidates_prompt_cache && !facts.activation_failed;
    InstallActivation {
        activated,
        ending: if activated {
            PLUGIN_ACTIVE
        } else {
            RELOAD_TO_APPLY
        }
        .to_owned(),
    }
}

// ---- Errors tab entries ----

/// `Plugin "<name>" not cached at <path>`: the install record points at a directory that is gone.
#[must_use]
pub fn not_cached(name: &str, path: &str) -> ErrorsTabEntry {
    ErrorsTabEntry {
        message: format!("Plugin \"{name}\" not cached at {path}"),
        guidance: "Run /plugin to refresh the plugin cache".to_owned(),
    }
}

/// The shell command that re-downloads a plugin whose record exists but whose directory is missing.
#[must_use]
pub fn reinstall_command(name: &str, marketplace: &str) -> String {
    install_command(&format!("{name}@{marketplace}"))
}

/// A project's settings enable the plugin, but it was never installed on this machine.
#[must_use]
pub fn enabled_not_installed(name: &str, marketplace: &str) -> ErrorsTabEntry {
    ErrorsTabEntry {
        message: format!(
            "Plugin \"{name}\" is enabled in project settings but isn't installed here"
        ),
        guidance: format!(
            "Run {} to install it for this project",
            collaborator_install_command(&format!("{name}@{marketplace}"))
        ),
    }
}

/// An enabled plugin its marketplace lists by relative path, with no directory at that path.
#[must_use]
pub fn plugin_directory_not_found(path: &str) -> String {
    format!("Plugin directory not found at path: {path}")
}

/// The marketplace's own directory is missing, such as a local marketplace that moved.
#[must_use]
pub fn marketplace_directory_not_found(path: &str) -> String {
    format!("Marketplace directory not found at path: {path}")
}

/// A plugin hook's failure when the directory the session loaded its hooks from is gone.
#[must_use]
pub fn plugin_directory_missing_at_run(path: &str, plugin: &str) -> String {
    format!(
        "Failed to run: Plugin directory does not exist: {path} ({plugin} — run /plugin to reinstall)"
    )
}

// ---- empty ----

/// Where the empty list shows: the **Discover** tab, or `claude plugin marketplace list`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub enum EmptyView {
    /// `discover`.
    #[serde(rename = "discover")]
    Discover,
    /// `marketplace-list`.
    #[serde(rename = "marketplace-list")]
    MarketplaceList,
}

const fn empty_message(view: EmptyView) -> &'static str {
    match view {
        EmptyView::Discover => "No plugins available",
        EmptyView::MarketplaceList => "No marketplaces configured",
    }
}

/// What the fix is when no marketplace is registered: add the official one.
const ADD_OFFICIAL: &str = "/plugin marketplace add anthropics/claude-plugins-official";

/// `No plugins available` on the Discover tab, `No marketplaces configured` from the list command:
/// both mean no marketplace is registered, so there is no catalog; `None` when one is.
#[must_use]
pub fn no_marketplaces(registered: &[String], view: EmptyView) -> Option<ErrorsTabEntry> {
    if registered.is_empty() {
        Some(ErrorsTabEntry {
            message: empty_message(view).to_owned(),
            guidance: ADD_OFFICIAL.to_owned(),
        })
    } else {
        None
    }
}

// ---- a different source ----

/// A member of a source, `undefined` when it has none.
fn member<'a>(source: &'a JsValue, key: &str) -> &'a JsValue {
    source.get(key).unwrap_or(&JsValue::Undefined)
}

/// Whether the source has the key (`key in source`).
fn has(source: &JsValue, key: &str) -> bool {
    source
        .as_object()
        .is_some_and(|members| members.contains_key(key))
}

fn ref_suffix(source: &JsValue) -> String {
    match member(source, "ref") {
        JsValue::Undefined => String::new(),
        other @ (JsValue::Null
        | JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_)) => format!("#{}", string_of(other)),
    }
}

/// One reading of a source per shape its fields give; the first that applies names it.
fn describers(source: &JsValue) -> [Option<JsValue>; 8] {
    let text = |value: String| Some(JsValue::from(value));
    [
        has(source, "repo").then(|| {
            JsValue::from(format!(
                "github:{}{}",
                string_of(member(source, "repo")),
                ref_suffix(source)
            ))
        }),
        has(source, "package")
            .then(|| JsValue::from(format!("npm:{}", string_of(member(source, "package"))))),
        if has(source, "url") && has(source, "path") {
            text(format!(
                "git:{}{}",
                string_of(member(source, "url")),
                ref_suffix(source)
            ))
        } else {
            None
        },
        has(source, "hostPattern").then(|| member(source, "hostPattern").clone()),
        has(source, "pathPattern").then(|| member(source, "pathPattern").clone()),
        has(source, "path").then(|| member(source, "path").clone()),
        has(source, "plugins")
            .then(|| JsValue::from(format!("settings:{}", string_of(member(source, "name"))))),
        has(source, "url").then(|| member(source, "url").clone()),
    ]
}

/// A source as the "already added from a different source" message writes it: `github:acme/plugins`
/// for GitHub (the docs' example). Judgment (spec/judgments.md): the other kinds are written the
/// same way, `git:<url>`, `npm:<package>`, `settings:<name>`, and a URL or a path as it is.
#[must_use]
pub fn describe_source(source: &JsValue) -> String {
    let found = describers(source)
        .into_iter()
        .flatten()
        .find(|text| !matches!(text, JsValue::Undefined));
    match found {
        None => "skills-dir".to_owned(),
        Some(text) => string_of(&text),
    }
}

/// `Marketplace "<name>" is already added from a different source (<source>). To use this source
/// instead, remove that marketplace first with /plugin marketplace remove <name>.`
#[must_use]
pub fn already_added_from_different_source(name: &str, existing: &JsValue) -> String {
    format!(
        "Marketplace \"{name}\" is already added from a different source ({}). To use this source instead, remove that marketplace first with /plugin marketplace remove {name}.",
        describe_source(existing)
    )
}

/// What `failureOnce` keeps: the events and commands that have failed.
#[derive(Debug, Default)]
pub struct FailureOnce {
    seen: Mutex<HashSet<(String, String)>>,
}

impl FailureOnce {
    /// Whether this event and command have not failed yet this session; remembers that they have.
    fn first(&self, event: String, command: String) -> bool {
        let mut seen = self
            .seen
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        seen.insert((event, command))
    }
}

crate::registry::methods! {
    FailureOnce {
        "call" => ok |this, event: String, command: String| this.first(event, command);
    }
}

/// "The failure is shown once per session for each hook event and command" (troubleshooting.md,
/// "Plugin directory does not exist"): the returned function says whether this event and command
/// have not failed yet this session, and remembers that they have.
#[must_use]
pub fn failure_once() -> Function {
    Function::new(FailureOnce::default())
}

crate::registry::exports! {
    "marketplace/install-errors" {
        "pluginNotFound" => ok |plugin: String, marketplace: String, refresh: Refresh| plugin_not_found(&plugin, &marketplace, &refresh);
        "pluginNotFoundAnywhere" => ok |name: String, surface: InstallSurface| plugin_not_found_anywhere(&name, surface);
        "nameTheMarketplace" => ok |name: String| name_the_marketplace(&name);
        "isInstalledGlobally" => ok |scopes: Vec<InstallScope>| is_installed_globally(&scopes);
        "alreadyInstalledGlobally" => ok |plugin: String, with_marketplace: bool| already_installed_globally(&plugin, with_marketplace);
        "alreadyInstalledAtScope" => ok |id: String, scope: InstallScope| already_installed_at_scope(&id, scope);
        "failedToInstall" => ok |failures: Vec<InstallFailure>| failed_to_install(&failures);
        "RELOAD_TO_APPLY" => ok || RELOAD_TO_APPLY;
        "PLUGIN_ACTIVE" => ok || PLUGIN_ACTIVE;
        "PLUGINS_CHANGED" => ok || PLUGINS_CHANGED;
        "installActivation" => ok |facts: InstallActivationFacts| install_activation(facts);
        "notCached" => ok |name: String, path: String| not_cached(&name, &path);
        "reinstallCommand" => ok |name: String, marketplace: String| reinstall_command(&name, &marketplace);
        "enabledNotInstalled" => ok |name: String, marketplace: String| enabled_not_installed(&name, &marketplace);
        "pluginDirectoryNotFound" => ok |path: String| plugin_directory_not_found(&path);
        "marketplaceDirectoryNotFound" => ok |path: String| marketplace_directory_not_found(&path);
        "pluginDirectoryMissingAtRun" => ok |path: String, plugin: String| plugin_directory_missing_at_run(&path, &plugin);
        "noMarketplaces" => ok |registered: Vec<String>, view: EmptyView| no_marketplaces(&registered, view);
        "describeSource" => ok |source: JsValue| describe_source(&source);
        "alreadyAddedFromDifferentSource" => ok |name: String, existing: JsValue| already_added_from_different_source(&name, &existing);
        "failureOnce" => ok failure_once;
    }
}
