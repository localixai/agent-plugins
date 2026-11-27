//! Ported from `src/loading/discover-listed.ts`; only the core uses it now, so it has no wrapper.
//!
//! The files of one directory-shaped component: the default location and the manifest's paths,
//! combined by the kind's rule, each walked, a file reached twice loaded once.

use std::collections::HashMap;

use super::combine::combine;
use super::declared_shape::declared_shape;
use super::discover_config::{self, Found};
use super::ignored_folder::{self, ignored_folder_issues};
use super::readers_text::DiscoveredFile;
use super::walk::{self, ListedKind, walk};
use crate::format::issues::{PluginIssue, error};
use crate::format::layout::{CombineMode, STANDARD_LAYOUT, declared_key};
use crate::format::load_files::Files;
use crate::format::locate::{LocateContext, locate, missing_at};
use crate::format::path_escape::LinkRoots;
use crate::format::path_text::{PathPlatform, root_relative};
use crate::js::{JsObject, JsValue, promise_all};
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};

/// What every discovery step needs (`DiscoverContext`): a `LocateContext` with the manifest, the
/// plugin's name and whether the default `skills/` is scanned.
#[derive(Clone, Debug)]
pub struct DiscoverContext {
    /// The plugin folder.
    pub files: Files,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Where the plugin and its marketplace really are.
    pub roots: Option<LinkRoots>,
    /// Cancels the reads.
    pub signal: Option<Signal>,
    /// The manifest, when there is one.
    pub manifest: JsValue,
    /// The name components are namespaced under; also names a skill at the plugin root.
    pub plugin_name: String,
    /// Whether the default `skills/` is scanned (`EntryMerge.scanDefaultSkills`).
    pub scan_default_skills: bool,
}

impl DiscoverContext {
    /// What `locate` reads of it.
    #[must_use]
    pub fn located(&self) -> LocateContext {
        LocateContext {
            files: self.files.clone(),
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
        }
    }

    /// What a walk reads of it.
    fn walked(&self) -> walk::DiscoverContext {
        walk::DiscoverContext {
            files: self.files.clone(),
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
            plugin_name: self.plugin_name.clone(),
        }
    }

    /// What the ignored-folder warning reads of it.
    fn ignored(&self) -> ignored_folder::DiscoverContext {
        ignored_folder::DiscoverContext {
            manifest: self.manifest.clone(),
            files: self.files.clone(),
            platform: self.platform,
            signal: self.signal.clone(),
        }
    }

    /// What the discovery of a config kind reads of it.
    #[must_use]
    pub fn config(&self) -> discover_config::DiscoverContext {
        discover_config::DiscoverContext {
            files: self.files.clone(),
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
            manifest: self.manifest.clone(),
        }
    }
}

/// The `commands` object map of a manifest, when it uses one.
fn command_map_of(manifest: &JsValue) -> Option<&JsObject> {
    match manifest.get("commands") {
        Some(JsValue::Object(commands)) => Some(commands),
        _ => None,
    }
}

/// The `source` of every entry of a `commands` object map.
fn map_sources(map: &JsObject) -> Vec<String> {
    map.values()
        .filter_map(|entry| {
            entry
                .get("source")
                .and_then(JsValue::as_str)
                .map(str::to_owned)
        })
        .collect()
}

/// Every path a manifest value names: a path, an array of paths, or the sources of a command map.
fn declared_paths(value: &JsValue) -> Vec<String> {
    match value {
        JsValue::String(path) => vec![path.clone()],
        JsValue::Array(entries) => entries
            .iter()
            .filter_map(|entry| entry.as_str().map(str::to_owned))
            .collect(),
        _ => Vec::new(),
    }
}

/// The locations scanned when the manifest does not set the key. Skills have one more: "a `SKILL.md`
/// at the plugin root, with no `skills/` directory and no `skills` manifest key loads as one skill"
/// (components.md, "Skills").
async fn default_locations(kind: ListedKind, context: &DiscoverContext) -> Found<String> {
    let standard = Found {
        found: vec![STANDARD_LAYOUT.of(kind.component()).location.to_owned()],
        issues: Vec::new(),
    };
    if kind == ListedKind::Skills && !context.scan_default_skills {
        return Found {
            found: Vec::new(),
            issues: Vec::new(),
        };
    }
    if kind != ListedKind::Skills || declared_key(&context.manifest, kind.component()).is_some() {
        return standard;
    }
    let skills = STANDARD_LAYOUT.of(kind.component()).location;
    let folder = context.files.stat(skills, context.signal.as_ref()).await;
    if folder.is_some_and(|stat| stat.is_directory) {
        return standard;
    }
    let root = locate(&context.located(), kind.as_str(), "SKILL.md", Some(true)).await;
    if let Some(issue) = root.issue {
        return Found {
            found: standard.found,
            issues: vec![issue],
        };
    }
    if root.stat.is_some_and(|stat| stat.is_file) {
        Found {
            found: vec![".".to_owned()],
            issues: Vec::new(),
        }
    } else {
        standard
    }
}

