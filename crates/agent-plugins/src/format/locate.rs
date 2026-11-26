//! Port of `src/format/locate.ts`, switched.
//!
//! Containment and existence of a plugin's paths, and the context every loader of plugin paths
//! shares.

use serde::{Deserialize, Serialize};

use super::host_errors::path_could_not_be_checked;
use super::issues::{PluginIssue, error};
use super::messages::missing_path;
use super::path_escape::{DeclaredPath, LinkRoots, escaped_path_issue};
use super::path_text::{PathPlatform, root_relative, without_trailing_slashes};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, promise_all};
use crate::ports::files::{FileStat, PluginFiles};
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;

/// What a path turned out to be: what it is, or the issue that stops it loading; neither when it is missing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Located {
    /// What it is.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stat: Option<FileStat>,
    /// What stops it loading.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issue: Option<PluginIssue>,
}

/// What `locate` needs: the folder, and what decides whether a path leaves it.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateContext {
    /// The plugin folder.
    pub files: Port,
    /// The backslash rule is the platform's: "on macOS and Linux" a backslash escapes.
    pub platform: PathPlatform,
    /// Where the plugin and its marketplace really are, so links between plugins of one marketplace load.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// Its cancellation.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// What a caller may leave out of a `LocateContext`.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocateOptions {
    /// The platform.
    #[serde(default)]
    pub platform: Option<PathPlatform>,
    /// The roots.
    #[serde(default)]
    pub roots: Option<LinkRoots>,
    /// The cancellation.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// The context every loader of plugin paths shares. `platform` defaults to `posix`, the stricter
/// one: a backslash in a declared path escapes there.
#[must_use]
pub fn locate_context(files: Port, options: LocateOptions) -> LocateContext {
    LocateContext {
        files,
        platform: options.platform.unwrap_or(PathPlatform::Posix),
        roots: options.roots,
        signal: options.signal,
    }
}

impl LocateContext {
    /// The context as the host holds it: `{ files, platform, roots?, signal? }`.
    #[must_use]
    pub fn to_js(&self) -> JsValue {
        let mut context = JsObject::new();
        context.insert(
            "files".to_owned(),
            to_js_value(&self.files).unwrap_or_default(),
        );
        context.insert(
            "platform".to_owned(),
            match self.platform {
                PathPlatform::Posix => "posix",
                PathPlatform::Windows => "windows",
            }
            .into(),
        );
        if let Some(roots) = &self.roots {
            let mut members = JsObject::new();
            members.insert("pluginRoot".to_owned(), roots.plugin_root.as_str().into());
            if let Some(marketplace) = &roots.marketplace_root {
                members.insert("marketplaceRoot".to_owned(), marketplace.as_str().into());
            }
            context.insert("roots".to_owned(), JsValue::Object(members));
        }
        if let Some(signal) = &self.signal {
            context.insert("signal".to_owned(), to_js_value(signal).unwrap_or_default());
        }
        JsValue::Object(context)
    }
}

/// The folders above a root-relative path, nearest the root first: `a/b/c` has `a` and `a/b`.
fn ancestors_of(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let above = parts
        .get(..parts.len().saturating_sub(1))
        .unwrap_or_default();
    (0..above.len())
        .map(|index| above.get(..=index).unwrap_or_default().join("/"))
        .collect()
}

/// Where the path leads through a folder that is a link: its real path and the segments below it.
fn through(folder: Option<&FileStat>, below: &[&str]) -> Option<String> {
    let folder = folder.filter(|folder| folder.is_symlink)?;
    let real_path = folder.real_path.as_deref()?;
    let mut parts = vec![without_trailing_slashes(real_path)];
    parts.extend_from_slice(below);
    Some(parts.join("/"))
}

fn link_issue(
    context: &LocateContext,
    declared: &DeclaredPath,
    resolves_to: &str,
) -> Option<PluginIssue> {
    escaped_path_issue(&DeclaredPath {
        resolves_to: Some(resolves_to.to_owned()),
        roots: context.roots.clone(),
        ..declared.clone()
    })
}

