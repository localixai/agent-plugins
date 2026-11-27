//! Port of `src/settings/marketplaces/registry.ts`, switched.
//!
//! `extraKnownMarketplaces` as the plan a host executes: which marketplaces register, from what,
//! and why a declaration does not (settings-reference.md, `extraKnownMarketplaces`, "Marketplace
//! source types", "Marketplace key aliases"; plugins/org.md, "Require a marketplace and its
//! plugins", "Require plugins per repository", "Allow the official marketplace and your own";
//! plugins/loading.md, "Which marketplaces and plugins auto-update"). Cloning, refreshing and
//! running a `headersHelper` are the host's. The aliases are already rewritten by
//! `readSettingsLayer`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::marketplace::marketplace_name_lists::is_official_but_off;
use crate::js::{JsValue, property_order};
use crate::registry::{Callback, Handle, Port};
use crate::settings::layers::{PolicyScope, rank_of};
use crate::settings::marketplaces::headers_helper::planned_source;
use crate::settings::marketplaces::source_validate::registration_problems;
use crate::settings::plugins::enabled_plugins::{DecidedBy, enabled_plugin};
use crate::settings::plugins::plugin_id::parse_plugin_id;
use crate::settings::view::PolicyView;

/// Anthropic's official marketplace, which "needs no `extraKnownMarketplaces` entry when
/// `enabledPlugins` sets one of its plugins to `true`" (plugins/org.md). Its source is the one the
/// docs' allowlist example names ("Allow only the official marketplace").
pub const OFFICIAL_MARKETPLACE: &str = "claude-plugins-official";

/// `OFFICIAL_SOURCE`.
#[must_use]
pub fn official_source() -> JsValue {
    JsValue::Object(
        [
            ("source".to_owned(), JsValue::from("github")),
            (
                "repo".to_owned(),
                JsValue::from("anthropics/claude-plugins-official"),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

/// - `register`: the host adds the marketplace from this entry.
/// - `ignoredUntrusted`: a project, local or `--add-dir` entry in a folder that is not trusted.
/// - `refusedByAllowlist`: the allowlist or blocklist does not let the source in; `issues` says
///   why.
/// - `invalid`: the entry cannot name a marketplace (`issues` says why).
/// - `shadowed`: a higher-precedence file declares the same name; this entry is replaced whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "the type's comment says what each status is")]
pub enum RegistrationStatus {
    Register,
    IgnoredUntrusted,
    RefusedByAllowlist,
    Invalid,
    Shadowed,
}

/// The entry as the host registers it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlannedEntry {
    /// The source, as planned.
    pub source: JsValue,
    /// Whether it auto-updates.
    pub auto_update: JsValue,
}

/// One settings file, as the layers order them: two files of one scope have two positions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SettingsFile {
    /// The file's scope.
    pub scope: PolicyScope,
    /// Its position among the layers.
    pub position: usize,
}

/// One declaration and what becomes of it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Registration {
    /// The marketplace's name.
    pub name: String,
    /// The entry.
    pub entry: PlannedEntry,
    /// The file's scope.
    pub from: PolicyScope,
    /// What becomes of it.
    pub status: RegistrationStatus,
    /// The official marketplace declared by an `enabledPlugins` entry, not by an
    /// `extraKnownMarketplaces` one.
    pub implied: bool,
    /// A registered managed entry that sets `autoUpdate`: the user's `/plugin` toggle is refused.
    pub auto_update_fixed: bool,
    /// The settings file whose entry sets `autoUpdate`, by its scope and its position among the
    /// layers; absent when this entry leaves it unset and `entry.autoUpdate` is the default (a
    /// lower entry's value never carries over: "inherits none of its fields"). Judgment
    /// (spec/judgments.md).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_update_from: Option<SettingsFile>,
    /// Why it does not register.
    pub issues: Vec<PluginIssue>,
}

/// The marketplace gate's `check` verdict, as the registry reads it.
#[derive(Clone, Debug, Deserialize)]
struct CheckVerdict {
    allowed: bool,
    #[serde(default)]
    message: Option<String>,
}

/// What the registry needs from the host, as plain data and one injected check: the host's
/// `RegistryContext`, read by its members.
struct Context(Port);

impl Context {
    /// Whether the user accepted the workspace trust dialog for the folder of that kind of file.
    async fn is_folder_trusted(&self, scope: PolicyScope) -> bool {
        let trusted: Callback = self.0.member("isFolderTrusted").await;
        trusted.call(vec![JsValue::from(scope.name())]).await
    }

    /// Marketplaces Anthropic runs whose `autoUpdate` defaults on.
    async fn official_marketplaces(&self) -> Vec<String> {
        self.0.member("officialMarketplaces").await
    }

    /// The marketplace gate's `check` (allowlist and blocklist), taken as a function.
    async fn check(&self, source: &JsValue) -> CheckVerdict {
        let check: Callback = self.0.member("check").await;
        check.call(vec![source.clone()]).await
    }
}

/// Whether the file's folder must be trusted first. Project and local files are the documented
/// gate; a `--add-dir` directory's files ask the host too (spec/judgments.md).
async fn trust(scope: PolicyScope, context: &Context) -> bool {
    match scope {
        PolicyScope::Managed | PolicyScope::Parent | PolicyScope::Flag | PolicyScope::User => true,
        PolicyScope::Project | PolicyScope::Local | PolicyScope::AddDir => {
            context.is_folder_trusted(scope).await
        }
    }
}

#[derive(Clone, Debug)]
struct Declaration {
    name: String,
    entry: JsValue,
    from: PolicyScope,
    /// The layer's position; absent for the official marketplace an `enabledPlugins` entry
    /// implies.
    position: Option<usize>,
}

async fn declarations(view: &dyn PolicyView) -> IndexMap<String, Vec<Declaration>> {
    let mut by_name: IndexMap<String, Vec<Declaration>> = IndexMap::new();
    for holder in view.each_from_highest("extraKnownMarketplaces").await {
        let JsValue::Object(members) = &holder.value else {
            continue;
        };
        for (name, entry) in property_order(members) {
            by_name.entry(name.clone()).or_default().push(Declaration {
                name: name.clone(),
                entry: entry.clone(),
                from: holder.scope,
                position: Some(holder.position),
            });
        }
    }
    by_name
}

fn member_of(entry: &JsValue, name: &str) -> JsValue {
    entry.get(name).cloned().unwrap_or_default()
}

fn auto_update_of(declaration: &Declaration, official: &[String]) -> JsValue {
    match member_of(&declaration.entry, "autoUpdate") {
        JsValue::Undefined | JsValue::Null => JsValue::Bool(
            declaration.name == OFFICIAL_MARKETPLACE
                || (official.contains(&declaration.name)
                    && !is_official_but_off(&declaration.name)),
        ),
        set @ (JsValue::Bool(_)
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_)) => set,
    }
}