/// The entries of a `commands` object map: a file named by `source`, or inline `content`.
async fn command_map(map: &JsObject, context: &DiscoverContext) -> Found<DiscoveredFile> {
    let located = context.located();
    let reads: Vec<BoxFuture<'_, Found<DiscoveredFile>>> = map
        .iter()
        .map(|(name, entry)| -> BoxFuture<'_, Found<DiscoveredFile>> {
            let located = &located;
            Box::pin(async move {
                let file = |path| DiscoveredFile {
                    path,
                    name: name.clone(),
                    directory: None,
                    alone: None,
                    entry: entry.clone(),
                };
                let Some(source) = entry.get("source").and_then(JsValue::as_str) else {
                    return Found {
                        found: vec![file(None)],
                        issues: Vec::new(),
                    };
                };
                let at = locate(located, "commands", source, None).await;
                if let Some(issue) = at.issue {
                    return Found {
                        found: Vec::new(),
                        issues: vec![issue],
                    };
                }
                if at.stat.is_none() {
                    let issues = vec![missing_at(located, "commands", source)];
                    return Found {
                        found: Vec::new(),
                        issues,
                    };
                }
                let path = root_relative(source, context.platform);
                Found {
                    found: vec![file(Some(path))],
                    issues: Vec::new(),
                }
            })
        })
        .collect();
    flatten(promise_all(reads).await)
}

/// Every step's files, then every step's issues.
fn flatten(steps: Vec<Found<DiscoveredFile>>) -> Found<DiscoveredFile> {
    let mut found = Vec::new();
    let mut issues = Vec::new();
    for step in steps {
        found.extend(step.found);
        issues.extend(step.issues);
    }
    Found { found, issues }
}

/// One location, default or declared: what it holds, or why it does not load.
async fn walk_one(
    kind: ListedKind,
    path: &str,
    written: Option<&str>,
    context: &DiscoverContext,
) -> Found<DiscoveredFile> {
    let located = locate(
        &context.located(),
        kind.as_str(),
        written.unwrap_or(path),
        None,
    )
    .await;
    if let Some(issue) = located.issue {
        return Found {
            found: Vec::new(),
            issues: vec![issue],
        };
    }
    let Some(stat) = located.stat else {
        let missing = written
            .map(|written| missing_at(&context.located(), kind.as_str(), written))
            .into_iter()
            .collect();
        return Found {
            found: Vec::new(),
            issues: missing,
        };
    };
    let wrong = written.and_then(|written| declared_shape(kind, &stat, written));
    if let Some(wrong) = wrong {
        return Found {
            found: Vec::new(),
            issues: vec![error(kind.as_str(), &wrong)],
        };
    }
    walk(kind, &context.walked(), path, &stat, written.is_some()).await
}

/// Where a found file really lives: a link inside the plugin counts as its target.
struct Place {
    /// Where the file really lives: a link inside the plugin counts as its target (`realPath`).
    key: Option<String>,
    linked: bool,
    /// A file link that leads outside the plugin: the file does not load (`locate`).
    issue: Option<PluginIssue>,
}

/// Where a found file really lives: a link inside the plugin counts as its target (`realPath`, relative
/// to the folder), so `agents/a.agent.md -> a.md` or `skills/alias -> real` is the same file as its
/// target; a skill is its folder. Inline commands have no path. A file link found by a scan goes
/// through `locate` like a declared path ("A symlink that leads outside the plugin", loading.md);
/// folder links were judged by the scan.
async fn place_of(kind: ListedKind, file: &DiscoveredFile, context: &DiscoverContext) -> Place {
    let Some(path) = file.directory.as_ref().or(file.path.as_ref()) else {
        return Place {
            key: None,
            linked: false,
            issue: None,
        };
    };
    let stat = context.files.stat(path, context.signal.as_ref()).await;
    let real = stat.and_then(|stat| stat.is_symlink.then_some(stat.real_path).flatten());
    let linked = real.is_some();
    let issue = if linked && file.directory.is_none() {
        locate(&context.located(), kind.as_str(), path, None)
            .await
            .issue
    } else {
        None
    };
    Place {
        key: real.or_else(|| Some(path.clone())),
        linked,
        issue,
    }
}

