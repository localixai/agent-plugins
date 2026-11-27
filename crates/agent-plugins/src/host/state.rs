//! Port of `src/host/state.ts`, switched.
//!
//! What every session of a host shares: the state of one generation of the host, made once per
//! host generation and per session that brings its own settings, plugins or saved values, and held
//! as one handle (`Own<HostState>`, docs/RUST.md, "The host session"). The core tells plugins apart
//! by their ids and their place in the list the edge gave, never by a host object; the edge keeps
//! what is the host's (its options with their ports, the `HostPlugin` objects, the state cells).

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::catalog::{PluginCatalog, read_catalog};
use super::facts::HostFacts;
use super::plugin_dependencies::with_dependencies;
use super::plugin_set::{
    HostPlugin, PluginRun, distinct_plugins, layered_plugins, with_name_conflicts,
};
use crate::format::issues::PluginIssue;
use crate::format::mcpb_manifest::bundle_setting_key;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::loading::resolve::user_config::ResolvedUserConfig;
use crate::registry::{Callback, Handle, Port};
use crate::settings::customization::policy::{CustomizationPolicy, create_customization_policy};
use crate::settings::hooks::session_gates::{HookSessionGates, hook_session_gates};
use crate::settings::managed::parent::ParentSettingsContext;
use crate::settings::plugins::enabled_plugins::enabled_plugin;
use crate::settings::plugins::plugin_config::plugin_config;
use crate::settings::view::PolicyView;

/// A plugin the host was given, with whether it runs (`HostedPlugin`).
#[derive(Clone, Debug, Serialize)]
pub struct HostedPlugin {
    /// Its id.
    pub id: String,
    /// Its manifest name.
    pub name: String,
    /// Whether it runs.
    pub enabled: bool,
    /// `true` for a core package.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub core: Option<bool>,
}

/// A plugin the host runs: what its components read to, and the options saved for it.
#[derive(Clone, Debug)]
pub struct ActivePlugin {
    /// The plugin as it was given: its id, the core's copy and its place in the edge's list.
    pub entry: HostPlugin,
    /// What its Markdown components read to.
    pub catalog: Arc<PluginCatalog>,
    /// Its saved options.
    pub user_config: ResolvedUserConfig,
}

impl ActivePlugin {
    /// The plugin's id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.entry.id
    }

    /// The core package the plugin is the view of, `None` for a Claude Code plugin (`coreOf`).
    #[must_use]
    pub fn core(&self) -> Option<&JsValue> {
        self.entry.plugin.core.as_ref()
    }
}

/// What every session of a host shares: the state of one generation of the host.
#[derive(Debug)]
pub struct HostState {
    /// The data members of the host's (or the session's) options.
    pub options: HostFacts,
    /// The settings: the host's view, a port the state keeps.
    pub view: Port,
    /// How parent settings are treated: the host's, else no administrator and no merge.
    pub parent: ParentSettingsContext,
    /// `strictPluginOnlyCustomization` over `view` and `parent`, read once for this generation.
    pub customization: CustomizationPolicy,
    /// The hook gates over `view` and `parent`, read once for this generation.
    pub gates: HookSessionGates,
    /// Every plugin the host was given, with whether it runs.
    pub all: Vec<HostedPlugin>,
    /// The plugins that run, in the order given.
    pub active: Vec<ActivePlugin>,
    /// What the readers and the plugin set found.
    pub issues: Vec<JsValue>,
}

crate::registry::methods! {
    HostState {}
}

/// What `createHostState` is given: the plugins (the host's, then a session's own over them), the
/// settings view, the parent settings, the saved values and the options' data.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateGiven {
    /// The host's plugins and core packages, core packages last.
    pub plugins: Vec<HostPlugin>,
    /// A session's own plugins and core packages, layered over the host's by id.
    #[serde(default)]
    pub own: Option<Vec<HostPlugin>>,
    /// The settings view.
    pub view: Port,
    /// How parent settings are treated: the host's, else the edge's default (no administrator, no
    /// merge).
    pub parent: ParentSettingsContext,
    /// The host's saved `userConfig` values, by plugin id.
    #[serde(default)]
    pub user_config: Option<IndexMap<String, JsObject>>,
    /// The options' data members.
    #[serde(default)]
    pub facts: HostFacts,
}

