//! Port of `src/skills/scan/locations.ts`, switched.
//!
//! Where skills and command files are read from, one rule per scope (skills.md, "Choose where
//! skills load"; permissions.md, "Additional directories grant file access, not configuration"). A
//! registry typed over the scopes, so a new scope fails the typecheck until it says where it reads
//! and what it loads as. `rank` is the precedence of "Resolve skills that share a name": the
//! managed (enterprise) directory over the personal folder over the project, then the skills below
//! the working directory, then the added directories, then the skills synced from claude.ai.
//!
//! A rule's `pluginsAt` and `places` are functions in the TypeScript; here they are a function and
//! a table key, and `SCOPE_RULES` crosses as the rules' data.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::skill_name::SkillOrigin;
use crate::foundation::scope_folders::{
    HostContext, ScopeContext, ScopePlace, ScopePlaceKind, ScopeRequest, places_below,
    scope_context_of, scope_places,
};
use crate::js::{JsObject, JsValue};
use crate::registry::Callback;

/// A scope a skill or command is read from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanScope {
    /// The managed (enterprise) directory.
    Managed,
    /// The personal folder.
    User,
    /// The project and its parents.
    Project,
    /// Below the working directory.
    Nested,
    /// The added directories.
    Additional,
    /// Synced from claude.ai.
    Synced,
}

/// `SCAN_SCOPES`.
pub const SCAN_SCOPES: [ScanScope; 6] = [
    ScanScope::Managed,
    ScanScope::User,
    ScanScope::Project,
    ScanScope::Nested,
    ScanScope::Additional,
    ScanScope::Synced,
];

/// What a scan is asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRequest {
    /// The scope folders' request.
    #[serde(flatten)]
    pub scope: ScopeRequest,
    /// Directories below `cwd` whose `.claude/skills` the session loaded (see `scanNested`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nested_directories: Option<Vec<String>>,
}

impl AsRef<ScopeRequest> for ScanRequest {
    fn as_ref(&self) -> &ScopeRequest {
        &self.scope
    }
}

/// Whether `strictPluginOnlyCustomization` lets a source load (`ScanContext.allows`).
#[derive(Clone)]
pub enum Allows {
    /// The host's function.
    Host(Callback),
    /// The core's answer, by the source's name.
    Core(std::sync::Arc<dyn Fn(&str) -> bool + Send + Sync>),
}

impl Allows {
    /// Whether the source `origin` loads.
    pub async fn allows(&self, origin: &str) -> bool {
        match self {
            Self::Host(callback) => callback.call(vec![JsValue::from(origin)]).await,
            Self::Core(allows) => allows(origin),
        }
    }
}

/// What a scan reads with.
#[derive(Clone)]
pub struct ScanContext {
    /// The scope folders' context, with the scan's request.
    pub scope: ScopeContext<ScanRequest>,
    /// Whether `strictPluginOnlyCustomization` lets a source load; absent, every source does.
    pub allows: Option<Allows>,
}

/// The host's `ScanContext`, its members read when it crossed as a port (it holds `allows`).
pub async fn scan_context_of(context: HostContext<ScanRequest>) -> ScanContext {
    let allows = match &context {
        HostContext::Host(port) => port
            .member::<Option<Callback>>("allows")
            .await
            .map(Allows::Host),
        HostContext::Data(_) => None,
    };
    ScanContext {
        scope: scope_context_of(context).await,
        allows,
    }
}

/// A folder of skills or commands below a base directory's scope folder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Container {
    /// The folder below a base directory's scope folder (`.claude`, or an extra one): `skills`.
    pub folder: &'static str,
    /// The name `strictPluginOnlyCustomization` knows this source by.
    pub origin: &'static str,
}

/// Where a scope's folders are: an entry of `SCOPE_PLACES`, or the nested directories asked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Places {
    /// `SCOPE_PLACES[kind]`.
    Scope(ScopePlaceKind),
    /// `placesBelow` the request's `nestedDirectories`.
    Nested,
}

/// One scope's rule.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRule {
    /// The precedence of "Resolve skills that share a name".
    pub rank: u32,
    /// What a skill or command of this scope is read as.
    pub origin: JsValue,
    /// `origin` as the skill readers take it.
    #[serde(skip)]
    pub layout_origin: SkillOrigin,
    /// Where its skills are.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Container>,
    /// Where its commands are.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<Container>,
    /// Whether the session watches the skills folder for changes (skills.md, "Edit a skill during
    /// a session": the personal folder, the project's and each added directory's).
    pub watched: bool,
    /// Whether a skill folder at the `index`th base may be a plugin (`<name>@skills-dir`); only
    /// below `.claude`.
    #[serde(skip)]
    pub plugins_at: fn(usize) -> bool,
    /// Whether the host's extra scope folders are read too (`scopeFoldersOf`), or `.claude` alone.
    pub extra_folders: bool,
    /// The folders read, for the scope folders asked: each base directory's `.claude` (the
    /// configuration directory itself for the user scope), then the extra ones. Order is
    /// precedence within the scope.
    #[serde(skip)]
    pub places: Places,
}

