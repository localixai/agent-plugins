//! Port of `src/settings/mcp/origins.ts`, switched.
//!
//! Where a server comes from, as far as the lists are concerned (managed-mcp.md, "How a server is
//! evaluated" and "Servers that skip the allowlist check"). `configured` is every server a user, a
//! plugin or claude.ai adds, an agent's frontmatter declares or `--mcp-config` passes: all three checks.
//!
//! `ORIGIN_RULES` crosses as the host's table: each rule's `skipsAllowlist` a function of the core's.
//! A checked member that is not text (a `url` or an argument of another type, which the
//! TypeScript's `matchAll` would throw on) holds no reference here: the file's reader refused it.

use indexmap::IndexMap;
use serde::Serialize;

use crate::format::mcp_config::is_remote_mcp_server;
use crate::format::mcp_env::{braced_references, is_variable_name, split_default};
use crate::js::{JsValue, property_order};
use crate::registry::Function;

/// Where a server comes from, as far as the lists are concerned.
pub const MCP_SERVER_ORIGINS: [&str; 6] = [
    "configured",
    "managedSettings",
    "managedFile",
    "builtIn",
    "claudeTagSlack",
    "inProcess",
];

/// Whether `text` holds a `${VAR}` reference (a braced reference whose name is a variable name).
#[must_use]
pub fn mentions_variable(text: &str) -> bool {
    braced_references(text)
        .into_iter()
        .any(|(_, inner)| is_variable_name(&split_default(inner).name))
}

/// Every text a `managed-mcp.json` server's "command, arguments, URL, or headers" holds; `env` is not one.
fn checked_texts(server: &JsValue) -> Vec<&JsValue> {
    let mut texts = Vec::new();
    if is_remote_mcp_server(server) {
        texts.extend(server.get("url"));
        if let Some(headers) = server.get("headers").and_then(JsValue::as_object) {
            texts.extend(property_order(headers).into_iter().map(|(_, value)| value));
        }
    } else {
        texts.extend(server.get("command"));
        if let Some(args) = server.get("args").and_then(JsValue::as_array) {
            texts.extend(args);
        }
    }
    texts
}

/// Whether a server of an origin skips the allowlist check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkipsAllowlist {
    /// It never does.
    Never,
    /// It always does.
    Always,
    /// "any `managed-mcp.json` entry whose values use no `${VAR}` expansion outside `env`"
    WithoutReferences,
}

impl SkipsAllowlist {
    /// The rule for one server.
    #[must_use]
    pub fn skips(self, server: &JsValue) -> bool {
        match self {
            Self::Never => false,
            Self::Always => true,
            Self::WithoutReferences => !checked_texts(server)
                .into_iter()
                .any(|text| text.as_str().is_some_and(mentions_variable)),
        }
    }
}

/// How the lists treat a server of one origin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginRule {
    /// The denylist applies; in-process `type: "sdk"` servers "skip all three" checks.
    pub denylist: bool,
    /// The server is not checked against the allowlist.
    pub skips_allowlist: SkipsAllowlist,
}

/// The rules by origin, in the TypeScript's order.
pub const ORIGIN_RULES: [(&str, OriginRule); 6] = [
    (
        "configured",
        OriginRule {
            denylist: true,
            skips_allowlist: SkipsAllowlist::Never,
        },
    ),
    // "every `managedMcpServers` entry"
    (
        "managedSettings",
        OriginRule {
            denylist: true,
            skips_allowlist: SkipsAllowlist::Always,
        },
    ),
    // "any `managed-mcp.json` entry whose values use no `${VAR}` expansion outside `env`"
    (
        "managedFile",
        OriginRule {
            denylist: true,
            skips_allowlist: SkipsAllowlist::WithoutReferences,
        },
    ),
    // "Claude in Chrome, the `ide` server ..., and servers the CLI itself configures"
    (
        "builtIn",
        OriginRule {
            denylist: true,
            skips_allowlist: SkipsAllowlist::Always,
        },
    ),
    // "a Claude Tag session's Slack tools"
    (
        "claudeTagSlack",
        OriginRule {
            denylist: true,
            skips_allowlist: SkipsAllowlist::Always,
        },
    ),
    (
        "inProcess",
        OriginRule {
            denylist: false,
            skips_allowlist: SkipsAllowlist::Always,
        },
    ),
];

/// `skipsAllowlist` as the host's function.
#[derive(Debug)]
struct Skips(SkipsAllowlist);

crate::registry::methods! {
    Skips {
        "call" => ok |this, server: JsValue| this.0.skips(&server);
    }
}

/// A rule as the host holds it.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginRow {
    denylist: bool,
    skips_allowlist: Function,
}

fn origin_rules_table() -> IndexMap<&'static str, OriginRow> {
    ORIGIN_RULES
        .into_iter()
        .map(|(origin, rule)| {
            (
                origin,
                OriginRow {
                    denylist: rule.denylist,
                    skips_allowlist: Function::new(Skips(rule.skips_allowlist)),
                },
            )
        })
        .collect()
}

crate::registry::exports! {
    "settings/mcp/origins" {
        "MCP_SERVER_ORIGINS" => ok || MCP_SERVER_ORIGINS;
        "ORIGIN_RULES" => ok origin_rules_table;
    }
}
