//! Port of `src/settings/marketplaces/gate.ts`, switched.
//!
//! The marketplace gate: `strictKnownMarketplaces` and `blockedMarketplaces` as verdicts over a
//! policy view (settings-reference.md, both keys; plugins/org.md, "Restrict what users can
//! install"). Effects stay the host's: the kit says whether a source may be added, installed,
//! updated, refreshed, auto-updated or loaded, and why.

use serde::Serialize;

use crate::js::JsValue;
use crate::js::array_join::string_of;
use crate::registry::{Handle, Port};
use crate::settings::customization::surfaces::read_surface_locks;
use crate::settings::marketplaces::match_source::{
    ListKind, match_claude_ai_marketplace, match_marketplace_source,
};
use crate::settings::marketplaces::source_kinds::{SourceKind, by_kind};
use crate::settings::view::PolicyView;

/// May a marketplace come from this source: `{ allowed: true }`, or the list that refuses it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SourceVerdict {
    /// The source is allowed.
    pub allowed: bool,
    /// `blockedMarketplaces` or `strictKnownMarketplaces`, when refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<&'static str>,
    /// Why, when refused.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Does a registered marketplace still load its plugins.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RegisteredVerdict {
    /// It loads.
    pub loads: bool,
    /// Why not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Whether skills-directory plugins (`@skills-dir`) load, and the list that stops them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SkillsDirVerdict {
    /// They load.
    pub admitted: bool,
    /// `blockedMarketplaces`, `strictKnownMarketplaces` or `strictPluginOnlyCustomization`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub by: Option<&'static str>,
}

/// What names a place a marketplace comes from: every kind but `skills-dir` (plugins/org.md).
const fn names_a_marketplace(kind: SourceKind) -> bool {
    !matches!(kind, SourceKind::SkillsDir)
}

/// The kinds written as the `owner/repo` shorthand, which points at github.com (plugins/org.md).
const fn is_shorthand(kind: SourceKind) -> bool {
    matches!(kind, SourceKind::Github)
}

/// The kinds that admit a git host by hostname.
const fn admits_by_hostname(kind: SourceKind) -> bool {
    matches!(kind, SourceKind::HostPattern)
}

fn kind_is(source: &JsValue, test: fn(SourceKind) -> bool) -> bool {
    SourceKind::of(source).is_some_and(test)
}

/// The one line that names a source in a message (judgment: the docs quote no per-kind wording).
fn labels(kind: SourceKind, source: &JsValue) -> String {
    let member = |name: &str| string_of(source.get(name).unwrap_or(&JsValue::Undefined));
    match kind {
        SourceKind::Github => format!("github:{}", member("repo")),
        SourceKind::Git => format!("git:{}", member("url")),
        SourceKind::Url => format!("url:{}", member("url")),
        SourceKind::File => format!("file:{}", member("path")),
        SourceKind::Directory => format!("directory:{}", member("path")),
        SourceKind::Npm => format!("npm:{}", member("package")),
        SourceKind::Settings => format!("settings:{}", member("name")),
        SourceKind::HostPattern => format!("hostPattern:{}", member("hostPattern")),
        SourceKind::PathPattern => format!("pathPattern:{}", member("pathPattern")),
        SourceKind::SkillsDir => "skills-dir".to_owned(),
    }
}

/// "The allowlist and blocklist match a marketplace hosted on claude.ai by its host": only a
/// `hostPattern` entry can name it, tested against `claude.ai` (`matchClaudeAiMarketplace`); on
/// the allowlist it "admits your organization's claude.ai marketplaces and the claude.ai default
/// marketplaces, but not a marketplace made of a member's own claude.ai uploads or one whose scope
/// claude.ai didn't state" (plugins/org.md, "Marketplaces hosted on claude.ai").
fn admitted_scope(scope: &str) -> bool {
    matches!(scope, "organization" | "default")
}

fn label(source: &JsValue) -> String {
    by_kind(labels, source).unwrap_or_default()
}

const BLOCKED_BY_POLICY: &str = "is blocked by enterprise policy";
const NOT_ALLOWED: &str = "is not in the allowed marketplace list";

