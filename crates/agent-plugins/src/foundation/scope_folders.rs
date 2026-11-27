//! Port of `src/foundation/scope-folders.ts`, switched.
//!
//! The scope folders of a session, shared by the skills, the agents and the saved-workflows scans:
//! where each scope's base directories are (the managed directory, the project and its parents, the
//! added directories, the home), and the files of one folder below a base, read through the host's
//! `FolderScan` port. Each scan keeps its own table of scopes and its own parser; this is what the
//! tables and the parsers stand on.
//!
//! The TypeScript's tables of functions (`SCOPE_BASES`, `SCOPE_PLACES`) are functions of the table's
//! key here (`scope_bases`, `scope_places`), and cross as objects whose methods are the entries. A
//! `ScopeContext` crosses as data whose `folders` is the host's port, or as a port of its own when
//! it holds a function (a scan's `allows`), its members then read one by one (`scope_context_of`).
//!
//! The reads of a folder's files, the source's second half, are in `scope_folders/files.rs` to keep
//! each file in RS5.

use std::future::Future;
use std::sync::Arc;

use indexmap::IndexSet;
use serde::{Deserialize, Serialize};

mod files;

pub use files::{
    Depth, ScopeFile, ScopeFiles, ScopeFilesOptions, WatchOptions, read_scope_files, watched_folder,
};

use super::project_directories::project_directories;
use crate::format::config_directory::config_folder_of;
use crate::js::JsValue;
use crate::ports::folder_scan::FolderScan;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Callback, Handle, Port};

/// What a scan of the scope folders is asked for.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeRequest {
    /// The home directory.
    pub home: String,
    /// The working directory the session started in (or moved to with `/cd`).
    pub cwd: String,
    /// The directories passed with `--add-dir` or `/add-dir`, in the order they were added.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub added_directories: Option<Vec<String>>,
    /// `CLAUDE_CONFIG_DIR` as the session's environment gives it; unset or empty, `<home>/.claude`.
    /// The user scope's documented folder is that directory (`format/config-directory`); the host's
    /// extra scope folders of the user scope stay below `home`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_dir: Option<String>,
}

impl AsRef<Self> for ScopeRequest {
    fn as_ref(&self) -> &Self {
        self
    }
}

/// What a scan of the scope folders reads with; `R` is a scan's own request, which holds a
/// `ScopeRequest`.
#[derive(Clone)]
pub struct ScopeContext<R = ScopeRequest> {
    /// The host's scope folders.
    pub folders: Arc<dyn FolderScan>,
    /// What the scan is asked for.
    pub request: R,
    /// Ends the scan early.
    pub signal: Option<Signal>,
    /// Folders beside `.claude` that the host also reads skills, commands and agents from, below
    /// every base of every scope that reads its `.claude` folder (`.agents`, `.localix`). See
    /// `scope_folders_of`.
    pub extra_scope_folders: Option<Vec<String>>,
}

/// A `ScopeContext` as it crosses: data whose `folders` is the host's port, or, when it holds a
/// function of its own (a scan's `allows`), a port whose members are read.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum HostContext {
    /// A context that holds a function: its members are read one by one.
    Host(Port),
    /// A context of data and the host's `folders`.
    Data(ContextData),
}

/// The members of a `ScopeContext` that crossed as data.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextData {
    folders: Port,
    request: ScopeRequest,
    #[serde(default)]
    signal: Option<Signal>,
    #[serde(default)]
    extra_scope_folders: Option<Vec<String>>,
}

/// The host's `ScopeContext`, its members read when it crossed as a port.
pub async fn scope_context_of(context: HostContext) -> ScopeContext {
    let data = match context {
        HostContext::Data(data) => data,
        HostContext::Host(port) => ContextData {
            folders: port.member("folders").await,
            request: port.member("request").await,
            signal: port.member("signal").await,
            extra_scope_folders: port.member("extraScopeFolders").await,
        },
    };
    let folders: Arc<dyn FolderScan> = Arc::new(data.folders);
    ScopeContext {
        folders,
        request: data.request,
        signal: data.signal,
        extra_scope_folders: data.extra_scope_folders,
    }
}

/// The host's `extraScopeFolders`, the one member `scopeFoldersOf` reads.
async fn extra_scope_folders_of(context: HostContext) -> Option<Vec<String>> {
    match context {
        HostContext::Data(data) => data.extra_scope_folders,
        HostContext::Host(port) => port.member("extraScopeFolders").await,
    }
}