/// Whether the host runs the plugin: the value the host gave, else what `enabledPlugins` says;
/// managed settings that turn it off win over both (settings-reference.md, `enabledPlugins`).
async fn runs(view: &dyn PolicyView, entry: &HostPlugin) -> bool {
    let default_enabled = entry.manifest("defaultEnabled").and_then(JsValue::as_bool);
    let decision = enabled_plugin(view, &entry.id, default_enabled.unwrap_or(true)).await;
    !decision.blocked_by_managed && entry.enabled.unwrap_or(decision.on)
}

/// `pluginConfigs.<id>.mcpServers` as values under `<server>.<key>`, the form a bundled server reads.
fn server_values(servers: &JsObject) -> JsObject {
    let mut values = JsObject::new();
    for (server, each) in servers {
        if let JsValue::Object(keys) = each {
            for (key, value) in keys {
                values.insert(bundle_setting_key(server, key), value.clone());
            }
        }
    }
    values
}

/// The saved options of a plugin: those of `pluginConfigs` in the settings (a bundled server's under
/// `<server>.<key>`), then the host's own values over them (the host keeps the secure store;
/// docs/API.md, "Variable substitution").
async fn user_config_of(
    view: &dyn PolicyView,
    host: Option<&IndexMap<String, JsObject>>,
    entry: &HostPlugin,
) -> (ResolvedUserConfig, Vec<PluginIssue>) {
    let saved = plugin_config(view, &entry.id).await;
    let mut values = server_values(&saved.mcp_servers);
    values.extend(saved.options);
    if let Some(own) = host.and_then(|all| all.get(&entry.id)) {
        values.extend(own.iter().map(|(key, value)| (key.clone(), value.clone())));
    }
    let declared = match entry.plugin.plugin.get("userConfig") {
        Some(JsValue::Object(declared)) => declared.clone(),
        _ => JsObject::new(),
    };
    (ResolvedUserConfig { declared, values }, saved.issues)
}

/// The catalog of a plugin that runs: known, a core package's (its view holds its Markdown), or read
/// from the Markdown the edge gives for the plugin at its place in the list (`markdownOf`).
/// Whether it was read now is the second member.
async fn catalog_of(
    entry: &HostPlugin,
    markdown: Callback,
) -> Result<(Arc<PluginCatalog>, bool), String> {
    if let Some(known) = entry.plugin.known_catalog() {
        return Ok((known, false));
    }
    let parts = if entry.plugin.core.is_some() {
        JsValue::Undefined
    } else {
        markdown
            .call::<JsValue>(vec![JsValue::from(entry.given as f64)])
            .await
    };
    Ok((read_catalog(&entry.plugin, &parts)?, true))
}

fn issue_values<T: Serialize>(issues: &[T]) -> impl Iterator<Item = JsValue> + '_ {
    issues.iter().filter_map(|issue| to_js_value(issue).ok())
}

/// The state of one generation of the host (or of a session over it).
///
/// # Errors
///
/// When a plugin's catalog cannot be read (`readCatalog`).
pub async fn create_host_state(given: StateGiven, markdown: Callback) -> Result<HostState, String> {
    Ok(created(given, markdown).await?.0)
}