fn registration(
    declaration: &Declaration,
    official: &[String],
    status: RegistrationStatus,
    issues: Vec<PluginIssue>,
) -> Registration {
    let fixed = matches!(
        member_of(&declaration.entry, "autoUpdate"),
        JsValue::Bool(_)
    );
    Registration {
        name: declaration.name.clone(),
        entry: PlannedEntry {
            source: planned_source(member_of(&declaration.entry, "source"), declaration.from),
            auto_update: auto_update_of(declaration, official),
        },
        from: declaration.from,
        status,
        implied: false,
        auto_update_fixed: status == RegistrationStatus::Register
            && declaration.from == PolicyScope::Managed
            && fixed,
        auto_update_from: declaration
            .position
            .filter(|_| fixed)
            .map(|position| SettingsFile {
                scope: declaration.from,
                position,
            }),
        issues,
    }
}

/// The entry that precedence picked: it must name a marketplace and pass the allowlist and
/// blocklist.
async fn decide(declaration: &Declaration, context: &Context, official: &[String]) -> Registration {
    let source = member_of(&declaration.entry, "source");
    let problems = registration_problems(&declaration.name, &source);
    if !problems.is_empty() {
        return registration(declaration, official, RegistrationStatus::Invalid, problems);
    }
    let verdict = context.check(&source).await;
    if verdict.allowed {
        return registration(
            declaration,
            official,
            RegistrationStatus::Register,
            Vec::new(),
        );
    }
    let path = format!("extraKnownMarketplaces.{}.source", declaration.name);
    let message = verdict.message.unwrap_or_default();
    registration(
        declaration,
        official,
        RegistrationStatus::RefusedByAllowlist,
        vec![error(&path, &message)],
    )
}