/// Which occurrence of a file or skill folder reached twice stays (Claude Code 2.1.322,
/// spec/oracle/runtime/symlinked-duplicates): the entry itself over a link to it, whatever their
/// order; of links only, the first in folder order.
fn keeper_of(places: &[Place]) -> HashMap<&str, usize> {
    let mut keeper: HashMap<&str, usize> = HashMap::new();
    for (index, place) in places.iter().enumerate() {
        let Some(key) = place.key.as_deref() else {
            continue;
        };
        let kept = keeper.get(key).copied();
        let better = kept.is_some_and(|kept| {
            places.get(kept).is_some_and(|other| other.linked) && !place.linked
        });
        if kept.is_none() || better {
            keeper.insert(key, index);
        }
    }
    keeper
}

/// A file reached twice loads once (oracle 2.1.319): a folder listed in the manifest that is also
/// inside the default `skills/` folder, or a link inside the plugin to another file of the same
/// component. Inline commands have no path and all stay.
async fn first_of_each(
    kind: ListedKind,
    found: Vec<DiscoveredFile>,
    context: &DiscoverContext,
) -> Found<DiscoveredFile> {
    let places = promise_all(
        found
            .iter()
            .map(|file| -> BoxFuture<'_, Place> { Box::pin(place_of(kind, file, context)) })
            .collect(),
    )
    .await;
    let keeper = keeper_of(&places);
    let kept = found
        .into_iter()
        .zip(&places)
        .enumerate()
        .filter(|(index, (_, place))| {
            place.issue.is_none()
                && place
                    .key
                    .as_deref()
                    .is_none_or(|key| keeper.get(key) == Some(index))
        })
        .map(|(_, (file, _))| file)
        .collect();
    let issues = places
        .iter()
        .filter_map(|place| place.issue.clone())
        .collect();
    Found {
        found: kept,
        issues,
    }
}

/// The files of one directory-shaped component (plugins/manifest-reference.md, "Standard layout",
/// "How each key combines with its default location"): the default location and the manifest's paths,
/// combined by the kind's rule; a default that is not there is skipped, a declared path that is not
/// there is `<component> path not found: <path>`.
pub async fn discover_listed(kind: ListedKind, context: &DiscoverContext) -> Found<DiscoveredFile> {
    let declared = declared_key(&context.manifest, kind.component());
    let map = if kind == ListedKind::Commands {
        command_map_of(&context.manifest)
    } else {
        None
    };
    let paths =
        declared.map(|declared| map.map_or_else(|| declared_paths(&declared.value), map_sources));
    let warnings = ignored_folder_issues(
        kind.component(),
        paths.as_deref().unwrap_or_default(),
        &context.ignored(),
    )
    .await;
    if let Some(map) = map {
        let entries = command_map(map, context).await;
        return Found {
            found: entries.found,
            issues: [warnings, entries.issues].concat(),
        };
    }
    let defaults = default_locations(kind, context).await;
    let relative = |path: &String| root_relative(path, context.platform);
    let written: HashMap<String, &String> = paths
        .iter()
        .flatten()
        .map(|path| (relative(path), path))
        .collect();
    let mode = STANDARD_LAYOUT
        .of(kind.component())
        .combine
        .unwrap_or(CombineMode::Replace);
    let declared_relative = paths
        .as_ref()
        .map(|paths| paths.iter().map(relative).collect::<Vec<_>>());
    let order = combine(
        mode,
        &defaults.found.iter().map(relative).collect::<Vec<_>>(),
        declared_relative.as_deref(),
    );
    let walked = promise_all(
        order
            .iter()
            .map(|path| -> BoxFuture<'_, Found<DiscoveredFile>> {
                let written = written.get(path).map(|written| written.as_str());
                Box::pin(walk_one(kind, path, written, context))
            })
            .collect(),
    )
    .await;
    let walked = flatten(walked);
    let kept = first_of_each(kind, walked.found, context).await;
    Found {
        found: kept.found,
        issues: [defaults.issues, warnings, walked.issues, kept.issues].concat(),
    }
}
