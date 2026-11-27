//! Port of `src/loading/walk.ts`, switched.
//!
//! How each directory-shaped component turns one existing location (a file or a folder, default or
//! declared) into the files it holds (plugins/manifest-reference.md, "Standard layout" and "Path-only
//! fields"; plugins/components.md for each kind). One strategy per kind, typed over the union.
//!
//! `WALKERS` is a table of the host's functions in the TypeScript; here it is [`walk`], one async
//! function over the kind, and the wrapper builds the table's host-facing shape from it.

use serde::{Deserialize, Serialize};

use super::discover_config::Found;
use super::readers_text::DiscoveredFile;
use crate::format::issues::PluginIssue;
use crate::format::layout::{ComponentKind, STANDARD_LAYOUT};
use crate::format::listing::list_sorted;
use crate::format::load_files::Files;
use crate::format::locate::{LocateContext, locate};
use crate::format::path_escape::LinkRoots;
use crate::format::path_text::{PathPlatform, child_path};
use crate::js::JsValue;
use crate::ports::files::{DirectoryEntry, FileStat, PluginFiles};
use crate::ports::{BoxFuture, Signal};

/// The kinds a manifest lists paths of, in the table's order.
pub const LISTED_KINDS: [ListedKind; 6] = [
    ListedKind::Skills,
    ListedKind::Commands,
    ListedKind::Agents,
    ListedKind::OutputStyles,
    ListedKind::Workflows,
    ListedKind::Themes,
];

/// One of `LISTED_KINDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListedKind {
    /// `skills`.
    Skills,
    /// `commands`.
    Commands,
    /// `agents`.
    Agents,
    /// `outputStyles`.
    OutputStyles,
    /// `workflows`.
    Workflows,
    /// `themes`.
    Themes,
}

impl ListedKind {
    /// The kind as the layout and the messages name it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Skills => "skills",
            Self::Commands => "commands",
            Self::Agents => "agents",
            Self::OutputStyles => "outputStyles",
            Self::Workflows => "workflows",
            Self::Themes => "themes",
        }
    }

    /// The component kind of the layout it is.
    #[must_use]
    pub const fn component(self) -> ComponentKind {
        match self {
            Self::Skills => ComponentKind::Skills,
            Self::Commands => ComponentKind::Commands,
            Self::Agents => ComponentKind::Agents,
            Self::OutputStyles => ComponentKind::OutputStyles,
            Self::Workflows => ComponentKind::Workflows,
            Self::Themes => ComponentKind::Themes,
        }
    }
}

/// What a walk reads of its context (`DiscoverContext`): the folder and what `locate` needs, and
/// the plugin's name.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverContext {
    /// The plugin folder.
    pub files: Files,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Where the plugin and its marketplace really are.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// The name components are namespaced under; also names a skill at the plugin root.
    pub plugin_name: String,
}

impl DiscoverContext {
    fn located(&self) -> LocateContext {
        LocateContext {
            files: self.files.clone(),
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
        }
    }
}

const SKILL_FILE: &str = "SKILL.md";
/// A folder tree deeper than this is not scanned further: a port that reports no links cannot tell
/// the scan that a folder loops (judgment); a reported link is never entered by a nested scan.
const MAX_DEPTH: usize = 16;

/// The file name without its last extension.
fn stem(name: &str) -> &str {
    match name.rfind('.') {
        Some(dot) if dot > 0 => name.get(..dot).unwrap_or(name),
        _ => name,
    }
}

fn base_name(path: &str) -> &str {
    path.rfind('/')
        .and_then(|slash| path.get(slash + 1..))
        .unwrap_or(path)
}

fn file(path: String, name: String, directory: Option<String>) -> DiscoveredFile {
    DiscoveredFile {
        path: Some(path),
        name,
        directory,
        alone: None,
        entry: JsValue::Undefined,
    }
}

