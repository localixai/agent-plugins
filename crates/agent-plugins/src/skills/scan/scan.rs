//! Port of `src/skills/scan/scan.ts`, switched.
//!
//! Reads the skills and command files of the scope folders through the `FolderScan` port
//! (skills.md, "Choose where skills load", "How a skill gets its command name"):
//! `<base>/.claude/skills/<name>/SKILL.md` and the flat `<base>/.claude/commands/**.md`, then the
//! same below each extra scope folder the host names (`ScopeContext.extraScopeFolders`). Nothing is
//! resolved here: two skills of one name are both returned, with the scope each came from, for
//! `resolveFolderSkills` to decide. Entries carry no body (`ScannedEntry.readBody` reads it when
//! the skill runs).

use super::locations::{
    Container, SCANNED_SCOPES, ScanContext, ScanRequest, ScanScope, places_of, scan_context_of,
    scope_rule,
};
use super::read::{read_commands, read_skills};
use super::scan_types::{ScanOutcome, SkillsDirectoryPlugin, Where, empty, merge};
use crate::format::read_json::{JsonRead, read_json};
use crate::foundation::scope_folders::{
    CLAUDE_FOLDER, HostContext, ScopePlace, WatchOptions, place_path, scope_folders_of,
    watched_folder,
};
use crate::js::JsValue;
use crate::js::{promise_all, promise_all_pair};
use crate::ports::BoxFuture;
use crate::protocol::is_record::is_record;

const PLUGIN_MANIFEST: &str = ".claude-plugin/plugin.json";

/// The manifest's `name` when it reads, else the folder's. Reading never fails the scan.
async fn plugin_name(place: &Where<'_>, directory: &str, fallback: &str) -> String {
    let files = place.folder.files().await;
    let path = format!("{directory}/{PLUGIN_MANIFEST}");
    let read = read_json(files.as_ref(), &path, place.context.scope.signal.as_ref()).await;
    let name = match &read {
        JsonRead::Value { value } if is_record(value) => value.get("name"),
        JsonRead::Value { .. } | JsonRead::Problem { .. } => None,
    };
    match name.and_then(JsValue::as_str) {
        Some(name) if !name.is_empty() => name.to_owned(),
        _ => fallback.to_owned(),
    }
}

async fn plugin_of(place: &Where<'_>, name: &str, directory: &str) -> Vec<SkillsDirectoryPlugin> {
    let Where {
        folder,
        context,
        scope,
        rule,
        ..
    } = place;
    // "Skill folder as a plugin" names the `.claude` skills folders only.
    if !place.place.documented || !(rule.plugins_at)(place.place.index) {
        return Vec::new();
    }
    let manifest = format!("{directory}/{PLUGIN_MANIFEST}");
    let signal = context.scope.signal.as_ref();
    if !folder.files().await.exists(&manifest, signal).await {
        return Vec::new();
    }
    let plugin_named = plugin_name(place, directory, name).await;
    vec![SkillsDirectoryPlugin {
        id: format!("{plugin_named}@skills-dir"),
        name: plugin_named,
        dir: folder.host_path(directory).await,
        scope: *scope,
        requires_trust: *scope == ScanScope::Project,
    }]
}

/// The container a source loads from, unless `strictPluginOnlyCustomization` stops that source.
async fn allowed_container(
    context: &ScanContext,
    container: Option<Container>,
) -> Option<Container> {
    let container = container?;
    match &context.allows {
        Some(allows) if !allows.allows(container.origin).await => None,
        _ => Some(container),
    }
}

async fn scan_base(context: &ScanContext, scope: ScanScope, place: ScopePlace) -> ScanOutcome {
    let rule = scope_rule(scope);
    let signal = context.scope.signal.as_ref();
    let Some(folder) = context.scope.folders.open(&place.base, signal).await else {
        return empty();
    };
    let skills = allowed_container(context, rule.skills).await;
    let commands = allowed_container(context, rule.commands).await;
    let watched = rule.watched;
    let place = Where {
        context,
        rule,
        scope,
        folder,
        place,
    };
    let skills_path = skills.map(|skills| place_path(&place.place.root, skills.folder));
    let commands_path = commands.map(|commands| place_path(&place.place.root, commands.folder));
    let place = &place;
    let (skills_read, commands_read) = promise_all_pair(
        Box::pin(async {
            match &skills_path {
                None => empty(),
                Some(path) => {
                    read_skills(place, path, |name, path| -> BoxFuture<'_, _> {
                        Box::pin(async move { plugin_of(place, &name, &path).await })
                    })
                    .await
                }
            }
        }),
        Box::pin(async {
            match &commands_path {
                None => empty(),
                Some(path) => read_commands(place, path).await,
            }
        }),
    )
    .await;
    let watch = match &skills_path {
        None => Vec::new(),
        Some(path) => {
            let options = WatchOptions {
                watched,
                signal: context.scope.signal.clone(),
            };
            watched_folder(place.folder.as_ref(), path, &options).await
        }
    };
    ScanOutcome {
        watch,
        ..merge(vec![skills_read, commands_read])
    }
}

/// One scope: every base directory of it, in order, below `.claude` and then below each of the
/// host's extra folders (`scopeFoldersOf`), so an extra folder's entries come after every
/// `.claude` entry of the scope and lose a shared name to them.
pub async fn scan_scope(context: &ScanContext, scope: ScanScope) -> ScanOutcome {
    let rule = scope_rule(scope);
    let roots = if rule.extra_folders {
        scope_folders_of(&context.scope)
    } else {
        vec![CLAUDE_FOLDER.to_owned()]
    };
    let mut parts = Vec::new();
    for place in places_of(rule.places, context, &roots).await {
        parts.push(scan_base(context, scope, place).await);
    }
    merge(parts)
}

/// Every scope of a session, in precedence order: managed, personal, project (the working
/// directory and its parents), the added directories, and the skills synced from claude.ai. The
/// skills below the working directory are `scanNested`'s, as the session touches files there.
pub async fn scan_folders(context: &ScanContext) -> ScanOutcome {
    let parts = promise_all(
        SCANNED_SCOPES
            .into_iter()
            .map(|scope| -> BoxFuture<'_, ScanOutcome> { Box::pin(scan_scope(context, scope)) })
            .collect(),
    )
    .await;
    merge(parts)
}

crate::registry::exports! {
    "skills/scan/scan" {
        "scanScope" => async ok |context: HostContext<ScanRequest>, scope: ScanScope|
            scan_scope(&scan_context_of(context).await, scope).await;
        "scanFolders" => async ok |context: HostContext<ScanRequest>|
            scan_folders(&scan_context_of(context).await).await;
    }
}
