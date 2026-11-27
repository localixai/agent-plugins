//! Port of `src/agents/catalog/scan.ts`, switched.
//!
//! Reads the agents folders through the `FolderScan` port (sub-agents.md, "Choose the subagent
//! scope": managed, project, user and `--add-dir` `.claude/agents/`, scanned "recursively, so you
//! can organize definitions into subfolders"). Nothing is resolved here: two files that declare one
//! name are both returned with where each came from, for `resolveAgents` to choose.

use serde::{Deserialize, Serialize};

use super::scopes::{AGENTS_FOLDER, AgentFolder, agent_scope_rule};
use super::types::{CatalogAgent, catalog_agent_of};
use crate::format::agent::read_agent;
use crate::format::agent_record::{
    AgentOrigin, AgentReadResult, AgentSource, FileOrigin, FileSource,
};
use crate::format::issues::PluginIssue;
use crate::foundation::scope_folders::{
    CLAUDE_FOLDER, ContextData, Depth, HostContext, ScopeContext, ScopeFilesOptions, ScopePlace,
    WatchOptions, place_path, read_scope_files, scope_context_of, scope_folders_of, scope_places,
    watched_folder,
};
use crate::js::{JsValue, promise_all};
use crate::ports::BoxFuture;
use crate::ports::folder_scan::ScanFolder;
use crate::registry::{Callback, Port};

/// What an agents scan reads with.
#[derive(Clone)]
pub struct AgentScanContext {
    /// The scope folders' context.
    pub scope: ScopeContext,
    /// Whether `strictPluginOnlyCustomization` lets a source load; absent, every source does.
    pub allows: Option<Callback>,
    /// False for a session started with `--disable-slash-commands`, which watches nothing. Default
    /// true.
    pub watch: Option<bool>,
}

/// An `AgentScanContext` as it crosses: a port when it holds `allows`, else data.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum HostAgentContext {
    /// A context that holds a function: its members are read one by one.
    Host(Port),
    /// A context of data and the host's `folders`.
    Data {
        /// The scope folders' members.
        #[serde(flatten)]
        context: ContextData,
        /// `watch`.
        #[serde(default)]
        watch: Option<bool>,
    },
}

/// The host's `AgentScanContext`, its members read when it crossed as a port.
pub async fn agent_context_of(context: HostAgentContext) -> AgentScanContext {
    match context {
        HostAgentContext::Host(port) => AgentScanContext {
            allows: port.member("allows").await,
            watch: port.member("watch").await,
            scope: scope_context_of(HostContext::Host(port)).await,
        },
        HostAgentContext::Data { context, watch } => AgentScanContext {
            scope: scope_context_of(HostContext::Data(context)).await,
            allows: None,
            watch,
        },
    }
}

/// One agent file of a scope folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedAgent {
    /// The agent.
    pub agent: CatalogAgent,
    /// The source of the file: `managed`, `project` or `user`.
    pub source: FileSource,
    /// Its folder's name for `strictPluginOnlyCustomization`; `addDir` for an added directory.
    pub origin: String,
    /// The base directory the file was read below.
    pub base: String,
    /// Where the file sits among the folders of its source: the nearer to the working directory,
    /// the lower. "The definition closest to the working directory" wins.
    pub order: u32,
    /// Where the file really lives; two entries with one target are one agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canonical: Option<String>,
}

/// What an agents scan found.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AgentScanOutcome {
    /// The agents, in scan order.
    pub entries: Vec<ScannedAgent>,
    /// The files that were skipped with a reason, and the fields a file's frontmatter got wrong.
    pub issues: Vec<PluginIssue>,
    /// The agents folders the session watches for changes: those that exist when it starts.
    pub watch: Vec<String>,
}

struct Where<'a> {
    context: &'a AgentScanContext,
    source: FileSource,
    folder: AgentFolder,
    base: String,
    order: u32,
    /// The agents folder below the base: `.claude/agents`, or `<extra>/agents`.
    container: String,
}

