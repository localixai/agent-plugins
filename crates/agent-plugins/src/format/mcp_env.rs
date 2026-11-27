//! Port of `src/format/mcp-env.ts`, switched.
//!
//! `${VAR}` and `${VAR:-default}` in an MCP server's `command`, `args`, `env`, `url` and `headers`
//! (mcp.md, "Environment variable expansion in `.mcp.json`"). Pure over a lookup function, so it
//! serves a server of the local, project and user scopes (no plugin, no `SubstitutionScope`) and,
//! through `resolveEnvReference` and `splitDefault`, the plugin servers of `loading/resolve`: one rule
//! for both, so they cannot drift.
//!
//! Grammar (judgment, shared with `loading/resolve/substitute`): a reference is `${` text `}` with no
//! braces inside; the default ends at the first `}` and nests nothing; the name is a shell variable
//! name, and anything else is left as written.
//!
//! The lookup and `readsAsEmpty` are the host's functions ([`EnvLookup`], [`ReadsAsEmpty`]), each
//! call a need, or the core's own closures.

use std::sync::LazyLock;

use indexmap::IndexSet;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::{JsValue, Pattern, regex_test};
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// The environment as a function: a name to its value, `None` when it is unset (`EnvLookup`): a
/// function of the host's, so its answer may wait. A Rust host passes a closure, answered at once.
pub trait EnvLookup: Send + Sync {
    /// The value of `name`.
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>>;
}

impl<F: Fn(&str) -> Option<String> + Send + Sync> EnvLookup for F {
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(std::future::ready(self(name)))
    }
}

impl EnvLookup for Callback {
    fn lookup<'a>(&'a self, name: &'a str) -> BoxFuture<'a, Option<String>> {
        Box::pin(self.call(vec![name.into()]))
    }
}

/// `(name) => boolean`: whether a variable reads as empty (only a `true` answer counts).
pub trait ReadsAsEmpty: Send + Sync {
    /// Whether `name` reads as empty.
    fn reads_as_empty<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool>;
}

impl<F: Fn(&str) -> bool + Send + Sync> ReadsAsEmpty for F {
    fn reads_as_empty<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        Box::pin(std::future::ready(self(name)))
    }
}

impl ReadsAsEmpty for Callback {
    fn reads_as_empty<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        let asked = self.call::<JsValue>(vec![name.into()]);
        Box::pin(async move { asked.await == JsValue::Bool(true) })
    }
}

/// Every `${...}` in a text; the group is what is between the braces (`BRACED_REFERENCE`, `/g`).
pub static BRACED_REFERENCE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"\$\{([^{}]*)\}").ok());

/// `BRACED_REFERENCE` as the wrapper's `RegExp`.
fn braced_reference_regexp() -> crate::registry::RegExp {
    crate::registry::RegExp {
        source: r"\$\{([^{}]*)\}".to_owned(),
        flags: "g".to_owned(),
    }
}

/// Each `${...}` of `text`: its byte range and what is between the braces.
#[must_use]
pub fn braced_references(text: &str) -> Vec<(std::ops::Range<usize>, &str)> {
    let Some(pattern) = BRACED_REFERENCE.as_ref() else {
        return Vec::new();
    };
    pattern
        .captures_iter(text)
        .filter_map(|found| {
            let whole = found.get(0)?;
            Some((whole.range(), found.get(1)?.as_str()))
        })
        .collect()
}

static VARIABLE_NAME: Pattern = LazyLock::new(|| Regex::new("^[A-Za-z_][A-Za-z0-9_]*$").ok());

/// Whether a text is a shell variable name.
#[must_use]
pub fn is_variable_name(text: &str) -> bool {
    regex_test(&VARIABLE_NAME, text)
}

/// A reference's name and its default.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvReference {
    /// The variable.
    pub name: String,
    /// What it falls back to; `undefined` with no `:-`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
}

/// `NAME` or `NAME:-default`.
#[must_use]
pub fn split_default(inner: &str) -> EnvReference {
    match inner.split_once(":-") {
        None => EnvReference {
            name: inner.to_owned(),
            fallback: None,
        },
        Some((name, fallback)) => EnvReference {
            name: name.to_owned(),
            fallback: Some(fallback.to_owned()),
        },
    }
}

/// The warning text for a reference that stayed as written (mcp.md, "Unset variables without a default").
#[must_use]
pub fn missing_variable_message(name: &str) -> String {
    format!("Environment variable {name} is not set and has no default")
}

