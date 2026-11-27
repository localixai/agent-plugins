//! Ported from `src/loading/resolve/shell-exports.ts`; only the core uses it now, so it has no wrapper.
//!
//! What a string a shell parses needs in its process environment for the references
//! `SubstitutionScope.shellPaths` left in it.

use indexmap::{IndexMap, IndexSet};
use serde::Deserialize;

use super::substitute::{ShellPaths, SubstitutionScope, TokenReference, name_of};
use super::substituter::VariableSource;
use super::surfaces::{ShellPathsScope, SubstitutionSurface, shell_paths_of};
use super::variables::PLUGIN_VARIABLES;
use crate::js::promise_all;
use crate::ports::BoxFuture;

/// What the exports read from the scope: its `shellPaths` and the paths another layer owns.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportScope {
    /// `SubstitutionScope.shellPaths`.
    #[serde(default)]
    pub shell_paths: Option<ShellPaths>,
    /// `SubstitutionScope.extra`.
    #[serde(default)]
    pub extra: Option<IndexMap<String, String>>,
    /// `SubstitutionScope.extraPaths`.
    #[serde(default)]
    pub extra_paths: Option<Vec<String>>,
}

impl<L, E> From<&SubstitutionScope<L, E>> for ExportScope {
    fn from(scope: &SubstitutionScope<L, E>) -> Self {
        Self {
            shell_paths: scope.shell_paths,
            extra: scope.extra.clone(),
            extra_paths: scope.extra_paths.clone(),
        }
    }
}

/// The `extra` paths a string references (`SubstitutionScope.extraPaths`), with their values.
fn extra_paths_of(referenced: &[TokenReference], scope: &ExportScope) -> Vec<(String, String)> {
    let paths = scope.extra_paths.as_deref().unwrap_or_default();
    referenced
        .iter()
        .filter_map(|token| {
            let name = name_of(token)?;
            if !paths.contains(&name) {
                return None;
            }
            let value = scope.extra.as_ref()?.get(&name)?.clone();
            Some((name, value))
        })
        .collect()
}

/// The path variables a string references, as the host reports them (native paths).
async fn referenced_paths(
    referenced: &[TokenReference],
    scope: &ExportScope,
    ask: &dyn VariableSource,
) -> IndexMap<String, String> {
    let names: IndexSet<_> = referenced
        .iter()
        .filter_map(name_of)
        .filter_map(|name| {
            PLUGIN_VARIABLES
                .into_iter()
                .find(|variable| variable.as_str() == name)
        })
        .collect();
    let asks: Vec<BoxFuture<'_, String>> = names.iter().map(|name| ask.variable(*name)).collect();
    let values = promise_all(asks).await;
    let mut exported: IndexMap<String, String> = names
        .iter()
        .zip(values)
        .map(|(name, value)| (name.as_str().to_owned(), value))
        .collect();
    exported.extend(extra_paths_of(referenced, scope));
    exported
}

/// What a string a shell parses needs in its process environment for the references
/// `SubstitutionScope.shellPaths` left in it: the path variables it references (and the `extra`
/// paths, `CLAUDE_SKILL_DIR`), nothing for pasted paths. The one rule for a monitor `command`, an
/// MCP `headersHelper` and a skill's `!` command, whose processes get no path variables of their
/// own (a hook's environment already carries all three). Pass the substituter's `variable` so the
/// data folder is asked for once.
pub async fn shell_exports(
    surface: SubstitutionSurface,
    referenced: &[TokenReference],
    scope: &ExportScope,
    ask: &dyn VariableSource,
) -> IndexMap<String, String> {
    let shell = ShellPathsScope {
        shell_paths: scope.shell_paths,
    };
    match shell_paths_of(shell, surface) {
        // A pasted path needs no environment; a reference needs the value it names.
        ShellPaths::Pasted => IndexMap::new(),
        ShellPaths::EnvironmentReference => referenced_paths(referenced, scope, ask).await,
    }
}
