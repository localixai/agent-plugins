//! Port of `src/format/mcp-oauth.ts`.
//!
//! The `oauth` block of a remote server's entry and what it decides (mcp.md, "Use a fixed OAuth
//! callback port", "Use pre-configured OAuth credentials", "Override OAuth metadata discovery",
//! "Restrict OAuth scopes", "Use OAuth with a server behind an authenticating proxy", "Authenticate
//! with remote MCP servers"). Pure data: the OAuth flow itself, the browser, the token store and the
//! HTTP requests are the host's.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::format::helper_headers::has_authorization_header;
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::mcp_config::signs_in_with_oauth;
use crate::js::url::{INVALID_URL_MESSAGE, parse};
use crate::js::{JsObject, JsValue, is_whitespace, number_to_string, property_order, string_of};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, fields};

/// The `oauth` block's fields as a host passes them back (`McpOAuthConfig`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOAuthConfig {
    /// The pre-registered client's ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// The fixed port of the redirect URI `http://localhost:PORT/callback`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub callback_port: Option<f64>,
    /// "The URL must use `https://`": the metadata document that replaces the discovery chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_server_metadata_url: Option<String>,
    /// "A single space-separated string, matching the `scope` parameter format in RFC 6749 §3.3".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<String>,
}

/// What reading an `oauth` block gives; the block is held as the object read, its keys in order.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpOAuthRead {
    /// Undefined when the block is absent or one of its fields is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<JsObject>,
    /// Why, or the keys it does not keep.
    pub issues: Vec<PluginIssue>,
}

fn is_https(text: &JsValue) -> bool {
    text.as_str()
        .and_then(parse)
        .is_some_and(|url| url.protocol == "https:")
}

/// Judgment (spec/judgments.md): a port is a whole number a redirect URI can carry, 1 to 65535.
const FIELDS: [&str; 4] = [
    "clientId",
    "callbackPort",
    "authServerMetadataUrl",
    "scopes",
];

static BLOCK: LazyLock<Schema> = LazyLock::new(|| {
    zod::loose_object(fields! {
        "clientId" => zod::string().min(1.0).exact_optional(),
        "callbackPort" => zod::int().min(1.0).max(65535.0).exact_optional(),
        "authServerMetadataUrl" => zod::string().refine(is_https, "must be an https:// URL").exact_optional(),
        "scopes" => zod::string().min(1.0).exact_optional(),
    })
});

fn refused(name: &str, why: &str) -> McpOAuthRead {
    McpOAuthRead {
        oauth: None,
        issues: vec![error(
            "oauth",
            &format!("Invalid MCP server config for \"{name}\": {why}"),
        )],
    }
}

/// The one key mcp.md says never lives in the configuration.
const SECRET_KEY: &str = "clientSecret";

fn unknown_key(name: &str, origin: &str, key: &str) -> PluginIssue {
    let message = if key == SECRET_KEY {
        format!(
            "MCP server \"{name}\" has \"oauth.{key}\", which is not a configuration field: the client secret is given only when the server is added (--client-secret, MCP_CLIENT_SECRET)"
        )
    } else {
        format!("MCP server \"{name}\" has \"oauth.{key}\", which Claude Code does not read")
    };
    warning(origin, &message)
}

