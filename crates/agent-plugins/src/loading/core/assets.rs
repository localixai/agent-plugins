//! Port of `src/loading/core/assets.ts`, switched.
//!
//! The package's files that are data, not components: icon candidates under `assets/` (core §4.3.1)
//! and the extension directories of the namespaces the host implements (core §8.2). A candidate or a
//! directory that resolves outside the plugin root is left out, its access denied (core §4.1,
//! boundary 6); neither ever invalidates the package. `assets/` is never an extension directory (§8).

use crate::format::core::icon::icon_candidates;
use crate::format::listing::list_sorted;
use crate::format::locate::{LocateContext, locate};
use crate::js::promise_all;
use crate::ports::BoxFuture;

const ASSETS: &str = "assets";

/// The usable-looking icon candidates, PNG first.
pub async fn core_icons(context: &LocateContext, formats: &[String]) -> Vec<String> {
    let folder = locate(context, ASSETS, ASSETS, Some(true)).await;
    if folder.issue.is_some() || !folder.stat.is_some_and(|stat| stat.is_directory) {
        return Vec::new();
    }
    let entries = list_sorted(&context.files, ASSETS, context.signal.as_ref())
        .await
        .unwrap_or_default();
    let candidates = icon_candidates(&entries, formats);
    let checks: Vec<BoxFuture<'_, Option<String>>> = candidates
        .into_iter()
        .map(|path| -> BoxFuture<'_, Option<String>> {
            Box::pin(async move {
                let found = locate(context, ASSETS, &path, Some(true)).await;
                found.stat.is_some_and(|stat| stat.is_file).then_some(path)
            })
        })
        .collect();
    promise_all(checks).await.into_iter().flatten().collect()
}

/// The top-level directories named exactly for an implemented namespace, in name order.
pub async fn core_extension_directories(
    context: &LocateContext,
    namespaces: &[String],
) -> Vec<String> {
    let wanted: Vec<&String> = namespaces.iter().filter(|name| *name != ASSETS).collect();
    if wanted.is_empty() {
        return Vec::new();
    }
    let entries = list_sorted(&context.files, ".", context.signal.as_ref())
        .await
        .unwrap_or_default();
    let checks: Vec<BoxFuture<'_, Option<String>>> = entries
        .into_iter()
        .filter(|entry| wanted.contains(&&entry.name))
        .map(|entry| -> BoxFuture<'_, Option<String>> {
            Box::pin(async move {
                let at = locate(context, &entry.name, &entry.name, Some(true)).await;
                at.stat
                    .is_some_and(|stat| stat.is_directory)
                    .then_some(entry.name)
            })
        })
        .collect();
    promise_all(checks).await.into_iter().flatten().collect()
}

crate::registry::exports! {
    "loading/core/assets" {
        "coreIcons" => async ok |context: LocateContext, formats: Vec<String>|
            core_icons(&context, &formats).await;
        "coreExtensionDirectories" => async ok |context: LocateContext, namespaces: Vec<String>|
            core_extension_directories(&context, &namespaces).await;
    }
}
