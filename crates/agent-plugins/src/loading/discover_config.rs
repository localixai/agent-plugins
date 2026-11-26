//! Port of `src/loading/discover-config.ts`, switched.
//!
//! The sources of a plugin's JSON configuration (hooks, LSP servers, monitors), its `settings.json`
//! and its `bin/` (plugins/manifest-reference.md, "Standard layout").

use serde::{Deserialize, Serialize};

use super::combine::combine;
use super::ignored_folder::{self, ignored_folder_issues};
use crate::format::issues::PluginIssue;
use crate::format::layout::{CombineMode, ComponentKind, STANDARD_LAYOUT, declared_key};
use crate::format::listing::list_sorted;
use crate::format::locate::{LocateContext, locate, missing_at};
use crate::format::path_escape::LinkRoots;
use crate::format::path_text::{PathPlatform, child_path, root_relative};
use crate::js::{JsValue, promise_all};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// The kinds whose files are JSON configuration, read by their own reader.
pub const CONFIG_KINDS: [ConfigKind; 3] = [
    ConfigKind::Hooks,
    ConfigKind::LspServers,
    ConfigKind::Monitors,
];

/// One of `CONFIG_KINDS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigKind {
    /// `hooks`.
    Hooks,
    /// `lspServers`.
    LspServers,
    /// `monitors`.
    Monitors,
}

impl ConfigKind {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Hooks => "hooks",
            Self::LspServers => "lspServers",
            Self::Monitors => "monitors",
        }
    }

    const fn component(self) -> ComponentKind {
        match self {
            Self::Hooks => ComponentKind::Hooks,
            Self::LspServers => ComponentKind::LspServers,
            Self::Monitors => ComponentKind::Monitors,
        }
    }
}

/// What every discovery step needs (`DiscoverContext`, a `LocateContext` with the manifest).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverContext {
    /// The plugin folder.
    pub files: Port,
    /// Whose path rules apply.
    pub platform: PathPlatform,
    /// Where the plugin and its marketplace really are.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Cancels the reads.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// The manifest, when there is one.
    #[serde(default)]
    pub manifest: JsValue,
}

impl DiscoverContext {
    fn located(&self) -> LocateContext {
        LocateContext {
            files: self.files,
            platform: self.platform,
            roots: self.roots.clone(),
            signal: self.signal.clone(),
        }
    }

    fn ignored(&self) -> ignored_folder::DiscoverContext {
        ignored_folder::DiscoverContext {
            manifest: self.manifest.clone(),
            files: self.files,
            platform: self.platform,
            signal: self.signal.clone(),
        }
    }
}

/// What a manifest value for a config key lists, one item per source: `hooks` and `lspServers` take
/// "a `.json` file path, an inline … object, or an array mixing both"; `experimental.monitors` takes
/// "a `.json` file path or the inline array", so an array there is one inline source.
fn items_of(kind: ConfigKind, value: &JsValue) -> Vec<JsValue> {
    match (kind, value) {
        (ConfigKind::Hooks | ConfigKind::LspServers, JsValue::Array(items)) => items.clone(),
        _ => vec![value.clone()],
    }
}

/// A source: a file (as the plugin path and as written) or an inline value.
#[derive(Clone, Debug, PartialEq)]
enum Item {
    File { path: String, written: String },
    Inline(JsValue),
}

fn item_of(value: &JsValue, platform: PathPlatform) -> Item {
    value.as_str().map_or_else(
        || Item::Inline(value.clone()),
        |text| Item::File {
            path: root_relative(text, platform),
            written: text.to_owned(),
        },
    )
}

/// A file that is already a source loads once, so a manifest `hooks` path that names the default
/// `hooks/hooks.json` does not run its hooks twice (judgment: the docs merge what the manifest declares
/// with the default file and give no de-duplication rule).
fn without_repeated_files(order: Vec<(Item, bool)>) -> Vec<(Item, bool)> {
    let mut seen: Vec<String> = Vec::new();
    order
        .into_iter()
        .filter(|(item, _)| match item {
            Item::Inline(_) => true,
            Item::File { path, .. } => {
                let repeated = seen.contains(path);
                seen.push(path.clone());
                !repeated
            }
        })
        .collect()
}

/// A configuration source as discovery found it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ConfigInput {
    /// A file to read.
    File {
        /// The plugin path.
        file: String,
        /// As the manifest wrote it.
        written: String,
    },
    /// A value written in the manifest.
    Inline {
        /// The value.
        inline: JsValue,
    },
}

/// What a discovery step found, and what it has to say.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Found<T> {
    /// What was found.
    pub found: Vec<T>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

