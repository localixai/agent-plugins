//! Ported from `src/skills/scan/read.ts`; only the core uses it now, so it has no wrapper.
//!
//! Reads the skills and the command files of one scope folder into catalog entries (skills.md,
//! "Choose where skills load", "How a skill gets its command name"). Each entry keeps the
//! frontmatter and a `BodyReader`, never the body.
//!
//! A `BodyReader` the TypeScript makes as a closure is a struct here (`SkillBody`, `CommandBody`)
//! holding what the closure held: the host's `FolderScan`, the folder's directory, the file and how
//! it is read again.

use std::sync::Arc;

use super::head::head_of;
use super::scan_types::{ScanOutcome, ScannedEntry, SkillsDirectoryPlugin, Where, empty, merge};
use crate::format::command::{CommandLayout, read_command};
use crate::format::issues::blocks_loading;
use crate::format::skill::read_skill;
use crate::format::skill_name::SkillLayout;
use crate::format::skill_types::{BodyReader, ReadBody};
use crate::foundation::folder_read::read_text_of;
use crate::foundation::folder_walk::list_children;
use crate::foundation::scope_folders::{Depth, ScopeFilesOptions, read_scope_files};
use crate::js::JsValue;
use crate::js::promise_all;
use crate::ports::folder_scan::FolderScan;
use crate::ports::{BoxFuture, Signal};

/// The file a skill folder holds.
pub const SKILL_FILE: &str = "SKILL.md";
/// Where Claude Code moves the skill folders it removes; they stay out of every scan (skills.md,
/// "Personal skills disappeared").
const TRASH: &str = ".trash";

/// The text of the file as it is now: the folder is opened again, so a host whose `ScanFolder`
/// pins what it saw at the scan (a handle, a resolved root) is asked afresh. `None` when the folder
/// or the file is gone.
async fn text_now(
    folders: &dyn FolderScan,
    dir: &str,
    path: &str,
    signal: Option<&Signal>,
) -> Option<String> {
    let folder = folders.open(dir, signal).await?;
    folder.files().await.read_text(path, signal).await
}

/// The body of the skill at `path` as it is now, or `None` when it is gone or no longer loads
/// (`skillBodyReader`).
struct SkillBody {
    folders: Arc<dyn FolderScan>,
    dir: String,
    path: String,
    host_file: String,
    layout: SkillLayout,
    origin: JsValue,
}

impl ReadBody for SkillBody {
    fn read_body<'a>(&'a self, signal: Option<&'a Signal>) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let text = text_now(self.folders.as_ref(), &self.dir, &self.path, signal).await?;
            let again = read_skill(&self.host_file, &text, &self.layout, self.origin.clone());
            if blocks_loading(&again.issues) {
                return None;
            }
            again
                .skill
                .get("body")
                .and_then(JsValue::as_str)
                .map(str::to_owned)
        })
    }
}

/// Reads one skill folder `<container>/<name>`; absent file, no entry.
pub async fn read_skill_folder(
    place: &Where<'_>,
    name: &str,
    path: &str,
    canonical: Option<String>,
) -> ScanOutcome {
    let Where {
        folder,
        context,
        rule,
        scope,
        ..
    } = place;
    let signal = context.scope.signal.as_ref();
    let file = if path == "." {
        SKILL_FILE.to_owned()
    } else {
        format!("{path}/{SKILL_FILE}")
    };
    let read = read_text_of(folder.as_ref(), &file, signal).await;
    let Some(text) = read.text else {
        return ScanOutcome {
            issues: read.issue.into_iter().collect(),
            ..empty()
        };
    };
    let host_file = folder.host_path(&file).await;
    let layout = SkillLayout {
        origin: rule.layout_origin.clone(),
        dir_name: name.to_owned(),
        fixed_name: None,
    };
    let skill = read_skill(&host_file, &text, &layout, rule.origin.clone());
    let base = folder.dir().await;
    let reader = SkillBody {
        folders: Arc::clone(&context.scope.folders),
        dir: base.clone(),
        path: file,
        host_file,
        layout,
        origin: rule.origin.clone(),
    };
    let entry = ScannedEntry {
        invocable: head_of(skill.skill),
        read_body: BodyReader::core(reader),
        scope: *scope,
        dir_name: name.to_owned(),
        base,
        canonical,
    };
    let loads = !blocks_loading(&skill.issues);
    ScanOutcome {
        entries: if loads { vec![entry] } else { Vec::new() },
        issues: skill.issues,
        ..empty()
    }
}