const SKILLS: &str = "skills";
const COMMANDS: &str = "commands";

fn origin(kind: &str) -> JsValue {
    JsValue::from(JsObject::from([("kind".to_owned(), JsValue::from(kind))]))
}

const fn never(_index: usize) -> bool {
    false
}

const fn always(_index: usize) -> bool {
    true
}

// "unlike plain skills and commands, [plugins don't] search parent directories" (loading.md).
const fn first_only(index: usize) -> bool {
    index == 0
}

/// `SCOPE_RULES[scope]`.
#[must_use]
pub fn scope_rule(scope: ScanScope) -> ScopeRule {
    match scope {
        ScanScope::Managed => ScopeRule {
            rank: 0,
            origin: origin("managed"),
            layout_origin: SkillOrigin::Managed,
            skills: Some(Container {
                folder: SKILLS,
                origin: "managedDirectory",
            }),
            commands: None,
            watched: false,
            plugins_at: never,
            extra_folders: true,
            places: Places::Scope(ScopePlaceKind::Managed),
        },
        ScanScope::User => ScopeRule {
            rank: 1,
            origin: origin("user"),
            layout_origin: SkillOrigin::User,
            skills: Some(Container {
                folder: SKILLS,
                origin: "userSkills",
            }),
            commands: Some(Container {
                folder: COMMANDS,
                origin: "userCommands",
            }),
            watched: true,
            plugins_at: always,
            extra_folders: true,
            places: Places::Scope(ScopePlaceKind::User),
        },
        ScanScope::Project => ScopeRule {
            rank: 2,
            origin: origin("project"),
            layout_origin: SkillOrigin::Project,
            skills: Some(Container {
                folder: SKILLS,
                origin: "projectSkills",
            }),
            commands: Some(Container {
                folder: COMMANDS,
                origin: "projectCommands",
            }),
            watched: true,
            plugins_at: first_only,
            extra_folders: true,
            places: Places::Scope(ScopePlaceKind::Project),
        },
        ScanScope::Nested => ScopeRule {
            rank: 3,
            origin: origin("project"),
            layout_origin: SkillOrigin::Project,
            skills: Some(Container {
                folder: SKILLS,
                origin: "projectSkills",
            }),
            commands: None,
            watched: false,
            plugins_at: never,
            extra_folders: true,
            places: Places::Nested,
        },
        ScanScope::Additional => ScopeRule {
            rank: 4,
            origin: origin("project"),
            layout_origin: SkillOrigin::Project,
            skills: Some(Container {
                folder: SKILLS,
                origin: "addDir",
            }),
            commands: Some(Container {
                folder: COMMANDS,
                origin: "addDir",
            }),
            watched: true,
            plugins_at: never,
            extra_folders: true,
            places: Places::Scope(ScopePlaceKind::Added),
        },
        ScanScope::Synced => ScopeRule {
            rank: 5,
            origin: origin("synced"),
            layout_origin: SkillOrigin::Synced,
            skills: Some(Container {
                folder: "skills/synced",
                origin: "synced",
            }),
            commands: None,
            watched: false,
            plugins_at: never,
            // claude.ai's own folder: never an extra one.
            extra_folders: false,
            places: Places::Scope(ScopePlaceKind::User),
        },
    }
}

/// `rule.places(context, roots)`.
pub async fn places_of(places: Places, context: &ScanContext, roots: &[String]) -> Vec<ScopePlace> {
    match places {
        Places::Scope(kind) => scope_places(kind, &context.scope, roots).await,
        Places::Nested => {
            let nested = context.scope.request.nested_directories.clone();
            places_below(async { nested.unwrap_or_default() }, roots).await
        }
    }
}

/// `SCOPE_RULES` as it crosses: each rule's data.
fn scope_rules() -> IndexMap<ScanScope, ScopeRule> {
    SCAN_SCOPES
        .into_iter()
        .map(|scope| (scope, scope_rule(scope)))
        .collect()
}

/// The scopes of a full scan, in precedence order; `nested` is scanned as the session touches
/// files.
pub const SCANNED_SCOPES: [ScanScope; 5] = [
    ScanScope::Managed,
    ScanScope::User,
    ScanScope::Project,
    ScanScope::Additional,
    ScanScope::Synced,
];

crate::registry::exports! {
    "skills/scan/locations" {
        "SCAN_SCOPES" => ok || SCAN_SCOPES;
        "SCOPE_RULES" => ok scope_rules;
    }
}