fn matches_any(list: ListKind, entries: &[JsValue], source: &JsValue) -> bool {
    entries
        .iter()
        .any(|entry| match_marketplace_source(list, entry, source))
}

fn names_claude_ai(list: ListKind, entries: &[JsValue]) -> bool {
    entries
        .iter()
        .any(|entry| match_claude_ai_marketplace(list, entry))
}

fn names_skills_dir(entries: &[JsValue]) -> bool {
    entries
        .iter()
        .any(|entry| !kind_is(entry, names_a_marketplace))
}

const GUIDANCE_BLOCKED: &str =
    "This marketplace source is explicitly blocked by your administrator";
const GUIDANCE_CONTACT: &str =
    "Contact your administrator to configure allowed marketplace sources";
const NO_EXTERNAL: &str = "No external marketplaces are allowed.";
const GITHUB_TIP: &str = "Tip: the owner/repo shorthand assumes github.com. If the repository lives on your internal host, add it again with its full URL, such as git@your-git-host.com:owner/repo.git";

fn allowed_sources(entries: &[JsValue]) -> String {
    let labels: Vec<String> = entries.iter().map(label).collect();
    format!("Allowed sources: {}", labels.join(", "))
}

/// Judgment: the tail the docs name; an empty list says so, a non-empty one lists its sources.
fn allowlist_tail(entries: &[JsValue]) -> String {
    if entries.is_empty() {
        NO_EXTERNAL.to_owned()
    } else {
        allowed_sources(entries)
    }
}

/// The allowlist admits a git host by hostname, yet the shorthand points at github.com.
fn needs_github_tip(entries: &[JsValue], source: &JsValue) -> bool {
    kind_is(source, is_shorthand)
        && entries
            .iter()
            .any(|entry| kind_is(entry, admits_by_hostname))
}

fn blocked_message(source: &JsValue) -> String {
    format!("Marketplace source '{}' {BLOCKED_BY_POLICY}", label(source))
}

fn not_allowed_message(name: &str, entries: &[JsValue]) -> String {
    let tail = if entries.is_empty() {
        GUIDANCE_CONTACT.to_owned()
    } else {
        allowed_sources(entries)
    };
    format!("Marketplace \"{name}\" {NOT_ALLOWED}\n{tail}")
}

fn registered_blocked_message(name: &str) -> String {
    format!("Marketplace \"{name}\" {BLOCKED_BY_POLICY}\n{GUIDANCE_BLOCKED}")
}

const fn loads() -> RegisteredVerdict {
    RegisteredVerdict {
        loads: true,
        message: None,
    }
}

const fn refused(message: String) -> RegisteredVerdict {
    RegisteredVerdict {
        loads: false,
        message: Some(message),
    }
}

/// The gate over one view: its lists, read once, since the view never changes.
#[derive(Clone, Debug)]
pub struct MarketplaceGate {
    allowed: Option<Vec<JsValue>>,
    blocked: Vec<JsValue>,
    active: bool,
    plugin_only_locked: bool,
}

impl MarketplaceGate {
    /// On add, install, update, refresh and auto-update: may a marketplace come from this source?
    #[must_use]
    pub fn check(&self, source: &JsValue) -> SourceVerdict {
        if matches_any(ListKind::Blocked, &self.blocked, source) {
            return SourceVerdict {
                allowed: false,
                by: Some("blockedMarketplaces"),
                message: Some(blocked_message(source)),
            };
        }
        let Some(allowlist) = self
            .allowed
            .as_ref()
            .filter(|list| !matches_any(ListKind::Strict, list, source))
        else {
            return SourceVerdict {
                allowed: true,
                by: None,
                message: None,
            };
        };
        let tip = if needs_github_tip(allowlist, source) {
            format!("\n{GITHUB_TIP}")
        } else {
            String::new()
        };
        SourceVerdict {
            allowed: false,
            by: Some("strictKnownMarketplaces"),
            message: Some(format!(
                "{}. {}{tip}",
                blocked_message(source),
                allowlist_tail(allowlist)
            )),
        }
    }