/// `.claude/skills/<name>/SKILL.md`: one level, a folder (or a link to one) per skill.
pub async fn read_skills<'a>(
    place: &'a Where<'a>,
    container: &str,
    plugins: impl Fn(String, String) -> BoxFuture<'a, Vec<SkillsDirectoryPlugin>> + Sync,
) -> ScanOutcome {
    let signal = place.context.scope.signal.as_ref();
    let listed = list_children(place.folder.as_ref(), container, signal).await;
    let plugins = &plugins;
    let parts = promise_all(
        listed
            .found
            .into_iter()
            .filter(|child| child.is_directory && child.name != TRASH)
            .map(|child| -> BoxFuture<'_, ScanOutcome> {
                Box::pin(async move {
                    let read =
                        read_skill_folder(place, &child.name, &child.path, child.canonical).await;
                    ScanOutcome {
                        plugins: plugins(child.name, child.path).await,
                        ..read
                    }
                })
            })
            .collect(),
    )
    .await;
    let mut found = merge(parts);
    let mut issues = listed.issues;
    issues.append(&mut found.issues);
    ScanOutcome { issues, ..found }
}

/// A command file's body as it is now (`readCommands`' `readBody`).
struct CommandBody {
    folders: Arc<dyn FolderScan>,
    dir: String,
    path: String,
    host_path: String,
    layout: CommandLayout,
}

impl ReadBody for CommandBody {
    fn read_body<'a>(&'a self, signal: Option<&'a Signal>) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            let now = text_now(self.folders.as_ref(), &self.dir, &self.path, signal).await?;
            let command = read_command(&self.host_path, &now, &self.layout)
                .ok()?
                .command?;
            command
                .get("body")
                .and_then(JsValue::as_str)
                .map(str::to_owned)
        })
    }
}

/// `.claude/commands/<path>.md`: the file name, and each folder before it joined by `:`.
pub async fn read_commands(place: &Where<'_>, container: &str) -> ScanOutcome {
    let Where {
        folder,
        context,
        rule,
        scope,
        ..
    } = place;
    let options = ScopeFilesOptions {
        extension: ".md".to_owned(),
        depth: Depth::Tree,
        signal: context.scope.signal.clone(),
    };
    let read = read_scope_files(folder.as_ref(), container, &options).await;
    let base = folder.dir().await;
    let mut entries = Vec::new();
    let mut issues = read.issues;
    for file in read.files {
        let layout = CommandLayout {
            origin: rule.origin.clone(),
            relative_path: file.segments.join("/"),
        };
        // The origin is a scope rule's, always a `SkillOrigin`: the read never refuses it.
        let Ok(command) = read_command(&file.host_path, &file.text, &layout) else {
            continue;
        };
        issues.extend(command.issues);
        let Some(command) = command.command else {
            continue;
        };
        let dir_name = command
            .get("name")
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned();
        let reader = CommandBody {
            folders: Arc::clone(&context.scope.folders),
            dir: base.clone(),
            path: file.path,
            host_path: file.host_path,
            layout,
        };
        entries.push(ScannedEntry {
            invocable: head_of(command),
            read_body: BodyReader::core(reader),
            scope: *scope,
            dir_name,
            base: base.clone(),
            canonical: file.canonical,
        });
    }
    ScanOutcome {
        entries,
        issues,
        ..empty()
    }
}