/// Reads the value of a server's `oauth` key. Judgments (spec/judgments.md): a field the docs do not
/// list is a warning and is not kept (so a secret never travels in the entry); a field of the wrong
/// type makes the whole block unusable, since half an OAuth setup would sign in with another client.
#[must_use]
pub fn read_mcp_oauth(name: &str, raw: &JsValue, origin: &str) -> McpOAuthRead {
    if matches!(raw, JsValue::Undefined) {
        return McpOAuthRead {
            oauth: None,
            issues: Vec::new(),
        };
    }
    if !is_record(raw) {
        return refused(name, "oauth: expected an object");
    }
    let data = match BLOCK.safe_parse(raw) {
        Ok(data) => data,
        Err(failure) => {
            let why = failure
                .issues
                .iter()
                .map(|found| {
                    let path: Vec<String> = std::iter::once("oauth".to_owned())
                        .chain(found.path().iter().map(string_of))
                        .collect();
                    format!("{}: {}", path.join("."), found.message())
                })
                .collect::<Vec<_>>()
                .join("; ");
            return refused(name, &why);
        }
    };
    let entries = data.as_object().map(property_order).unwrap_or_default();
    let known = |key: &str| FIELDS.contains(&key);
    let kept: JsObject = entries
        .iter()
        .filter(|(key, _)| known(key))
        .map(|(key, value)| ((*key).clone(), (*value).clone()))
        .collect();
    let issues = entries
        .iter()
        .filter(|(key, _)| !known(key))
        .map(|(key, _)| unknown_key(name, origin, key))
        .collect();
    McpOAuthRead {
        oauth: Some(kept),
        issues,
    }
}

/// The redirect URI a fixed callback port registers (mcp.md: `http://localhost:PORT/callback`; never `127.0.0.1`).
#[must_use]
pub fn mcp_oauth_redirect_uri(port: f64) -> String {
    format!("http://localhost:{}/callback", number_to_string(port))
}

/// Where the authorization server's metadata is looked for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum McpOAuthDiscovery {
    /// `authServerMetadataUrl`.
    Configured {
        /// The metadata document.
        url: String,
    },
    /// The discovery chain.
    Chain {
        /// The two well-known documents, in the order they are tried.
        urls: Vec<String>,
    },
}

const PROTECTED_RESOURCE: &str = "/.well-known/oauth-protected-resource";
const AUTHORIZATION_SERVER: &str = "/.well-known/oauth-authorization-server";

/// "By default, Claude Code first checks RFC 9728 Protected Resource Metadata at
/// `/.well-known/oauth-protected-resource`, then falls back to RFC 8414 authorization server metadata
/// at `/.well-known/oauth-authorization-server`"; `authServerMetadataUrl` bypasses that chain.
/// Judgment (spec/judgments.md): both paths are taken from the server's origin, as the docs write them
/// (RFC 9728 and RFC 8414 also define forms that insert the path of the resource).
///
/// # Errors
///
/// `new URL(serverUrl)`'s `TypeError` for a URL it refuses.
pub fn mcp_oauth_discovery(
    server_url: &str,
    config: Option<&McpOAuthConfig>,
) -> Result<McpOAuthDiscovery, String> {
    if let Some(url) = config.and_then(|config| config.auth_server_metadata_url.clone()) {
        return Ok(McpOAuthDiscovery::Configured { url });
    }
    let origin = parse(server_url)
        .ok_or_else(|| INVALID_URL_MESSAGE.to_owned())?
        .origin;
    Ok(McpOAuthDiscovery::Chain {
        urls: vec![
            format!("{origin}{PROTECTED_RESOURCE}"),
            format!("{origin}{AUTHORIZATION_SERVER}"),
        ],
    })
}

/// Where scope metadata came from: `configured`, fetched from `authServerMetadataUrl`; `discovered`, found by the chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpOAuthSource {
    /// Fetched from `authServerMetadataUrl`.
    Configured,
    /// Found by the chain.
    Discovered,
}

/// The metadata of an authorization server as far as scopes go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOAuthMetadata {
    /// Where it came from.
    pub source: McpOAuthSource,
    /// Its `scopes_supported`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes_supported: Option<Vec<String>>,
}

const OFFLINE_ACCESS: &str = "offline_access";

/// "Metadata fetched from a configured `authServerMetadataUrl` still supplies its `scopes_supported`"; discovered metadata does not.
const fn supplies_scopes(source: McpOAuthSource) -> bool {
    match source {
        McpOAuthSource::Configured => true,
        McpOAuthSource::Discovered => false,
    }
}