/// The state, and the places of the plugins whose catalog was read in this call, which the edge
/// does not hold yet.
async fn created(given: StateGiven, markdown: Callback) -> Result<(HostState, Vec<usize>), String> {
    let StateGiven {
        mut plugins,
        own,
        view,
        parent,
        user_config,
        facts,
    } = given;
    let base = plugins.len();
    plugins
        .iter_mut()
        .enumerate()
        .for_each(|(at, entry)| entry.given = at);
    let own = own.map(|mut own| {
        own.iter_mut()
            .enumerate()
            .for_each(|(at, entry)| entry.given = base + at);
        own
    });
    let distinct = distinct_plugins(layered_plugins(plugins, own));
    let mut decided = Vec::with_capacity(distinct.kept.len());
    for entry in distinct.kept {
        decided.push(PluginRun {
            enabled: runs(&view, &entry).await,
            entry,
        });
    }
    let named = with_name_conflicts(&view, decided).await;
    let (states, needed) = with_dependencies(named.runs);
    let mut active = Vec::new();
    let mut active_issues = Vec::new();
    let mut read = Vec::new();
    for PluginRun { entry, enabled } in &states {
        if !enabled {
            continue;
        }
        let (config, config_issues) = user_config_of(&view, user_config.as_ref(), entry).await;
        let (catalog, now) = catalog_of(entry, markdown).await?;
        if now {
            read.push(entry.given);
        }
        active_issues.extend(issue_values(&catalog.issues));
        active_issues.extend(issue_values(&config_issues));
        active.push(ActivePlugin {
            entry: entry.clone(),
            catalog,
            user_config: config,
        });
    }
    let shared: Arc<dyn PolicyView> = Arc::new(view);
    let found = issue_values(&distinct.issues).chain(issue_values(&named.issues));
    let issues = found
        .chain(issue_values(&needed))
        .chain(active_issues)
        .collect();
    let state = HostState {
        options: facts,
        view,
        customization: create_customization_policy(Arc::clone(&shared), &parent).await,
        gates: hook_session_gates(shared, &parent).await,
        parent,
        all: states
            .iter()
            .map(|run| HostedPlugin {
                id: run.entry.id.clone(),
                name: run.entry.name().to_owned(),
                enabled: run.enabled,
                core: run.entry.plugin.core.is_some().then_some(true),
            })
            .collect(),
        active,
        issues,
    };
    Ok((state, read))
}

/// One plugin that runs, as the edge maps it back: its place in the edge's list, its options, and
/// its catalog when it was read in this call (the edge keeps it with the plugin object).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActiveAnswer<'a> {
    given: usize,
    user_config: &'a ResolvedUserConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    catalog: Option<&'a PluginCatalog>,
}

/// The state as the edge takes it: the handle, and what the edge's `HostState` carries.
#[derive(Debug, Serialize)]
struct StateAnswer<'a> {
    all: &'a [HostedPlugin],
    active: Vec<ActiveAnswer<'a>>,
    issues: &'a [JsValue],
    customization: &'a CustomizationPolicy,
    gates: &'a HookSessionGates,
}

/// The state's answer to the edge, then its handle under `handle`.
fn answer_of((state, read): (HostState, Vec<usize>)) -> Result<JsValue, String> {
    let answer = StateAnswer {
        all: &state.all,
        active: state
            .active
            .iter()
            .map(|plugin| ActiveAnswer {
                given: plugin.entry.given,
                user_config: &plugin.user_config,
                catalog: read
                    .contains(&plugin.entry.given)
                    .then_some(plugin.catalog.as_ref()),
            })
            .collect(),
        issues: &state.issues,
        customization: &state.customization,
        gates: &state.gates,
    };
    let mut made = to_js_value(&answer).map_err(|problem| problem.to_string())?;
    let handle = to_js_value(&Handle::own(state)).map_err(|problem| problem.to_string())?;
    if let JsValue::Object(members) = &mut made {
        members.insert("handle".to_owned(), handle);
    }
    Ok(made)
}

crate::registry::exports! {
    "host/state" {
        "createHostState" => async fallible |given: StateGiven, markdown: Callback|
            created(given, markdown).await.and_then(answer_of);
    }
}
