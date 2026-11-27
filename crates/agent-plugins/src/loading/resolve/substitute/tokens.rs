//! The tokens of `src/loading/resolve/substitute.ts`: each reference kind's parser and what it
//! resolves to, the parsers in order (`TOKEN_PARSERS`).

use indexmap::IndexMap;

use super::super::user_config::{user_config_text, user_config_value};
use super::super::variables::{
    PLUGIN_VARIABLES, PluginVariable, render_plugin_variable, rendering_needs_value,
};
use super::{
    Answers, EnvironmentExpansion, FieldPolicy, SubstitutionScope, TokenReference, UserConfigPolicy,
};
use crate::format::mcp_env::{
    EnvLookup, EnvReference, ReadsAsEmpty, braced_references, is_variable_name,
    missing_variable_message, resolve_env_reference, split_default,
};
use crate::format::user_config_refs::valid_option_key_in;
use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::ports::environment::Environment;

/// The text a reference becomes, or why it stayed as written (`problem`).
#[derive(Default)]
pub(super) struct Resolution {
    pub(super) text: String,
    pub(super) problem: Option<String>,
    pub(super) variable: Option<String>,
    pub(super) option: Option<String>,
}

/// What a token's `resolve` reads, per parser.
enum Resolve {
    UserConfig {
        key: String,
        raw: String,
        disposition: Disposition,
        declared: bool,
    },
    PathVariable(PluginVariable),
    Extra {
        name: String,
        value: String,
    },
    Environment(EnvReference),
}

pub(super) struct Token {
    pub(super) reference: TokenReference,
    resolve: Resolve,
    /// The path variable a path-variable reference names: what the host is asked for.
    pub(super) variable: Option<PluginVariable>,
}

pub(super) struct Context<'a, L, E> {
    pub(super) scope: &'a SubstitutionScope<L, E>,
    pub(super) policy: &'a FieldPolicy,
    /// The variables as the host reported them; only the ones a rendering needs are asked for.
    pub(super) answers: Option<&'a Answers>,
}

impl<L, E> Context<'_, L, E> {
    /// The variable as the host reported it.
    fn variable(&self, name: PluginVariable) -> String {
        self.answers
            .and_then(|answers| answers.get(&name).cloned())
            .unwrap_or_default()
    }
}

type TokenParser<L, E> = fn(&str, &Context<'_, L, E>) -> Option<Token>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Disposition {
    Substitute,
    Placeholder,
    Keep,
}

/// Whether a field takes a value, by whether the option is sensitive.
const fn disposition_of(policy: UserConfigPolicy, sensitive: bool) -> Disposition {
    match policy {
        UserConfigPolicy::Allow => Disposition::Substitute,
        UserConfigPolicy::NonSensitive => {
            if sensitive {
                Disposition::Placeholder
            } else {
                Disposition::Substitute
            }
        }
        UserConfigPolicy::Refuse => Disposition::Keep,
    }
}

/// What stands for a sensitive value in skill, command and agent content, where "a sensitive value
/// there becomes a placeholder" (plugins/manifest-reference.md). The docs give no text; Claude Code
/// 2.1.319 sends the model this one, a command's body included, saved value or not
/// (the spec's model-content oracle).
#[must_use]
pub fn sensitive_placeholder(key: &str) -> String {
    format!("[sensitive option '{key}' not available in skill content]")
}

fn rendering(disposition: Disposition, raw: &str, key: &str, value: &JsValue) -> String {
    match disposition {
        Disposition::Substitute => user_config_text(value),
        Disposition::Placeholder => sensitive_placeholder(key),
        Disposition::Keep => raw.to_owned(),
    }
}

fn user_config_token<L, E>(inner: &str, context: &Context<'_, L, E>) -> Option<Token> {
    let key = valid_option_key_in(inner)?;
    let raw = format!("${{{inner}}}");
    let declared = context
        .scope
        .user_config
        .declared
        .get(key)
        .filter(|value| !matches!(value, JsValue::Undefined));
    let sensitive = declared.and_then(|one| one.get("sensitive")) == Some(&JsValue::Bool(true));
    Some(Token {
        reference: TokenReference::UserConfig {
            key: key.to_owned(),
        },
        resolve: Resolve::UserConfig {
            key: key.to_owned(),
            raw,
            disposition: disposition_of(context.policy.user_config, sensitive),
            declared: declared.is_some(),
        },
        variable: None,
    })
}

fn resolve_user_config<L, E>(
    (key, raw, disposition, declared): (&str, &str, Disposition, bool),
    context: &Context<'_, L, E>,
) -> Resolution {
    // A sensitive option is never shown, so whether it has a value does not matter (oracle).
    if disposition == Disposition::Placeholder {
        return Resolution {
            text: sensitive_placeholder(key),
            ..Resolution::default()
        };
    }
    let value = user_config_value(&context.scope.user_config, key);
    if value.is_none() && disposition != Disposition::Keep {
        let why = if declared {
            "has no value"
        } else {
            "is not declared in userConfig"
        };
        return Resolution {
            text: raw.to_owned(),
            problem: Some(format!("${{user_config.{key}}} {why}")),
            option: Some(key.to_owned()),
            ..Resolution::default()
        };
    }
    let text = value.map_or_else(
        || raw.to_owned(),
        |value| rendering(disposition, raw, key, &value),
    );
    Resolution {
        text,
        ..Resolution::default()
    }
}

