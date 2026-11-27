//! `src/format/mcp-startup.ts`, its client runtime part: `MCP_SDK_GENERATION` and
//! `MCP_PROTOCOL_NEGOTIATION`.

use serde::{Deserialize, Serialize};

use super::variable;
use crate::format::mcp_env::EnvLookup;

/// The MCP client runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpClientRuntime {
    /// `v1`.
    V1,
    /// `v2`.
    V2,
}

/// `MCP_PROTOCOL_NEGOTIATION`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum McpProtocolNegotiation {
    /// `legacy`.
    Legacy,
    /// `auto`.
    Auto,
    /// `default`.
    Default,
}

/// What the client settings are.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpClientSettings {
    /// The runtime.
    pub runtime: McpClientRuntime,
    /// `MCP_PROTOCOL_NEGOTIATION`, which applies "on the v2 MCP client runtime only": `legacy` keeps every
    /// server on the earlier handshake, `auto` probes the kinds that support revision 2026-07-28, and
    /// `default` (unset) probes them but may skip some. `undefined` on v1.
    #[serde(serialize_with = "crate::js::value::serialize_or_undefined")]
    pub negotiation: Option<McpProtocolNegotiation>,
    /// The v2 runtime "checks the issuer an MCP OAuth server returns in its authorization response".
    pub checks_issuer: bool,
    /// For the debug log: "any other value is ignored with a warning". The wording is the kit's.
    pub warnings: Vec<String>,
}

const RUNTIMES: &[(&str, McpClientRuntime)] =
    &[("v1", McpClientRuntime::V1), ("v2", McpClientRuntime::V2)];
const NEGOTIATIONS: &[(&str, McpProtocolNegotiation)] = &[
    ("legacy", McpProtocolNegotiation::Legacy),
    ("auto", McpProtocolNegotiation::Auto),
];

/// The negotiation variable and the issuer check belong to the v2 runtime only (`IS_V2`).
pub(super) const fn is_v2(runtime: McpClientRuntime) -> bool {
    match runtime {
        McpClientRuntime::V1 => false,
        McpClientRuntime::V2 => true,
    }
}

/// The value the table names, or the default with the warning for a value it does not name.
fn choice<T: Copy>(
    table: &[(&str, T)],
    name: &str,
    text: Option<&str>,
    fallback: T,
) -> (T, Vec<String>) {
    let Some(text) = text else {
        return (fallback, Vec::new());
    };
    match table.iter().find(|(key, _)| *key == text) {
        Some((_, named)) => (*named, Vec::new()),
        None => (
            fallback,
            vec![format!(
                "{name}=\"{text}\" is not a value it takes: ignored"
            )],
        ),
    }
}

/// `MCP_SDK_GENERATION` pins the client runtime (`v1`, or `v2`, the default; "if you set an unrecognized
/// value, Claude Code ignores it and writes a warning to the debug log") and `MCP_PROTOCOL_NEGOTIATION`
/// decides whether the v2 runtime probes servers for the newer revision.
pub async fn mcp_client_settings(lookup: &dyn EnvLookup) -> McpClientSettings {
    let text = variable(lookup, "MCP_SDK_GENERATION").await;
    let (runtime, mut warnings) = choice(
        RUNTIMES,
        "MCP_SDK_GENERATION",
        text.as_deref(),
        McpClientRuntime::V2,
    );
    let v2 = is_v2(runtime);
    let text = variable(lookup, "MCP_PROTOCOL_NEGOTIATION").await;
    let (negotiation, negotiation_warnings) = choice(
        NEGOTIATIONS,
        "MCP_PROTOCOL_NEGOTIATION",
        text.as_deref(),
        McpProtocolNegotiation::Default,
    );
    if v2 {
        warnings.extend(negotiation_warnings);
    }
    McpClientSettings {
        runtime,
        negotiation: v2.then_some(negotiation),
        checks_issuer: v2,
        warnings,
    }
}