/// The documented folder below every scope base.
pub const CLAUDE_FOLDER: &str = ".claude";

/// The folders read below each base of a scope, in precedence order: `.claude`, then the host's
/// extra folders as given, a repeat dropped. Judgment (spec/judgments.md): the docs name `.claude`
/// only; an extra folder of a scope ranks after the `.claude` folder of every base of that scope, so
/// a documented file never yields its name to one of an extra folder of its own scope, and the scopes
/// keep their documented order.
#[must_use]
pub fn scope_folders_of<R>(context: &ScopeContext<R>) -> Vec<String> {
    scope_folders_with(context.extra_scope_folders.as_deref())
}

/// `scope_folders_of` of a context whose extra folders are `extra`.
fn scope_folders_with(extra: Option<&[String]>) -> Vec<String> {
    let extra = extra.into_iter().flatten().cloned();
    std::iter::once(CLAUDE_FOLDER.to_owned())
        .chain(extra)
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect()
}

/// Where the base directories of a scope are; the user scope's are `ScopePlaceKind::User`'s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScopeBase {
    /// The managed directory.
    Managed,
    /// The working directory up to the repository root, and a worktree's main checkout.
    Project,
    /// The working directory up to the repository root, without the main checkout.
    Repository,
    /// The added directories.
    Added,
}

fn only(dir: Option<String>) -> Vec<String> {
    dir.into_iter().collect()
}

/// The base directories of each kind, in precedence order within it (`SCOPE_BASES[base]`).
pub fn scope_bases<'a, R: AsRef<ScopeRequest> + Sync>(
    base: ScopeBase,
    context: &'a ScopeContext<R>,
) -> BoxFuture<'a, Vec<String>> {
    let request = context.request.as_ref();
    let signal = context.signal.as_ref();
    Box::pin(async move {
        match base {
            ScopeBase::Managed => only(context.folders.managed_directory().await),
            // The working directory up to the repository root, and a worktree's main checkout
            // (`project_directories`).
            ScopeBase::Project => {
                project_directories(context.folders.as_ref(), &request.cwd, signal).await
            }
            // The working directory up to the repository root, nearest first, without the main
            // checkout.
            ScopeBase::Repository => {
                context
                    .folders
                    .repository(&request.cwd, signal)
                    .await
                    .directories
            }
            ScopeBase::Added => request.added_directories.clone().unwrap_or_default(),
        }
    })
}

/// One folder a scope reads its containers (`skills`, `agents`) below.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopePlace {
    /// The directory opened through the `FolderScan` port.
    pub base: String,
    /// The folder below the base: `.claude`, an extra scope folder, or `''` for the configuration
    /// directory itself.
    pub root: String,
    /// Its base's place among the bases of the scope, in precedence order.
    pub index: usize,
    /// The documented folder (`.claude`, or the configuration directory), not one of the host's
    /// extra ones.
    pub documented: bool,
}

/// Each root below each base: every base's `.claude` first, then each extra folder of every base
/// (`placesBelow(bases)(context, roots)`, with `bases(context)` the future given).
pub async fn places_below(
    bases: impl Future<Output = Vec<String>>,
    roots: &[String],
) -> Vec<ScopePlace> {
    let found = bases.await;
    roots
        .iter()
        .flat_map(|root| {
            found
                .iter()
                .enumerate()
                .map(move |(index, base)| ScopePlace {
                    base: base.clone(),
                    root: root.clone(),
                    index,
                    documented: root == CLAUDE_FOLDER,
                })
        })
        .collect()
}

/// The user scope: `.claude` is the configuration directory; an extra folder stays below the home.
fn user_places(request: &ScopeRequest, roots: &[String]) -> Vec<ScopePlace> {
    let config = config_folder_of(&request.home, request.config_dir.as_deref());
    roots
        .iter()
        .map(|root| {
            if root == CLAUDE_FOLDER {
                ScopePlace {
                    base: config.base.clone(),
                    root: config.folder.clone(),
                    index: 0,
                    documented: true,
                }
            } else {
                ScopePlace {
                    base: request.home.clone(),
                    root: root.clone(),
                    index: 0,
                    documented: false,
                }
            }
        })
        .collect()
}

/// A kind of scope base, or the user scope: the keys of `SCOPE_PLACES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScopePlaceKind {
    /// Below the managed directory.
    Managed,
    /// Below the project directories.
    Project,
    /// Below the repository directories.
    Repository,
    /// Below the added directories.
    Added,
    /// The user scope.
    User,
}

