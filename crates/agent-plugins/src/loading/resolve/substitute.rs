//! Port of `src/loading/resolve/substitute.ts`, switched.
//!
//! Substitution of `${...}` in one string. One pass over the original text: a substituted value is
//! never scanned again (skills.md, "Available string substitutions"), so a saved option holding
//! `${CLAUDE_PLUGIN_ROOT}` stays that text.
//!
//! Grammar (judgment: the docs name the forms, not the edges): a reference is `${` name `}` with no
//! braces inside; a `${VAR:-default}` default ends at the first `}` and nests nothing; there is no
//! escape (a backslash does not prevent a `${CLAUDE_*}` variable, skills.md); anything that is not
//! a known reference is left as written.
//!
//! The scope crosses as data with its `locations` and `environment` as ports; a policy holding
//! the host's `readsAsEmpty` crosses as a port and is read member by member ([`PolicyArg`]).

pub mod scope;
mod tokens;

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};

pub use scope::{
    EnvironmentExpansion, FieldPolicy, PolicyArg, ReadsAsEmptyRule, ShellPaths, SubstitutionScope,
    UserConfigPolicy,
};

pub use tokens::sensitive_placeholder;
use tokens::{Context, Resolution, tokens_of};

use super::variables::{PluginVariable, rendering_needs_value};
use crate::format::issues::{PluginIssue, warning};
use crate::format::mcp_env::braced_references;
use crate::ports::environment::Environment;
use crate::registry::Map;

/// What a template referenced, in order of first appearance, once each.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum TokenReference {
    /// One of the three path variables.
    PathVariable {
        /// The variable.
        name: PluginVariable,
    },
    /// `${user_config.KEY}`.
    UserConfig {
        /// The option.
        key: String,
    },
    /// `${VAR}` of the environment.
    Environment {
        /// The variable.
        name: String,
    },
    /// A variable of `SubstitutionScope.extra`.
    Extra {
        /// The variable.
        name: String,
    },
}

/// A template substituted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Substituted {
    /// The text.
    pub text: String,
    /// Warnings only: an unresolved reference stays as written and is reported.
    pub issues: Vec<PluginIssue>,
    /// What the template referenced.
    pub referenced: Vec<TokenReference>,
    /// What stayed as written because nothing answered it: the names of `${VAR}` references with
    /// no value and no default, and the keys of `${user_config.KEY}` references with no value or
    /// no declaration. A caller that treats them as an error (an MCP server that cannot start)
    /// reads them here instead of from the warning text.
    pub unresolved: Unresolved,
}

/// What stayed as written (`Substituted.unresolved`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unresolved {
    /// `${VAR}` names.
    pub variables: Vec<String>,
    /// `${user_config.KEY}` keys.
    pub options: Vec<String>,
}

/// What the host answered for the path variables.
pub type Answers = IndexMap<PluginVariable, String>;

fn unique_of(values: impl Iterator<Item = Option<String>>) -> Vec<String> {
    values
        .flatten()
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect()
}

/// The substitution once the variables the template needs are known: no port is asked here but
/// the environment, for `${VAR}`.
pub async fn run_with<L, E: Environment>(
    template: &str,
    policy: &FieldPolicy,
    path: &str,
    (scope, answers): (&SubstitutionScope<L, E>, &Answers),
) -> Substituted {
    let context = Context {
        scope,
        policy,
        answers: Some(answers),
    };
    let tokens = tokens_of(template, &context);
    let mut resolved = IndexMap::new();
    for (raw, token) in &tokens {
        resolved.insert(raw.clone(), token.resolve(&context).await);
    }
    let mut references: Vec<TokenReference> = Vec::new();
    for token in tokens.values() {
        if !references.contains(&token.reference) {
            references.push(token.reference.clone());
        }
    }
    let mut text = String::new();
    let mut last = 0;
    for (range, _) in braced_references(template) {
        text.push_str(template.get(last..range.start).unwrap_or_default());
        let raw = template.get(range.clone()).unwrap_or_default();
        text.push_str(resolved.get(raw).map_or(raw, |one| one.text.as_str()));
        last = range.end;
    }
    text.push_str(template.get(last..).unwrap_or_default());
    let all: Vec<&Resolution> = resolved.values().collect();
    Substituted {
        text,
        issues: all
            .iter()
            .filter_map(|one| one.problem.as_deref().map(|problem| warning(path, problem)))
            .collect(),
        referenced: references,
        unresolved: Unresolved {
            variables: unique_of(all.iter().map(|one| one.variable.clone())),
            options: unique_of(all.iter().map(|one| one.option.clone())),
        },
    }
}

/// Which reference kinds name an environment variable of the host.
const fn from_environment(reference: &TokenReference) -> bool {
    match reference {
        TokenReference::PathVariable { .. }
        | TokenReference::UserConfig { .. }
        | TokenReference::Extra { .. } => false,
        TokenReference::Environment { .. } => true,
    }
}

/// The name a reference carries, if its kind has one.
pub(super) fn name_of(reference: &TokenReference) -> Option<String> {
    match reference {
        TokenReference::PathVariable { name } => Some(name.as_str().to_owned()),
        TokenReference::Environment { name } | TokenReference::Extra { name } => Some(name.clone()),
        TokenReference::UserConfig { .. } => None,
    }
}

/// The environment variables among `referenced`, each once, in order.
#[must_use]
pub fn environment_names(referenced: &[TokenReference]) -> Vec<String> {
    let names = referenced
        .iter()
        .filter(|one| from_environment(one))
        .filter_map(name_of);
    names.collect::<IndexSet<_>>().into_iter().collect()
}

/// The path variables whose value the template writes, under the policy's rendering.
#[must_use]
pub fn needed_variables<L, E>(
    template: &str,
    scope: &SubstitutionScope<L, E>,
    policy: &FieldPolicy,
) -> Vec<PluginVariable> {
    if !rendering_needs_value(policy.paths) {
        return Vec::new();
    }
    let probe = Context {
        scope,
        policy,
        answers: None,
    };
    tokens_of(template, &probe)
        .values()
        .filter_map(|token| token.variable)
        .collect()
}

/// `runWith`'s `base`: the scope and the variables the host answered.
#[derive(Deserialize)]
struct Base {
    scope: SubstitutionScope,
    answers: Map<PluginVariable, String>,
}

async fn run_with_base(
    template: String,
    policy: PolicyArg,
    path: String,
    base: Base,
) -> Substituted {
    let policy = policy.read().await;
    let answers: Answers = base.answers.entries.into_iter().collect();
    run_with(&template, &policy, &path, (&base.scope, &answers)).await
}

async fn needed_variables_of(
    template: String,
    scope: SubstitutionScope,
    policy: PolicyArg,
) -> Vec<PluginVariable> {
    needed_variables(&template, &scope, &policy.read().await)
}

crate::registry::exports! {
    "loading/resolve/substitute" {
        "SENSITIVE_PLACEHOLDER" => ok |key: String| sensitive_placeholder(&key);
        "runWith" => async ok |template: String, policy: PolicyArg, path: String, base: Base|
            run_with_base(template, policy, path, base).await;
        "environmentNames" => ok |referenced: Vec<TokenReference>| environment_names(&referenced);
        "neededVariables" => async ok |template: String, scope: SubstitutionScope, policy: PolicyArg|
            needed_variables_of(template, scope, policy).await;
    }
}