const fn found(found: Vec<DiscoveredFile>, issues: Vec<PluginIssue>) -> Found<DiscoveredFile> {
    Found { found, issues }
}

/// A declared file is taken as given whatever its extension: the user named it.
fn single(path: &str) -> Found<DiscoveredFile> {
    found(
        vec![file(
            path.to_owned(),
            stem(base_name(path)).to_owned(),
            None,
        )],
        Vec::new(),
    )
}

async fn entries_of(context: &DiscoverContext, path: &str) -> Vec<DirectoryEntry> {
    let files: &dyn PluginFiles = &context.files;
    list_sorted(files, path, context.signal.as_ref())
        .await
        .unwrap_or_default()
}

/// Whether a scan enters a symlinked folder it finds (Claude Code 2.1.322, spec/judgments.md): a
/// skills scan follows a linked skill folder, a commands or agents scan does not enter a linked
/// folder at all; the flat kinds read no folders.
const fn follows_links(kind: ListedKind) -> bool {
    matches!(kind, ListedKind::Skills)
}

/// One scan of one location, and the links it refused; for a nested scan, the extension and
/// whether a folder holding `SKILL.md` is one skill, and nothing under it is scanned (commands).
struct Scan<'a> {
    context: &'a DiscoverContext,
    kind: ListedKind,
    issues: Vec<PluginIssue>,
    extension: Option<&'static str>,
    skill_folders: bool,
}

const fn start_scan(context: &DiscoverContext, kind: ListedKind) -> Scan<'_> {
    Scan {
        context,
        kind,
        issues: Vec::new(),
        extension: None,
        skill_folders: false,
    }
}

/// The folder `name` below `parent`, when the scan enters it: a link only where the kind follows
/// links, and then through `locate` like a declared path ("A symlink that leads outside the plugin"
/// is a path-escapes error, loading.md). Which of a folder and a link to it loads is
/// `discover-listed`'s (`keeperOf`).
async fn enter(scan: &mut Scan<'_>, parent: &str, name: &str) -> Option<String> {
    let path = child_path(parent, name);
    let context = scan.context;
    let stat = context.files.stat(&path, context.signal.as_ref()).await;
    if !stat.is_some_and(|stat| stat.is_symlink) {
        return Some(path);
    }
    if !follows_links(scan.kind) {
        return None;
    }
    let located = locate(&context.located(), scan.kind.as_str(), &path, None).await;
    match located.issue {
        Some(issue) => {
            scan.issues.push(issue);
            None
        }
        None => Some(path),
    }
}

/// `one.isDirectory ? await enter(scan, parent, one.name) : undefined`.
async fn child_of(scan: &mut Scan<'_>, parent: &str, one: &DirectoryEntry) -> Option<String> {
    if one.is_directory {
        enter(scan, parent, &one.name).await
    } else {
        None
    }
}

fn done(scan: Scan<'_>, files: Vec<DiscoveredFile>) -> Found<DiscoveredFile> {
    found(files, scan.issues)
}

fn has_extension(name: &str, extension: Option<&str>) -> bool {
    extension.is_none_or(|extension| name.ends_with(extension))
}

/// The direct children of a folder that are files of the kind's extension.
async fn scan_flat(
    context: &DiscoverContext,
    path: &str,
    extension: Option<&str>,
) -> Found<DiscoveredFile> {
    let entries = entries_of(context, path).await;
    let files = entries
        .iter()
        .filter(|one| one.is_file && has_extension(&one.name, extension))
        .map(|one| {
            file(
                child_path(path, &one.name),
                stem(&one.name).to_owned(),
                None,
            )
        })
        .collect();
    found(files, Vec::new())
}

async fn holds_skill(context: &DiscoverContext, folder: &str) -> bool {
    let path = child_path(folder, SKILL_FILE);
    context.files.exists(&path, context.signal.as_ref()).await
}