/// The `scope` of the authorization request, or `undefined` for none (the server decides). "`oauth.scopes`
/// takes precedence over both `authServerMetadataUrl` and the scopes the server discovers"; when the
/// server advertises `offline_access` it is appended to the pinned scopes. Unset: discovered metadata
/// does not put its whole `scopes_supported` catalog in the request, while metadata fetched from a
/// configured `authServerMetadataUrl` "still supplies its `scopes_supported` as the requested scopes".
#[must_use]
pub fn requested_oauth_scopes(
    config: Option<&McpOAuthConfig>,
    metadata: &McpOAuthMetadata,
) -> Option<String> {
    let supported = metadata.scopes_supported.as_deref().unwrap_or_default();
    if let Some(scopes) = config.and_then(|config| config.scopes.as_deref()) {
        // `split(/\s+/)`, `\s` being JavaScript's whitespace (RS6).
        let mut pinned: Vec<&str> = scopes
            .split(is_whitespace)
            .filter(|scope| !scope.is_empty())
            .collect();
        let has = |list: &[&str]| list.contains(&OFFLINE_ACCESS);
        let advertised = supported.iter().any(|scope| scope == OFFLINE_ACCESS);
        if advertised && !has(&pinned) {
            pinned.push(OFFLINE_ACCESS);
        }
        return Some(pinned.join(" "));
    }
    (supplies_scopes(metadata.source) && !supported.is_empty()).then(|| supported.join(" "))
}

/// Why a server does not sign in with OAuth.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpOAuthRefusal {
    /// Its transport has no OAuth.
    Transport,
    /// It sends an `Authorization` header of its own.
    AuthorizationHeader,
}

/// Whether a server signs in with OAuth, and with which headers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpOAuthPlan {
    /// `oauth: true`.
    OAuth {
        /// Always `true`.
        oauth: bool,
        /// Sent with the OAuth requests too, so a proxy that checks a header of its own lets them through.
        headers: IndexMap<String, String>,
        /// The block's configuration, when the entry has one.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        config: Option<JsValue>,
    },
    /// `oauth: false`.
    No {
        /// Always `false`.
        oauth: bool,
        /// Why not.
        reason: McpOAuthRefusal,
    },
}

/// The connection a plan is made for.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct McpOAuthConnection {
    /// The headers the connection sends.
    pub headers: IndexMap<String, String>,
    /// The entry's `oauth` block.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth: Option<JsValue>,
}

/// Whether a server signs in with OAuth, and with which headers. "OAuth authentication works with HTTP
/// servers" and the flags apply "to HTTP and SSE transports"; WebSocket "supports neither" and a stdio
/// server has no URL. "If you configure an `Authorization` header, Claude Code uses it as the server's
/// credential and doesn't fall back to OAuth for that server": `headers` are the ones the connection
/// sends (static headers with a `headersHelper`'s output merged in, as `connectionHeaders` does).
#[must_use]
pub fn plan_mcp_oauth(server: &JsValue, connection: McpOAuthConnection) -> McpOAuthPlan {
    let refused = |reason| McpOAuthPlan::No {
        oauth: false,
        reason,
    };
    if !signs_in_with_oauth(server) {
        return refused(McpOAuthRefusal::Transport);
    }
    if has_authorization_header(&connection.headers) {
        return refused(McpOAuthRefusal::AuthorizationHeader);
    }
    McpOAuthPlan::OAuth {
        oauth: true,
        headers: connection.headers,
        config: connection
            .oauth
            .filter(|oauth| !matches!(oauth, JsValue::Undefined)),
    }
}

/// What a rejected request means for a server (mcp.md, "Authenticate with remote MCP servers").
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpAuthRejectionInput {
    /// The response's status.
    pub status: f64,
    /// A configured server, a claude.ai connector, or a connector delivered to a cloud session.
    pub source: McpAuthSource,
    /// An `Authorization` header configured in `headers` or produced by a `headersHelper`.
    pub authorization_configured: bool,
    /// For a connector: the 401 is claude.ai rejecting the session token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_token_rejected: Option<bool>,
}

/// Where a rejected server comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpAuthSource {
    /// A configured server.
    Server,
    /// A claude.ai connector.
    Connector,
    /// A connector delivered to a cloud session.
    CloudConnector,
}

