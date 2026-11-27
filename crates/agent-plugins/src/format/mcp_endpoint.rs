//! Port of `src/format/mcp-endpoint.ts`, switched.
//!
//! What identifies the endpoint of an MCP server (mcp.md, "Scope hierarchy and precedence"): plugins
//! and connectors are matched with the servers above them by endpoint, "so one that points at the same
//! URL or command as an enabled server above counts as a duplicate".
//!
//! A URL: "Two URL spellings count as the same endpoint when they differ only in the letter case of the
//! scheme or host, the scheme's default port, such as `:443` on `https`, or a trailing slash. A
//! different path, query string, userinfo, or non-default port makes two servers." The URL parser
//! lowercases scheme and host and drops the default port; the trailing slash of a path is dropped here.
//!
//! Judgments (spec/judgments.md): a fragment is kept (the docs list the spellings that merge, and a
//! fragment is not among them); a URL that does not parse is its own endpoint by its text; a command is
//! compared with its arguments as a list, never joined into one string; the transport type is not
//! part of the key (the docs compare "the same URL or command").

use super::mcp_config::is_remote_mcp_server;
use crate::js::{JsValue, json_stringify, string_of, url};

fn url_key(text: &str) -> String {
    let Some(parsed) = url::parse(text) else {
        return format!("url:{text}");
    };
    // `url.pathname = url.pathname.slice(0, -1)`: the href ends with the path, the query and the
    // fragment; an opaque path (one not starting with `/`) ignores the setter.
    let path = parsed.pathname.as_str();
    let tail_len = path.len() + parsed.search.len() + parsed.hash.len();
    let shortened = path.len() > 1 && path.starts_with('/') && path.ends_with('/');
    let href = match parsed.href.len().checked_sub(tail_len) {
        Some(at) if shortened => {
            let head = parsed.href.get(..at).unwrap_or_default();
            let trimmed = path.get(..path.len() - 1).unwrap_or_default();
            format!("{head}{trimmed}{}{}", parsed.search, parsed.hash)
        }
        _ => parsed.href.clone(),
    };
    format!("url:{href}")
}

fn command_key(command: &JsValue, args: Option<&JsValue>) -> String {
    let mut list = vec![command.clone()];
    if let Some(JsValue::Array(args)) = args {
        list.extend(args.iter().cloned());
    }
    let text = json_stringify(&JsValue::Array(list)).unwrap_or_default();
    format!("command:{text}")
}

/// A key equal for two servers exactly when Claude Code counts them as one endpoint.
#[must_use]
pub fn mcp_endpoint_key(server: &JsValue) -> String {
    let member = |key: &str| server.get(key).cloned().unwrap_or_default();
    if is_remote_mcp_server(server) {
        url_key(&string_of(&member("url")))
    } else {
        command_key(&member("command"), server.get("args"))
    }
}

crate::registry::exports! {
    "format/mcp-endpoint" {
        "mcpEndpointKey" => ok |server: JsValue| mcp_endpoint_key(&server);
    }
}