/// One name across the files. An entry in an untrusted folder is "ignored without a message", as
/// if it were not there, so the highest entry left decides and replaces every lower one whole
/// (settings-reference.md: "inherits none of its fields"). Whether that entry then passes the
/// policy does not bring a lower one back (spec/judgments.md).
async fn plan_name(
    entries: &[Declaration],
    context: &Context,
    official: &[String],
) -> Vec<Registration> {
    let mut trusted = Vec::with_capacity(entries.len());
    for entry in entries {
        trusted.push(trust(entry.from, context).await);
    }
    let winner = trusted.iter().position(|one| *one);
    let mut planned = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if Some(index) == winner {
            planned.push(decide(entry, context, official).await);
            continue;
        }
        let status = if trusted.get(index).copied().unwrap_or(false) {
            RegistrationStatus::Shadowed
        } else {
            RegistrationStatus::IgnoredUntrusted
        };
        planned.push(registration(entry, official, status, Vec::new()));
    }
    planned
}

/// The highest scope whose `enabledPlugins` turns on a plugin of the official marketplace.
async fn official_enabled_by(view: &dyn PolicyView) -> Option<PolicyScope> {
    let enabled = view.get("enabledPlugins").await;
    let ids: Vec<String> = match &enabled {
        Some(JsValue::Object(members)) => property_order(members)
            .into_iter()
            .map(|(id, _)| id.clone())
            .collect(),
        _ => Vec::new(),
    };
    let mut highest: Option<PolicyScope> = None;
    for id in ids {
        if parse_plugin_id(&id).is_none_or(|parsed| parsed.origin != OFFICIAL_MARKETPLACE) {
            continue;
        }
        let decision = enabled_plugin(view, &id, true).await;
        if let (true, DecidedBy::Scope(scope)) = (decision.on, decision.decided_by)
            && highest.is_none_or(|known| rank_of(scope) > rank_of(known))
        {
            highest = Some(scope);
        }
    }
    highest
}

/// Whether a declaration was read at all: an ignored one is as if absent, any other one names the
/// marketplace.
const fn is_read(status: RegistrationStatus) -> bool {
    !matches!(status, RegistrationStatus::IgnoredUntrusted)
}

fn has_effective_entry(planned: Option<&Vec<Registration>>) -> bool {
    planned.is_some_and(|planned| planned.iter().any(|each| is_read(each.status)))
}

/// The registry over a view and the host's context.
struct MarketplaceRegistry {
    view: Port,
    context: Context,
}

impl MarketplaceRegistry {
    /// Every declaration, per name from the highest-precedence file down; names in order of
    /// appearance.
    async fn registrations(&self) -> Vec<Registration> {
        let by_name = declarations(&self.view).await;
        let official = self.context.official_marketplaces().await;
        let mut planned: IndexMap<String, Vec<Registration>> = IndexMap::new();
        for (name, entries) in by_name {
            let plan = plan_name(&entries, &self.context, &official).await;
            planned.insert(name, plan);
        }
        let enabled_by = official_enabled_by(&self.view).await;
        if let Some(from) = enabled_by
            && !has_effective_entry(planned.get(OFFICIAL_MARKETPLACE))
        {
            let declaration = Declaration {
                name: OFFICIAL_MARKETPLACE.to_owned(),
                entry: JsValue::Object(
                    [("source".to_owned(), official_source())]
                        .into_iter()
                        .collect(),
                ),
                from,
                position: None,
            };
            let implied = decide(&declaration, &self.context, &official).await;
            planned
                .entry(OFFICIAL_MARKETPLACE.to_owned())
                .or_default()
                .push(Registration {
                    implied: true,
                    ..implied
                });
        }
        planned.into_values().flatten().collect()
    }
}

crate::registry::methods! {
    MarketplaceRegistry {
        "registrations" => sync async ok |this| this.registrations().await;
    }
}

crate::registry::exports! {
    "settings/marketplaces/registry" {
        "OFFICIAL_MARKETPLACE" => ok || OFFICIAL_MARKETPLACE;
        "createMarketplaceRegistry" => ok |view: Port, context: Port| Handle::new(MarketplaceRegistry { view, context: Context(context) });
    }
}