/// What `/mcp` shows for a rejected server.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpAuthState {
    /// It needs a sign-in.
    NeedsAuthentication,
    /// Its own credential failed.
    Failed,
    /// claude.ai rejected the session token.
    SessionTokenRejected,
    /// Reconnect it in claude.ai.
    ReconnectInClaudeAi,
    /// The status is no rejection.
    NotARejection,
}

/// "A remote server as needing authentication when it responds with `401 Unauthorized` or `403 Forbidden`".
const REJECTING_STATUSES: [f64; 2] = [401.0, 403.0];

const UNAUTHORIZED: f64 = 401.0;

const fn signed_in_or_failed(authorization_configured: bool) -> McpAuthState {
    if authorization_configured {
        McpAuthState::Failed
    } else {
        McpAuthState::NeedsAuthentication
    }
}

/// `REACTIONS[input.source](input)`.
fn reaction(input: &McpAuthRejectionInput) -> McpAuthState {
    match input.source {
        McpAuthSource::Server => signed_in_or_failed(input.authorization_configured),
        // "A `401` caused by claude.ai rejecting your session token": a `403` is flagged like any other.
        McpAuthSource::Connector => {
            if input.status == UNAUTHORIZED && input.session_token_rejected == Some(true) {
                return McpAuthState::SessionTokenRejected;
            }
            signed_in_or_failed(input.authorization_configured)
        }
        // The session's proxy authenticates with the authorization granted in claude.ai: no sign-in flow here.
        McpAuthSource::CloudConnector => McpAuthState::ReconnectInClaudeAi,
    }
}

/// What a rejected request means for a server.
#[must_use]
pub fn mcp_auth_rejection(input: &McpAuthRejectionInput) -> McpAuthState {
    if REJECTING_STATUSES.contains(&input.status) {
        reaction(input)
    } else {
        McpAuthState::NotARejection
    }
}

/// How a stored token rejected with 401 is recovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTokenRecovery {
    /// `refresh-and-retry` or `connect-without-token`.
    pub first: &'static str,
    /// When `/mcp` flags the server: the retry failed, or the connection without the token was rejected too.
    pub flagged_when: &'static str,
    /// "Its tools work while you're signed out, and the rejected token is deleted."
    pub deletes_rejected_token: bool,
}

/// The stored login `storedTokenRecovery` reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredTokenLogin {
    /// Whether it has a refresh token.
    pub refresh_token: bool,
}

/// A stored OAuth token rejected with 401: "Your stored login has a refresh token" or not.
#[must_use]
pub const fn stored_token_recovery(login: StoredTokenLogin) -> StoredTokenRecovery {
    if login.refresh_token {
        StoredTokenRecovery {
            first: "refresh-and-retry",
            flagged_when: "retry-fails",
            deletes_rejected_token: false,
        }
    } else {
        StoredTokenRecovery {
            first: "connect-without-token",
            flagged_when: "connection-rejected",
            deletes_rejected_token: true,
        }
    }
}

crate::registry::exports! {
    "format/mcp-oauth" {
        "readMcpOAuth" => ok |name: String, raw: JsValue, origin: String| read_mcp_oauth(&name, &raw, &origin);
        "mcpOAuthRedirectUri" => ok |port: f64| mcp_oauth_redirect_uri(port);
        "mcpOAuthDiscovery" => fallible |server_url: String, config: Option<McpOAuthConfig>| mcp_oauth_discovery(&server_url, config.as_ref());
        "requestedOAuthScopes" => ok |config: Option<McpOAuthConfig>, metadata: McpOAuthMetadata| requested_oauth_scopes(config.as_ref(), &metadata);
        "planMcpOAuth" => ok |server: JsValue, connection: McpOAuthConnection| plan_mcp_oauth(&server, connection);
        "mcpAuthRejection" => ok |input: McpAuthRejectionInput| mcp_auth_rejection(&input);
        "storedTokenRecovery" => ok |login: StoredTokenLogin| stored_token_recovery(login);
    }
}