/// A skill folder found by a commands scan: named by its folder, never by its frontmatter `name`.
fn skill_folder(folder: &str, name: String) -> DiscoveredFile {
    file(
        child_path(folder, SKILL_FILE),
        name,
        Some(folder.to_owned()),
    )
}

/// What one child folder of a nested scan holds: a skill, or the files below it.
async fn nested_folder(
    scan: &mut Scan<'_>,
    folder: &str,
    name: &str,
    (prefix, depth): (&str, usize),
) -> Vec<DiscoveredFile> {
    if scan.skill_folders && holds_skill(scan.context, folder).await {
        return vec![skill_folder(folder, format!("{prefix}{name}"))];
    }
    scan_nested(scan, folder, &format!("{prefix}{name}:"), depth + 1).await
}

/// Files at any depth; every folder on the way becomes a `:`-joined segment of the name
/// (`agents/review/security.md` is `review:security`, `commands/db/migrate.md` is `db:migrate`).
/// Folder by folder in sorted order.
fn scan_nested<'a>(
    scan: &'a mut Scan<'_>,
    folder: &'a str,
    prefix: &'a str,
    depth: usize,
) -> BoxFuture<'a, Vec<DiscoveredFile>> {
    Box::pin(async move {
        if depth > MAX_DEPTH {
            return Vec::new();
        }
        let mut files = Vec::new();
        for one in entries_of(scan.context, folder).await {
            if let Some(child) = child_of(scan, folder, &one).await {
                files.extend(nested_folder(scan, &child, &one.name, (prefix, depth)).await);
            }
            if one.is_file && has_extension(&one.name, scan.extension) {
                let name = format!("{prefix}{}", stem(&one.name));
                files.push(file(child_path(folder, &one.name), name, None));
            }
        }
        files
    })
}

/// Skills: "each a directory of `<name>/SKILL.md` folders or one folder holding `SKILL.md`
/// directly" (manifest-reference, "Fields"). A folder with a `SKILL.md` of its own is one skill
/// named after it (the plugin root, named after the plugin: judgment, the frontmatter `name` decides
/// the rest); otherwise each child folder with a `SKILL.md` is a skill.
async fn skills(
    context: &DiscoverContext,
    path: &str,
    stat: &FileStat,
    declared: bool,
) -> Found<DiscoveredFile> {
    if !stat.is_directory {
        return found(Vec::new(), Vec::new());
    }
    let own = holds_skill(context, path).await;
    // Oracle 2.1.319: the default `skills/` folder is only a parent, a `SKILL.md` directly in it is no skill.
    if own && (declared || path == ".") {
        let name = if path == "." {
            &context.plugin_name
        } else {
            base_name(path)
        };
        let mut skill = file(
            child_path(path, SKILL_FILE),
            name.to_owned(),
            Some(path.to_owned()),
        );
        skill.alone = Some(true);
        return found(vec![skill], Vec::new());
    }
    let mut scan = start_scan(context, ListedKind::Skills);
    let mut files = Vec::new();
    for one in entries_of(context, path).await {
        if let Some(child) = child_of(&mut scan, path, &one).await
            && holds_skill(context, &child).await
        {
            files.push(file(child_path(&child, SKILL_FILE), one.name, Some(child)));
        }
    }
    done(scan, files)
}

async fn nested(
    kind: ListedKind,
    extension: Option<&'static str>,
    (context, path, stat): (&DiscoverContext, &str, &FileStat),
) -> Found<DiscoveredFile> {
    if !stat.is_directory {
        return single(path);
    }
    let mut scan = Scan {
        extension,
        ..start_scan(context, kind)
    };
    let files = scan_nested(&mut scan, path, "", 0).await;
    done(scan, files)
}

