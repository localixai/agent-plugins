//! Ported from `src/hooks/output/watch-paths.ts`; only the core uses it now, so it has no wrapper.
//!
//! `watchPaths` of `SessionStart`, `CwdChanged` and `FileChanged` asks for absolute paths (hooks.md,
//! "CwdChanged decision control"); a relative entry is resolved against the hook input's `cwd`, `.`
//! and `..` collapsed, and each path is kept once (Agent Plugins Extended §7.5.28, Judgment J7.131).
//! `CwdChanged`'s `cwd` is the new directory, so its paths are read where the session now is.
//!
//! The outcome is the host's whole value, so every member it does not change goes back as given.

use indexmap::IndexSet;

use super::worktree_path::{is_absolute, resolve_relative};
use crate::js::{JsObject, JsValue};

/// The texts of `effects.watchPaths`, when the outcome has the array.
fn given_paths(outcome: &JsObject) -> Option<Vec<String>> {
    let given = outcome.get("effects")?.get("watchPaths")?.as_array()?;
    given
        .iter()
        .map(|path| path.as_str().map(str::to_owned))
        .collect()
}

/// The outcome with each relative `watchPaths` entry resolved against `cwd`, each path once.
#[must_use]
pub fn with_resolved_watch_paths(mut outcome: JsObject, cwd: &str) -> JsObject {
    let Some(given) = given_paths(&outcome) else {
        return outcome;
    };
    if given.iter().all(|path| is_absolute(path)) {
        return outcome;
    }
    let watch_paths: IndexSet<String> = given
        .iter()
        .map(|path| {
            if is_absolute(path) {
                path.clone()
            } else {
                resolve_relative(cwd, path)
            }
        })
        .collect();
    if let Some(JsValue::Object(effects)) = outcome.get_mut("effects") {
        effects.insert(
            "watchPaths".to_owned(),
            JsValue::Array(watch_paths.into_iter().map(JsValue::String).collect()),
        );
    }
    outcome
}
#[cfg(test)]
mod tests {
    use super::*;

    fn outcome(paths: &[&str]) -> JsObject {
        let mut effects = JsObject::new();
        effects.insert(
            "watchPaths".to_owned(),
            JsValue::Array(paths.iter().map(|path| (*path).into()).collect()),
        );
        let mut outcome = JsObject::new();
        outcome.insert("effects".to_owned(), JsValue::Object(effects));
        outcome
    }

    #[test]
    fn resolves_relative_paths_once_each() {
        let resolved = with_resolved_watch_paths(outcome(&["a", "/r/a", "./a"]), "/r");
        assert_eq!(resolved, outcome(&["/r/a"]));
    }
}