fn path_variable_token<L, E>(inner: &str, _context: &Context<'_, L, E>) -> Option<Token> {
    // Judgment: a path variable always resolves, so a `:-default` written after it is ignored.
    let EnvReference { name, .. } = split_default(inner);
    let name = PLUGIN_VARIABLES
        .into_iter()
        .find(|variable| variable.as_str() == name)?;
    Some(Token {
        reference: TokenReference::PathVariable { name },
        resolve: Resolve::PathVariable(name),
        variable: Some(name),
    })
}

fn resolve_path_variable<L, E>(name: PluginVariable, context: &Context<'_, L, E>) -> String {
    let Context { policy, scope, .. } = context;
    let value = if rendering_needs_value(policy.paths) {
        context.variable(name)
    } else {
        String::new()
    };
    render_plugin_variable(policy.paths, name.as_str(), &value, scope.windows)
}

/// An `extra` path (`SubstitutionScope.extraPaths`) is written as the policy writes a path.
fn extra_text<L, E>(name: &str, value: &str, context: &Context<'_, L, E>) -> String {
    let Context { scope, policy, .. } = context;
    if scope
        .extra_paths
        .as_ref()
        .is_some_and(|paths| paths.iter().any(|path| path == name))
    {
        render_plugin_variable(policy.paths, name, value, scope.windows)
    } else {
        value.to_owned()
    }
}

fn extra_token<L, E>(inner: &str, context: &Context<'_, L, E>) -> Option<Token> {
    let EnvReference { name, .. } = split_default(inner);
    let value = context.scope.extra.as_ref()?.get(&name)?.clone();
    Some(Token {
        reference: TokenReference::Extra { name: name.clone() },
        resolve: Resolve::Extra { name, value },
        variable: None,
    })
}

const fn expands(expansion: EnvironmentExpansion) -> bool {
    match expansion {
        EnvironmentExpansion::Expand => true,
        EnvironmentExpansion::Literal => false,
    }
}

/// The rule is `format/mcp-env`'s, the one servers of the settings scopes use too.
fn environment_token<L, E>(inner: &str, context: &Context<'_, L, E>) -> Option<Token> {
    let reference = split_default(inner);
    if !expands(context.policy.environment) || !is_variable_name(&reference.name) {
        return None;
    }
    Some(Token {
        reference: TokenReference::Environment {
            name: reference.name.clone(),
        },
        resolve: Resolve::Environment(reference),
        variable: None,
    })
}

/// `(name) => scope.environment.get(name)`.
struct EnvironmentLookup<'a, E>(&'a E);

impl<E: Environment> EnvLookup for EnvironmentLookup<'_, E> {
    fn lookup<'b>(&'b self, name: &'b str) -> BoxFuture<'b, Option<String>> {
        self.0.get(name)
    }
}

async fn resolve_environment<L, E: Environment>(
    reference: &EnvReference,
    context: &Context<'_, L, E>,
) -> Resolution {
    let reads = context
        .policy
        .reads_as_empty
        .as_ref()
        .map(|rule| rule as &dyn ReadsAsEmpty);
    let lookup = EnvironmentLookup(&context.scope.environment);
    let done = resolve_env_reference(reference, &lookup, reads).await;
    if done.missing {
        Resolution {
            text: done.text,
            problem: Some(missing_variable_message(&reference.name)),
            variable: Some(reference.name.clone()),
            ..Resolution::default()
        }
    } else {
        Resolution {
            text: done.text,
            ..Resolution::default()
        }
    }
}

impl Token {
    pub(super) async fn resolve<L, E: Environment>(
        &self,
        context: &Context<'_, L, E>,
    ) -> Resolution {
        match &self.resolve {
            Resolve::UserConfig {
                key,
                raw,
                disposition,
                declared,
            } => resolve_user_config((key, raw, *disposition, *declared), context),
            Resolve::PathVariable(name) => Resolution {
                text: resolve_path_variable(*name, context),
                ..Resolution::default()
            },
            Resolve::Extra { name, value } => Resolution {
                text: extra_text(name, value, context),
                ..Resolution::default()
            },
            Resolve::Environment(reference) => resolve_environment(reference, context).await,
        }
    }
}

/// In order: the first parser that knows the reference decides.
const fn token_parsers<L, E>() -> [TokenParser<L, E>; 4] {
    [
        user_config_token,
        path_variable_token,
        extra_token,
        environment_token,
    ]
}

fn parse_token<L, E>(inner: &str, context: &Context<'_, L, E>) -> Option<Token> {
    token_parsers()
        .into_iter()
        .find_map(|parser| parser(inner, context))
}

/// Every distinct reference of the template, parsed once.
pub(super) fn tokens_of<L, E>(
    template: &str,
    context: &Context<'_, L, E>,
) -> IndexMap<String, Token> {
    let mut tokens = IndexMap::new();
    for (range, inner) in braced_references(template) {
        let raw = template.get(range).unwrap_or_default();
        if tokens.contains_key(raw) {
            continue;
        }
        if let Some(token) = parse_token(inner, context) {
            tokens.insert(raw.to_owned(), token);
        }
    }
    tokens
}