/// The places of each kind of scope base, and of the user scope (`SCOPE_PLACES[kind]`).
pub async fn scope_places<R: AsRef<ScopeRequest> + Sync>(
    kind: ScopePlaceKind,
    context: &ScopeContext<R>,
    roots: &[String],
) -> Vec<ScopePlace> {
    match kind {
        ScopePlaceKind::Managed => {
            places_below(scope_bases(ScopeBase::Managed, context), roots).await
        }
        ScopePlaceKind::Project => {
            places_below(scope_bases(ScopeBase::Project, context), roots).await
        }
        ScopePlaceKind::Repository => {
            places_below(scope_bases(ScopeBase::Repository, context), roots).await
        }
        ScopePlaceKind::Added => places_below(scope_bases(ScopeBase::Added, context), roots).await,
        ScopePlaceKind::User => user_places(context.request.as_ref(), roots),
    }
}

/// The path of a container below a place's root: `.claude/skills`, or `skills` in the
/// configuration directory.
#[must_use]
pub fn place_path(root: &str, container: &str) -> String {
    if root.is_empty() {
        container.to_owned()
    } else {
        format!("{root}/{container}")
    }
}

/// `SCOPE_BASES` as the host holds it: an object whose methods are the table's entries.
#[derive(Debug)]
struct ScopeBases;

crate::registry::methods! {
    ScopeBases {
        "managed" => async ok |_this, context: HostContext|
            scope_bases(ScopeBase::Managed, &scope_context_of(context).await).await;
        "project" => async ok |_this, context: HostContext|
            scope_bases(ScopeBase::Project, &scope_context_of(context).await).await;
        "repository" => async ok |_this, context: HostContext|
            scope_bases(ScopeBase::Repository, &scope_context_of(context).await).await;
        "added" => async ok |_this, context: HostContext|
            scope_bases(ScopeBase::Added, &scope_context_of(context).await).await;
    }
}

/// `SCOPE_PLACES` as the host holds it: an object whose methods are the table's entries.
#[derive(Debug)]
struct ScopePlaces;

/// One entry of `SCOPE_PLACES` called by the host.
async fn host_places(
    kind: ScopePlaceKind,
    context: HostContext,
    roots: Vec<String>,
) -> Vec<ScopePlace> {
    scope_places(kind, &scope_context_of(context).await, &roots).await
}

crate::registry::methods! {
    ScopePlaces {
        "managed" => async ok |_this, context: HostContext, roots: Vec<String>|
            host_places(ScopePlaceKind::Managed, context, roots).await;
        "project" => async ok |_this, context: HostContext, roots: Vec<String>|
            host_places(ScopePlaceKind::Project, context, roots).await;
        "repository" => async ok |_this, context: HostContext, roots: Vec<String>|
            host_places(ScopePlaceKind::Repository, context, roots).await;
        "added" => async ok |_this, context: HostContext, roots: Vec<String>|
            host_places(ScopePlaceKind::Added, context, roots).await;
        "user" => async ok |_this, context: HostContext, roots: Vec<String>|
            host_places(ScopePlaceKind::User, context, roots).await;
    }
}

/// The host's `bases` called with its own context, as it crossed, for `placesBelow`.
async fn host_bases(bases: Callback, context: JsValue) -> Vec<String> {
    bases.call(vec![context]).await
}

/// The part of a `ScopePlace` `placePath` reads.
#[derive(Deserialize)]
struct PlaceRoot {
    root: String,
}

crate::registry::exports! {
    "foundation/scope-folders" {
        "CLAUDE_FOLDER" => ok || CLAUDE_FOLDER;
        "scopeFoldersOf" => async ok |context: HostContext|
            scope_folders_with(extra_scope_folders_of(context).await.as_deref());
        "SCOPE_BASES" => ok || Handle::new(ScopeBases);
        "placesBelow" => async ok |bases: Callback, context: JsValue, roots: Vec<String>|
            places_below(host_bases(bases, context), &roots).await;
        "SCOPE_PLACES" => ok || Handle::new(ScopePlaces);
        "placePath" => ok |place: PlaceRoot, container: String| place_path(&place.root, &container);
        "readScopeFiles" => async ok |base: Port, container: String, options: ScopeFilesOptions|
            read_scope_files(&base, &container, &options).await;
        "watchedFolder" => async ok |base: Port, container: String, options: WatchOptions|
            watched_folder(&base, &container, &options).await;
    }
}