    /// At session start: does a marketplace registered from this source still load its plugins?
    #[must_use]
    pub fn check_registered(&self, name: &str, source: &JsValue) -> RegisteredVerdict {
        if matches_any(ListKind::Blocked, &self.blocked, source) {
            return refused(registered_blocked_message(name));
        }
        match &self.allowed {
            Some(allowlist) if !matches_any(ListKind::Strict, allowlist, source) => {
                refused(not_allowed_message(name, allowlist))
            }
            _ => loads(),
        }
    }

    /// A marketplace hosted on claude.ai, matched by its host `claude.ai` (plugins/org.md).
    #[must_use]
    pub fn check_claude_ai(&self, name: &str, scope: Option<&str>) -> RegisteredVerdict {
        if names_claude_ai(ListKind::Blocked, &self.blocked) {
            return refused(registered_blocked_message(name));
        }
        let admitted = self.allowed.as_ref().is_none_or(|allowlist| {
            scope.is_some_and(admitted_scope) && names_claude_ai(ListKind::Strict, allowlist)
        });
        if admitted {
            loads()
        } else {
            refused(not_allowed_message(
                name,
                self.allowed.as_deref().unwrap_or_default(),
            ))
        }
    }

    /// A marketplace Claude Code cannot find: it does not load while a restriction is active.
    #[must_use]
    pub fn check_missing(&self, name: &str) -> RegisteredVerdict {
        if !self.restriction_active() {
            return loads();
        }
        // Judgment: the docs say /plugin "shows the policy error"; the allowlist's wording wins
        // when both lists are set, since it is the one that makes every unlisted marketplace
        // unusable.
        refused(match &self.allowed {
            None => registered_blocked_message(name),
            Some(allowlist) => not_allowed_message(name, allowlist),
        })
    }

    /// Any allowlist is set, or the blocklist names a source other than `skills-dir`.
    #[must_use]
    pub const fn restriction_active(&self) -> bool {
        self.active
    }

    /// `skills-dir` entries: whether skills-directory plugins may load.
    #[must_use]
    pub fn skills_directory_plugins(&self) -> SkillsDirVerdict {
        let refused_by = |by| SkillsDirVerdict {
            admitted: false,
            by: Some(by),
        };
        if names_skills_dir(&self.blocked) {
            return refused_by("blockedMarketplaces");
        }
        // Applies "whether or not the entry is present" (plugins/org.md, "Keep skills-directory
        // plugins loading").
        if self.plugin_only_locked {
            return refused_by("strictPluginOnlyCustomization");
        }
        match &self.allowed {
            Some(allowlist) if !names_skills_dir(allowlist) => {
                refused_by("strictKnownMarketplaces")
            }
            _ => SkillsDirVerdict {
                admitted: true,
                by: None,
            },
        }
    }
}

fn list_of(value: Option<JsValue>) -> Option<Vec<JsValue>> {
    let JsValue::Array(entries) = value? else {
        return Some(Vec::new());
    };
    Some(entries)
}

/// The gate over a view.
pub async fn create_marketplace_gate(view: &dyn PolicyView) -> MarketplaceGate {
    let allowed = list_of(view.get("strictKnownMarketplaces").await);
    let blocked = list_of(view.get("blockedMarketplaces").await).unwrap_or_default();
    let active = allowed.is_some()
        || blocked
            .iter()
            .any(|entry| kind_is(entry, names_a_marketplace));
    let locks = read_surface_locks(
        &view
            .get("strictPluginOnlyCustomization")
            .await
            .unwrap_or_default(),
    );
    MarketplaceGate {
        allowed,
        blocked,
        active,
        plugin_only_locked: !locks.locked.members.is_empty(),
    }
}

crate::registry::methods! {
    MarketplaceGate {
        "check" => ok |this, source: JsValue| this.check(&source);
        "checkRegistered" => ok |this, name: String, source: JsValue| this.check_registered(&name, &source);
        "checkClaudeAi" => ok |this, name: String, scope: Option<String>| this.check_claude_ai(&name, scope.as_deref());
        "checkMissing" => ok |this, name: String| this.check_missing(&name);
        "restrictionActive" => ok |this| this.restriction_active();
        "skillsDirectoryPlugins" => ok |this| this.skills_directory_plugins();
    }
}

crate::registry::exports! {
    "settings/marketplaces/gate" {
        "createMarketplaceGate" => async ok |view: Port| Handle::new(create_marketplace_gate(&view).await);
    }
}
