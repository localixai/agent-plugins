//! The core's counterpart of `src/loading/resolve/field-runner.ts` at sha256:2e27e43aee6e, which stays TypeScript: its
//! `fieldRunner` returns closures over the host's substituter, and its `when` calls a host
//! function whose answer may be a promise, which no synchronous export can take; it only looks at
//! the outside of the value (`undefined` or not). The core's resolvers use this copy.
//!
//! Substitutes the strings of one component config, each with the policy of the surface it
//! belongs to. One substituter serves all fields, so the data folder is asked for once.

use indexmap::{IndexMap, IndexSet};

use super::substitute::{TokenReference, Unresolved};
use super::substituter::Substituter;
use super::surfaces::{ShellPathsScope, SubstitutionSurface, surface_policy};
use crate::format::issues::PluginIssue;
use crate::js::promise_all;
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// A substituted value with the issues its references raised.
#[derive(Clone, Debug)]
pub struct Part<T> {
    /// The value.
    pub value: T,
    /// The issues.
    pub issues: Vec<PluginIssue>,
    /// What stayed as written (`Substituted.unresolved`).
    pub unresolved: Unresolved,
    /// What the strings referenced (`Substituted.referenced`), in order.
    pub referenced: Vec<TokenReference>,
}

/// What several fields left unresolved, each name once.
#[must_use]
pub fn merge_unresolved<'a>(all: impl Iterator<Item = &'a Unresolved> + Clone) -> Unresolved {
    let variables: IndexSet<String> = all
        .clone()
        .flat_map(|each| each.variables.clone())
        .collect();
    let options: IndexSet<String> = all.flat_map(|each| each.options.clone()).collect();
    Unresolved {
        variables: variables.into_iter().collect(),
        options: options.into_iter().collect(),
    }
}

/// `fieldRunner(run, base)`: `one`, `list` and `table`.
pub struct FieldRunner<'r, 's, L, E> {
    run: &'r Substituter<'s, L, E>,
    base: String,
}

/// Substitutes the strings of one component config, each with the policy of the surface it belongs
/// to. One substituter serves all fields, so the data folder is asked for once.
pub fn field_runner<'r, 's, L, E>(
    run: &'r Substituter<'s, L, E>,
    base: &str,
) -> FieldRunner<'r, 's, L, E> {
    FieldRunner {
        run,
        base: base.to_owned(),
    }
}

impl<L: Locations + Clone, E: Environment> FieldRunner<'_, '_, L, E> {
    /// One string.
    pub async fn one(
        &self,
        surface: SubstitutionSurface,
        template: &str,
        field: &str,
    ) -> Part<String> {
        let shell = ShellPathsScope {
            shell_paths: self.run.scope().shell_paths,
        };
        let path = format!("{}.{field}", self.base);
        let done = self
            .run
            .run(template, &surface_policy(surface, shell), &path)
            .await;
        Part {
            value: done.text,
            issues: done.issues,
            unresolved: done.unresolved,
            referenced: done.referenced,
        }
    }

    /// A list of strings.
    pub async fn list(
        &self,
        surface: SubstitutionSurface,
        templates: &[String],
        field: &str,
    ) -> Part<Vec<String>> {
        // Each element is named by its index, as a hook's `args[i]`.
        let fields: Vec<String> = (0..templates.len())
            .map(|index| format!("{field}[{index}]"))
            .collect();
        let asks = templates
            .iter()
            .zip(&fields)
            .map(|(each, field)| -> BoxFuture<'_, _> { Box::pin(self.one(surface, each, field)) })
            .collect();
        let parts = promise_all(asks).await;
        Part {
            value: parts.iter().map(|part| part.value.clone()).collect(),
            issues: parts.iter().flat_map(|part| part.issues.clone()).collect(),
            unresolved: merge_unresolved(parts.iter().map(|part| &part.unresolved)),
            referenced: parts
                .iter()
                .flat_map(|part| part.referenced.clone())
                .collect(),
        }
    }

    /// A table of strings.
    pub async fn table(
        &self,
        surface: SubstitutionSurface,
        entries: &IndexMap<String, String>,
        field: &str,
    ) -> Part<IndexMap<String, String>> {
        let fields: Vec<String> = entries.keys().map(|key| format!("{field}.{key}")).collect();
        let asks = entries
            .values()
            .zip(&fields)
            .map(|(each, field)| -> BoxFuture<'_, _> { Box::pin(self.one(surface, each, field)) })
            .collect();
        let parts = promise_all(asks).await;
        Part {
            value: entries
                .keys()
                .cloned()
                .zip(parts.iter().map(|part| part.value.clone()))
                .collect(),
            issues: parts.iter().flat_map(|part| part.issues.clone()).collect(),
            unresolved: merge_unresolved(parts.iter().map(|part| &part.unresolved)),
            referenced: parts
                .iter()
                .flat_map(|part| part.referenced.clone())
                .collect(),
        }
    }
}
