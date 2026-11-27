//! Ported from `src/loading/resolve/prepared.ts`; only the core uses it now, so it has no wrapper.
//!
//! A substituter for a host that substitutes synchronously (a lazily read Markdown body): the
//! three path variables are asked for once, here, and every `run` after is synchronous. Judgment
//! (spec/judgments.md): preparing asks for the data folder, so the folder is created when the host
//! prepares, not at the first reference.

use super::substitute::{Answers, FieldPolicy, Substituted, SubstitutionScope, run_with};
use super::variables::{PLUGIN_VARIABLES, plugin_variable_value};
use crate::js::promise_all;
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// `PreparedSubstituter`: every variable is known.
pub struct PreparedSubstituter<L, E> {
    scope: SubstitutionScope<L, E>,
    answers: Answers,
}

impl<L, E: Environment> PreparedSubstituter<L, E> {
    /// The same substitution as `Substituter.run`, without a port: every variable is known.
    pub async fn run(&self, template: &str, policy: &FieldPolicy, path: &str) -> Substituted {
        run_with(template, policy, path, (&self.scope, &self.answers)).await
    }
}

/// A substituter whose three path variables are asked for now.
pub async fn prepare_substituter<L: Locations + Clone, E>(
    scope: SubstitutionScope<L, E>,
) -> PreparedSubstituter<L, E> {
    let variables = scope.variable_scope();
    let asks = PLUGIN_VARIABLES
        .into_iter()
        .map(|name| -> BoxFuture<'_, _> {
            let variables = &variables;
            Box::pin(async move { (name, plugin_variable_value(name, variables).await) })
        })
        .collect();
    let answers = promise_all(asks).await.into_iter().collect();
    PreparedSubstituter { scope, answers }
}