/// Containment and existence (plugins/manifest-reference.md, "Containment and existence"; loading.md,
/// "Paths that escape the plugin directory"): a path with `..`, an absolute path, a backslash on
/// macOS and Linux, or a symlink that leads outside the plugin (other than to another plugin of
/// its marketplace) "doesn't load" and is reported as `<component> path escapes plugin directory:
/// <path>` (`escapedPathIssue`); a path that is not there is `undefined` and the caller decides
/// whether it was declared. A path the system could not check (`FileStat.checkFailed`, on it or a
/// folder above it) does not load either: `<component> path could not be checked: <path> (<code>)`
/// (errors.md, "Path could not be checked"), for a default location as for a declared one, the
/// plugin's other components loading. `written` is the spelling the message quotes. A link is seen on the
/// path itself and on every folder above it: each ancestor is stat'd once per call, since a port
/// reports a link only on the entry it is asked about (`FileStat.realPath`). Every reader of a
/// declared or standard plugin path decides containment here, and nowhere else.
pub async fn locate(
    context: &LocateContext,
    component: &str,
    written: &str,
    own: Option<bool>,
) -> Located {
    let declared = DeclaredPath {
        component: component.to_owned(),
        written: written.to_owned(),
        platform: context.platform,
        resolves_to: None,
        roots: None,
        own: Some(own == Some(true)),
    };
    if let Some(as_written) = escaped_path_issue(&declared) {
        return Located {
            stat: None,
            issue: Some(as_written),
        };
    }
    let path = root_relative(written, context.platform);
    let mut asked = vec![path.clone()];
    asked.extend(ancestors_of(&path));
    let files: &dyn PluginFiles = &context.files;
    let signal = context.signal.as_ref();
    let found: Vec<Option<FileStat>> = promise_all(
        asked
            .iter()
            .map(|one| -> BoxFuture<'_, Option<FileStat>> { files.stat(one, signal) })
            .collect(),
    )
    .await;
    let mut found = found.into_iter();
    let stat = found.next().flatten();
    let folders: Vec<Option<FileStat>> = found.collect();
    let failed = folders
        .iter()
        .chain(std::iter::once(&stat))
        .find_map(|one| one.as_ref().and_then(|one| one.check_failed.as_deref()));
    if let Some(failed) = failed {
        return Located {
            stat: None,
            issue: Some(error(
                component,
                &path_could_not_be_checked(component, written, failed),
            )),
        };
    }
    let parts: Vec<&str> = path.split('/').collect();
    let mut resolved: Vec<String> = folders
        .iter()
        .enumerate()
        .filter_map(|(index, folder)| {
            through(folder.as_ref(), parts.get(index + 1..).unwrap_or_default())
        })
        .collect();
    let leaf = stat
        .as_ref()
        .filter(|stat| stat.is_symlink)
        .and_then(|stat| stat.real_path.clone());
    resolved.extend(leaf);
    let linked = resolved
        .iter()
        .find_map(|resolves_to| link_issue(context, &declared, resolves_to));
    if linked.is_some() {
        return Located {
            stat: None,
            issue: linked,
        };
    }
    match stat {
        Some(stat) if stat.is_file || stat.is_directory => Located {
            stat: Some(stat),
            issue: None,
        },
        Some(_) | None => Located::default(),
    }
}

/// The load error of a declared path that is not there, `<component> path not found: <absolute
/// path>` (spec §2.5 item 3): the plugin root joined with the path when the host gave `roots`, the
/// path as written when it did not (the loader then knows no host path).
#[must_use]
pub fn missing_at(context: &LocateContext, component: &str, written: &str) -> PluginIssue {
    let Some(roots) = &context.roots else {
        return missing_path(component, written);
    };
    let relative = root_relative(written, context.platform);
    let root = without_trailing_slashes(&roots.plugin_root);
    missing_path(
        component,
        &if relative == "." {
            root.to_owned()
        } else {
            format!("{root}/{relative}")
        },
    )
}

crate::registry::exports! {
    "format/locate" {
        "locateContext" => ok |files: Port, options: Option<LocateOptions>| locate_context(files, options.unwrap_or_default()).to_js();
        "locate" => async ok |context: LocateContext, component: String, written: String, own: Option<bool>| locate(&context, &component, &written, own).await;
        "missingAt" => ok |context: LocateContext, component: String, written: String| missing_at(&context, &component, &written);
    }
}