async fn resolve_item(
    kind: ConfigKind,
    item: Item,
    declared: bool,
    context: &LocateContext,
) -> Found<ConfigInput> {
    let (path, written) = match item {
        Item::Inline(inline) => {
            return Found {
                found: vec![ConfigInput::Inline { inline }],
                issues: Vec::new(),
            };
        }
        Item::File { path, written } => (path, written),
    };
    let located = locate(context, kind.as_str(), &written, Some(!declared)).await;
    if let Some(issue) = located.issue {
        return Found {
            found: Vec::new(),
            issues: vec![issue],
        };
    }
    if located.stat.is_none() {
        let missing = if declared {
            vec![missing_at(context, kind.as_str(), &written)]
        } else {
            Vec::new()
        };
        return Found {
            found: Vec::new(),
            issues: missing,
        };
    }
    Found {
        found: vec![ConfigInput::File {
            file: path,
            written,
        }],
        issues: Vec::new(),
    }
}

/// The sources of hooks, LSP servers or monitors in loading order: the default file and what the
/// manifest declares, combined by the kind's rule (`hooks` and `lspServers` merge, the default
/// first; `monitors` is replaced by `experimental.monitors`, the top-level `monitors` alias used
/// only when it is absent: judgment). A default file that is not there is skipped. An item keeps
/// whether the manifest declared it, where the TypeScript finds it again among the declared items
/// by identity.
pub async fn discover_config(kind: ConfigKind, context: &DiscoverContext) -> Found<ConfigInput> {
    let entry = STANDARD_LAYOUT.of(kind.component());
    let items: Option<Vec<(Item, bool)>> =
        declared_key(&context.manifest, kind.component()).map(|declared| {
            items_of(kind, &declared.value)
                .iter()
                .map(|value| (item_of(value, context.platform), true))
                .collect()
        });
    let written: Vec<String> = items
        .iter()
        .flatten()
        .filter_map(|(item, _)| match item {
            Item::File { written, .. } => Some(written.clone()),
            Item::Inline(_) => None,
        })
        .collect();
    let warnings = ignored_folder_issues(kind.component(), &written, &context.ignored()).await;
    let defaults = [(
        Item::File {
            path: entry.location.to_owned(),
            written: entry.location.to_owned(),
        },
        false,
    )];
    let mode = entry.combine.unwrap_or(CombineMode::Replace);
    let order = without_repeated_files(combine(mode, &defaults, items.as_deref()));
    let located = context.located();
    let resolving: Vec<BoxFuture<'_, Found<ConfigInput>>> = order
        .into_iter()
        .map(|(item, declared)| -> BoxFuture<'_, Found<ConfigInput>> {
            Box::pin(resolve_item(kind, item, declared, &located))
        })
        .collect();
    let resolved = promise_all(resolving).await;
    let mut issues = warnings;
    let mut found = Vec::new();
    for one in resolved {
        found.extend(one.found);
        issues.extend(one.issues);
    }
    Found { found, issues }
}

/// The `settings.json` at the plugin root, when it is there and inside the plugin.
pub async fn discover_settings(context: &DiscoverContext) -> Found<String> {
    let location = STANDARD_LAYOUT.of(ComponentKind::Settings).location;
    let located = locate(&context.located(), "settings", location, Some(true)).await;
    if let Some(issue) = located.issue {
        return Found {
            found: Vec::new(),
            issues: vec![issue],
        };
    }
    let found = if located.stat.is_some_and(|stat| stat.is_file) {
        vec![location.to_owned()]
    } else {
        Vec::new()
    };
    Found {
        found,
        issues: Vec::new(),
    }
}

/// `bin/`: whether it exists, and the files directly in it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DiscoveredBin {
    /// A plugin "that has this directory".
    pub present: bool,
    /// The files directly in it.
    pub files: Vec<String>,
}

/// `bin/`: whether it exists (a plugin "that has this directory") and the files directly in it.
pub async fn discover_bin(context: &DiscoverContext) -> DiscoveredBin {
    let location = STANDARD_LAYOUT.of(ComponentKind::Bin).location;
    let located = locate(&context.located(), "bin", location, None).await;
    if !located.stat.is_some_and(|stat| stat.is_directory) {
        return DiscoveredBin {
            present: false,
            files: Vec::new(),
        };
    }
    let entries = list_sorted(&context.files, location, context.signal.as_ref())
        .await
        .unwrap_or_default();
    DiscoveredBin {
        present: true,
        files: entries
            .iter()
            .filter(|one| one.is_file)
            .map(|one| child_path(location, &one.name))
            .collect(),
    }
}

crate::registry::exports! {
    "loading/discover-config" {
        "CONFIG_KINDS" => ok || CONFIG_KINDS;
        "discoverConfig" => async ok |kind: ConfigKind, context: DiscoverContext|
            discover_config(kind, &context).await;
        "discoverSettings" => async ok |context: DiscoverContext| discover_settings(&context).await;
        "discoverBin" => async ok |context: DiscoverContext| discover_bin(&context).await;
    }
}