/// What one reference becomes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvReferenceResolution {
    /// Its text.
    pub text: String,
    /// True when the variable is unset with no default: `text` is the `${VAR}` as written.
    pub missing: bool,
    /// True when a covered credential variable the environment has set read as empty.
    pub credential_read: bool,
}

/// What one reference becomes. An empty string counts as set (mcp.md, "Variables set to an empty
/// string"): the default is for an unset variable only. A name `readsAsEmpty` covers is empty whether
/// or not it is set and its default is ignored, with no warning (mcp.md, "Credential variables that
/// read as empty").
pub async fn resolve_env_reference(
    reference: &EnvReference,
    lookup: &dyn EnvLookup,
    reads_as_empty: Option<&dyn ReadsAsEmpty>,
) -> EnvReferenceResolution {
    let name = &reference.name;
    let value = lookup.lookup(name).await;
    if let Some(reads) = reads_as_empty
        && reads.reads_as_empty(name).await
    {
        return EnvReferenceResolution {
            text: String::new(),
            missing: false,
            credential_read: value.is_some(),
        };
    }
    match value.or_else(|| reference.fallback.clone()) {
        None => EnvReferenceResolution {
            text: format!("${{{name}}}"),
            missing: true,
            credential_read: false,
        },
        Some(text) => EnvReferenceResolution {
            text,
            missing: false,
            credential_read: false,
        },
    }
}

/// A text expanded.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct EnvExpansion {
    /// The text.
    pub text: String,
    /// Variables left as written because they are unset and have no default, once each.
    pub missing: Vec<String>,
    /// Covered credential variables that are set and were read as empty, once each.
    pub credentials: Vec<String>,
}

/// Expands every reference of a text in one pass: a value is never scanned again.
pub async fn expand_env_text(
    text: &str,
    lookup: &dyn EnvLookup,
    reads_as_empty: Option<&dyn ReadsAsEmpty>,
) -> EnvExpansion {
    let mut missing = IndexSet::new();
    let mut credentials = IndexSet::new();
    let mut expanded = String::new();
    let mut last = 0;
    for (range, inner) in braced_references(text) {
        expanded.push_str(text.get(last..range.start).unwrap_or_default());
        last = range.end;
        let reference = split_default(inner);
        if !is_variable_name(&reference.name) {
            expanded.push_str(text.get(range).unwrap_or_default());
            continue;
        }
        let done = resolve_env_reference(&reference, lookup, reads_as_empty).await;
        if done.missing {
            missing.insert(reference.name.clone());
        }
        if done.credential_read {
            credentials.insert(reference.name.clone());
        }
        expanded.push_str(&done.text);
    }
    expanded.push_str(text.get(last..).unwrap_or_default());
    EnvExpansion {
        text: expanded,
        missing: missing.into_iter().collect(),
        credentials: credentials.into_iter().collect(),
    }
}

mod server;

pub use server::{
    DisplayedHeader, MCP_DISPLAY_SCOPES, McpDisplayScope, McpExpansion, McpExpansionOptions,
    display_headers, display_reference, expand_mcp_server,
};

crate::registry::exports! {
    "format/mcp-env" {
        "BRACED_REFERENCE" => ok braced_reference_regexp;
        "isVariableName" => ok |text: String| is_variable_name(&text);
        "splitDefault" => ok |inner: String| split_default(&inner);
        "missingVariableMessage" => ok |name: String| missing_variable_message(&name);
        "resolveEnvReference" => async ok |reference: EnvReference, lookup: Callback, reads_as_empty: Option<Callback>|
            resolve_env_reference(&reference, &lookup, reads_as_empty.as_ref().map(|f| f as &dyn ReadsAsEmpty)).await;
        "expandEnvText" => async ok |text: String, lookup: Callback, reads_as_empty: Option<Callback>|
            expand_env_text(&text, &lookup, reads_as_empty.as_ref().map(|f| f as &dyn ReadsAsEmpty)).await;
        "expandMcpServer" => async ok |name: String, server: JsValue, lookup: Callback, options: Option<McpExpansionOptions>|
            expand_mcp_server(&name, &server, &lookup, options.unwrap_or_default()).await;
        "MCP_DISPLAY_SCOPES" => ok || MCP_DISPLAY_SCOPES;
        "displayReference" => ok |server: JsValue, scope: McpDisplayScope| display_reference(&server, scope);
        "displayHeaders" => ok |server: JsValue, scope: McpDisplayScope| display_headers(&server, scope);
    }
}

#[cfg(test)]
mod tests;