async fn read_files(place: &Where<'_>, files: &dyn ScanFolder) -> AgentScanOutcome {
    let options = ScopeFilesOptions {
        extension: ".md".to_owned(),
        depth: Depth::Tree,
        signal: place.context.scope.signal.clone(),
    };
    let read = read_scope_files(files, &place.container, &options).await;
    let mut outcome = AgentScanOutcome {
        issues: read.issues,
        ..AgentScanOutcome::default()
    };
    for file in read.files {
        let origin = AgentOrigin::File(FileOrigin {
            path: file.host_path.clone(),
            source: place.source,
        });
        match read_agent(&file.text, &origin) {
            AgentReadResult::Loaded { agent, issues } => {
                outcome.entries.push(ScannedAgent {
                    agent: catalog_agent_of(&agent, None),
                    source: place.source,
                    origin: place.folder.origin.to_owned(),
                    base: place.base.clone(),
                    order: place.order,
                    canonical: file.canonical,
                });
                outcome.issues.extend(issues);
            }
            AgentReadResult::Skipped { issues, .. } => outcome.issues.extend(issues),
        }
    }
    outcome
}

async fn scan_base(place: Where<'_>) -> AgentScanOutcome {
    let scope = &place.context.scope;
    let Some(files) = scope.folders.open(&place.base, scope.signal.as_ref()).await else {
        return AgentScanOutcome::default();
    };
    let read = read_files(&place, files.as_ref()).await;
    let options = WatchOptions {
        watched: place.folder.watched && place.context.watch != Some(false),
        signal: scope.signal.clone(),
    };
    let watch = watched_folder(files.as_ref(), &place.container, &options).await;
    AgentScanOutcome { watch, ..read }
}

fn merge(parts: Vec<AgentScanOutcome>) -> AgentScanOutcome {
    parts
        .into_iter()
        .fold(AgentScanOutcome::default(), |mut all, part| {
            all.entries.extend(part.entries);
            all.issues.extend(part.issues);
            all.watch.extend(part.watch);
            all
        })
}

const fn agent_source(source: FileSource) -> AgentSource {
    match source {
        FileSource::Managed => AgentSource::Managed,
        FileSource::Project => AgentSource::Project,
        FileSource::User => AgentSource::User,
    }
}

/// The folders of a source that may load, with their places below every scope folder.
async fn places_of(
    context: &AgentScanContext,
    source: FileSource,
    roots: &[String],
) -> Vec<(AgentFolder, Vec<ScopePlace>)> {
    let mut allowed = Vec::new();
    for folder in agent_scope_rule(agent_source(source)).folders {
        let allows = match context.allows {
            None => true,
            Some(allows) => allows.call(vec![JsValue::from(folder.origin)]).await,
        };
        if allows {
            allowed.push(*folder);
        }
    }
    let places = promise_all(
        allowed
            .iter()
            .map(|folder| -> BoxFuture<'_, _> {
                Box::pin(scope_places(folder.places, &context.scope, roots))
            })
            .collect(),
    )
    .await;
    allowed.into_iter().zip(places).collect()
}

/// The places below one scope folder: the documented one is `.claude`, whatever its root is.
fn below<'a>(places: &'a [ScopePlace], root: &'a str) -> impl Iterator<Item = &'a ScopePlace> {
    places.iter().filter(move |place| {
        if place.documented {
            root == CLAUDE_FOLDER
        } else {
            place.root == root
        }
    })
}

/// The folders of one source, base after base and folder after folder, below `.claude` and then
/// below each of the host's extra folders (`scopeFoldersOf`): `order` counts across them, so an
/// extra folder's agent loses a shared name to every `.claude` one of its source.
async fn scan_source(context: &AgentScanContext, source: FileSource) -> AgentScanOutcome {
    let roots = scope_folders_of(&context.scope);
    let folders = places_of(context, source, &roots).await;
    let mut parts = Vec::new();
    let mut order = 0;
    for root in &roots {
        for (folder, places) in &folders {
            for place in below(places, root) {
                let container = place_path(&place.root, AGENTS_FOLDER);
                let base = place.base.clone();
                let at = Where {
                    context,
                    source,
                    folder: *folder,
                    base,
                    order,
                    container,
                };
                parts.push(scan_base(at).await);
                order += 1;
            }
        }
    }
    merge(parts)
}

const FILE_SOURCES: [FileSource; 3] = [FileSource::Managed, FileSource::Project, FileSource::User];

/// Every agents folder of a session: managed, the project (and the added directories), personal.
pub async fn scan_agents(context: &AgentScanContext) -> AgentScanOutcome {
    let parts = promise_all(
        FILE_SOURCES
            .into_iter()
            .map(|source| -> BoxFuture<'_, _> { Box::pin(scan_source(context, source)) })
            .collect(),
    )
    .await;
    merge(parts)
}

crate::registry::exports! {
    "agents/catalog/scan" {
        "scanAgents" => async ok |context: HostAgentContext|
            scan_agents(&agent_context_of(context).await).await;
    }
}
