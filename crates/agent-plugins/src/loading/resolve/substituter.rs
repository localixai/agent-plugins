//! The core's counterpart of `src/loading/resolve/substituter.ts` at sha256:94a818bf41aa, which stays TypeScript: its
//! substituter hands back the host's own scope (`run.scope`, read by its callers) and keeps the
//! host's promises of the path variables across separate calls, neither of which crosses.
//!
//! A substituter keeps what the host answered for the variables, so several strings of one
//! component (a server's `command`, `args` and `env`) ask for the data folder once, and only a
//! string that writes a variable's path asks for it.

use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::sync::{Mutex, PoisonError};
use std::task::{Context, Poll};

use indexmap::IndexMap;

use super::substitute::{
    Answers, FieldPolicy, Substituted, SubstitutionScope, needed_variables, run_with,
};
use super::variables::{PluginVariable, plugin_variable_value};
use crate::js::promise_all;
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::registry::{Port, Thrown, catch_host};

/// One variable's ask: the host's answer while it is awaited, then the answer.
enum Asking<'a> {
    Pending(Pin<Box<dyn Future<Output = Result<String, Thrown>> + Send + 'a>>),
    Done(String),
}

/// A substituter of one scope (`Substituter`).
pub struct Substituter<'a, L, E> {
    scope: &'a SubstitutionScope<L, E>,
    answers: Mutex<Answers>,
    asked: Mutex<IndexMap<PluginVariable, Asking<'a>>>,
}

/// A substituter of `scope`.
pub fn substituter<L, E>(scope: &SubstitutionScope<L, E>) -> Substituter<'_, L, E> {
    Substituter {
        scope,
        answers: Mutex::new(Answers::new()),
        asked: Mutex::new(IndexMap::new()),
    }
}

impl<'a, L, E> Substituter<'a, L, E> {
    /// What the references resolve against.
    pub const fn scope(&self) -> &'a SubstitutionScope<L, E> {
        self.scope
    }
}

impl<'a, L: Locations + Clone + 'a, E: Environment> Substituter<'a, L, E> {
    /// The variable's path as the host reported it, asked once per substituter
    /// (`exportedEnvironment`). Every string awaiting it polls the one ask, so it is asked once
    /// even while several strings wait on it together.
    pub async fn variable(&self, name: PluginVariable) -> String {
        match poll_fn(|context| self.poll_variable(name, context)).await {
            Ok(text) => text,
            Err(cause) => cause.rethrow().await,
        }
    }

    fn poll_variable(
        &self,
        name: PluginVariable,
        context: &mut Context<'_>,
    ) -> Poll<Result<String, Thrown>> {
        let mut asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        let slot = asked.entry(name).or_insert_with(|| {
            let scope = self.scope.variable_scope();
            Asking::Pending(Box::pin(catch_host(async move {
                plugin_variable_value(name, &scope).await
            })))
        });
        let polled = match slot {
            Asking::Done(text) => return Poll::Ready(Ok(text.clone())),
            Asking::Pending(asking) => asking.as_mut().poll(context),
        };
        match polled {
            Poll::Ready(Ok(text)) => {
                *slot = Asking::Done(text.clone());
                Poll::Ready(Ok(text))
            }
            // A host that failed is asked again by the next string, not answered with its old failure.
            Poll::Ready(Err(cause)) => {
                asked.shift_remove(&name);
                Poll::Ready(Err(cause))
            }
            Poll::Pending => Poll::Pending,
        }
    }

    async fn ask(&self, name: PluginVariable) {
        let value = self.variable(name).await;
        self.answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(name, value);
    }

    /// `path` names the field in the issues.
    pub async fn run(&self, template: &str, policy: &FieldPolicy, path: &str) -> Substituted {
        let asks = needed_variables(template, self.scope, policy)
            .into_iter()
            .map(|name| -> BoxFuture<'_, ()> { Box::pin(self.ask(name)) })
            .collect();
        promise_all(asks).await;
        let answers = self
            .answers
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        run_with(template, policy, path, (self.scope, &answers)).await
    }
}

/// `substituter(scope).run(template, policy, path)`.
pub async fn substitute_template<L: Locations + Clone, E: Environment>(
    template: &str,
    scope: &SubstitutionScope<L, E>,
    policy: &FieldPolicy,
    path: &str,
) -> Substituted {
    substituter(scope).run(template, policy, path).await
}

/// `Pick<Substituter, 'variable'>`: where a path variable's value comes from, a substituter of
/// the core's or the host's.
pub trait VariableSource: Send + Sync {
    /// The variable's path as the host reported it.
    fn variable(&self, name: PluginVariable) -> BoxFuture<'_, String>;
}

impl<'a, L: Locations + Clone + 'a, E: Environment> VariableSource for Substituter<'a, L, E> {
    fn variable(&self, name: PluginVariable) -> BoxFuture<'_, String> {
        Box::pin(Self::variable(self, name))
    }
}

impl VariableSource for Port {
    fn variable(&self, name: PluginVariable) -> BoxFuture<'_, String> {
        Box::pin(self.method("variable", vec![name.as_str().into()]))
    }
}

/// `(name) => Promise<string>`, the host's function (`Substituter.variable` passed alone).
impl VariableSource for crate::registry::Callback {
    fn variable(&self, name: PluginVariable) -> BoxFuture<'_, String> {
        Box::pin(self.call(vec![name.as_str().into()]))
    }
}