/// Output styles: the `.md` files of the folder and of its subfolders, each named by its own file
/// name, the subfolder no part of it (spec J6.4, oracle 2.1.325: `output-styles/sub/deep.md` is
/// `<plugin>:deep`). The frontmatter `name` decides the rest (format/output-style.ts).
async fn deep(
    kind: ListedKind,
    extension: Option<&'static str>,
    (context, path, stat): (&DiscoverContext, &str, &FileStat),
) -> Found<DiscoveredFile> {
    if !stat.is_directory {
        return single(path);
    }
    let mut scan = Scan {
        extension,
        ..start_scan(context, kind)
    };
    let files = scan_nested(&mut scan, path, "", 0).await;
    let files = files
        .into_iter()
        .map(|mut file| {
            if let Some(path) = &file.path {
                file.name = stem(base_name(path)).to_owned();
            }
            file
        })
        .collect();
    done(scan, files)
}

async fn flat(
    extension: Option<&'static str>,
    (context, path, stat): (&DiscoverContext, &str, &FileStat),
) -> Found<DiscoveredFile> {
    if stat.is_directory {
        scan_flat(context, path, extension).await
    } else {
        single(path)
    }
}

/// Commands and agents nest (components.md: `commands/db/migrate.md` is `/my-plugin:db:migrate`,
/// "agents ... loads recursively"); output styles read their subfolders without naming them
/// (`deep`); workflows and themes are one folder of files (judgment: the docs name `workflows/`
/// `.js` files and `themes/<slug>.json`, no nesting).
/// A declared agent file loads without subfolder names ("a file you list there loads without
/// subfolder names"): its name is its file name.
///
/// Commands, oracle 2.1.319 (spec/judgments.md): a folder holding `SKILL.md` that a commands scan
/// reaches is a skill named by its folder (`commands/nest/deep/SKILL.md` is `nest:deep`) and nothing
/// under it loads; a commands location that holds `SKILL.md` itself, the default `commands/`
/// included, is that one skill. Discovery hands these to the skills (`discoverComponents`).
async fn commands(context: &DiscoverContext, path: &str, stat: &FileStat) -> Found<DiscoveredFile> {
    if !stat.is_directory {
        return single(path);
    }
    if holds_skill(context, path).await {
        let name = if path == "." {
            &context.plugin_name
        } else {
            base_name(path)
        };
        return found(vec![skill_folder(path, name.to_owned())], Vec::new());
    }
    let mut scan = Scan {
        extension: STANDARD_LAYOUT.of(ComponentKind::Commands).extension,
        skill_folders: true,
        ..start_scan(context, ListedKind::Commands)
    };
    let files = scan_nested(&mut scan, path, "", 0).await;
    done(scan, files)
}

/// `WALKERS[kind](context, path, stat, declared)`: the files one location of the kind holds.
pub async fn walk(
    kind: ListedKind,
    context: &DiscoverContext,
    path: &str,
    stat: &FileStat,
    declared: bool,
) -> Found<DiscoveredFile> {
    let at = (context, path, stat);
    match kind {
        ListedKind::Skills => skills(context, path, stat, declared).await,
        ListedKind::Commands => commands(context, path, stat).await,
        ListedKind::Agents => {
            nested(
                kind,
                STANDARD_LAYOUT.of(ComponentKind::Agents).extension,
                at,
            )
            .await
        }
        ListedKind::OutputStyles => {
            deep(
                kind,
                STANDARD_LAYOUT.of(ComponentKind::OutputStyles).extension,
                at,
            )
            .await
        }
        ListedKind::Workflows => {
            flat(STANDARD_LAYOUT.of(ComponentKind::Workflows).extension, at).await
        }
        ListedKind::Themes => flat(STANDARD_LAYOUT.of(ComponentKind::Themes).extension, at).await,
    }
}

crate::registry::exports! {
    "loading/walk" {
        "LISTED_KINDS" => ok || LISTED_KINDS;
        "walk" => async ok |kind: ListedKind, context: DiscoverContext, path: String, stat: FileStat, declared: bool|
            walk(kind, &context, &path, &stat, declared).await;
    }
}
