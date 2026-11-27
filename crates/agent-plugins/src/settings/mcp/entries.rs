//! Port of `src/settings/mcp/entries.ts`, switched.
//!
//! The entries of `allowedMcpServers` and `deniedMcpServers` (managed-mcp.md, "Match servers by
//! URL, command, or name"): "an object with a single key that identifies servers by their URL,
//! their command, or their name". The two lists differ in one rule, the `serverName` text: the
//! allowlist limits it to "letters, numbers, hyphens, and underscores", the denylist takes "any
//! non-empty string without leading or trailing whitespace", so a claude.ai connector can be
//! blocked by its display name.
//!
//! Invalid entries follow managed-settings.md, "Keys that fail closed": "an individual invalid
//! entry is stripped and the valid subset is enforced". A value that is no list: unreadable in the
//! policy tier an allowlist is enforced as an empty one ("so no MCP servers that users add are
//! admitted"); a denylist is dropped with a warning ("denying every server would block servers the
//! policy never named"); in any other source the value is dropped.

use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue, property_order, trim};
use crate::protocol::is_record::is_record;
use crate::settings::repair::support::{
    RepairAt, Repaired, dropped, failed_closed, kept, strip_invalid_now,
};

/// The kinds of entry.
pub const MCP_ENTRY_KINDS: [&str; 3] = ["serverName", "serverCommand", "serverUrl"];

/// One entry of either list.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum McpEntry {
    /// A server by its name.
    #[serde(rename = "serverName")]
    ServerName(String),
    /// A server by its command: the program and its arguments.
    #[serde(rename = "serverCommand")]
    ServerCommand(Vec<String>),
    /// A server by its URL.
    #[serde(rename = "serverUrl")]
    ServerUrl(String),
}

/// The two lists.
pub const MCP_LISTS: [&str; 2] = ["allowed", "denied"];

/// Which list an entry is read for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum McpList {
    /// `allowedMcpServers`.
    Allowed,
    /// `deniedMcpServers`.
    Denied,
}

struct NameRule {
    accepts: fn(&str) -> bool,
    describes: &'static str,
}

const fn name_rule(list: McpList) -> NameRule {
    match list {
        McpList::Allowed => NameRule {
            accepts: |name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            },
            describes: "letters, numbers, hyphens and underscores",
        },
        McpList::Denied => NameRule {
            accepts: |name| !name.is_empty() && trim(name) == name,
            describes: "a non-empty text without leading or trailing whitespace",
        },
    }
}

fn read_kind(kind: &str, value: &JsValue, list: McpList) -> Option<McpEntry> {
    match kind {
        "serverName" => value
            .as_str()
            .filter(|name| (name_rule(list).accepts)(name))
            .map(|name| McpEntry::ServerName(name.to_owned())),
        // "@minItems 1" in the SDK's type: a command is the program and its arguments.
        "serverCommand" => {
            let items = value.as_array().filter(|items| !items.is_empty())?;
            items
                .iter()
                .map(|item| item.as_str().map(ToOwned::to_owned))
                .collect::<Option<Vec<_>>>()
                .map(McpEntry::ServerCommand)
        }
        "serverUrl" => value
            .as_str()
            .map(|url| McpEntry::ServerUrl(url.to_owned())),
        _ => None,
    }
}

/// The entry `raw` is, or `None` when it is not an object with exactly one valid key.
#[must_use]
pub fn read_mcp_entry(raw: &JsValue, list: McpList) -> Option<McpEntry> {
    if !is_record(raw) {
        return None;
    }
    let members = raw.as_object()?;
    let keys = property_order(members);
    let (first, value) = keys.first()?;
    let kind = MCP_ENTRY_KINDS
        .iter()
        .find(|known| **known == first.as_str())?;
    if keys.len() == 1 {
        read_kind(kind, value, list)
    } else {
        None
    }
}

/// The valid entries of a list value, in order; what is no list has none.
#[must_use]
pub fn read_mcp_entries(value: &JsValue, list: McpList) -> Vec<McpEntry> {
    value.as_array().map_or_else(Vec::new, |items| {
        items
            .iter()
            .filter_map(|raw| read_mcp_entry(raw, list))
            .collect()
    })
}

fn reason(list: McpList) -> String {
    format!(
        "Expected an object with one of serverName ({}), serverCommand (a non-empty list of texts) or serverUrl (a text)",
        name_rule(list).describes
    )
}

/// What a value that is no list becomes, per list.
fn not_a_list(list: McpList, at: &RepairAt) -> Repaired {
    match list {
        McpList::Allowed if at.closed => failed_closed(
            at,
            JsValue::Array(Vec::new()),
            "an empty allowlist: no server a user adds is admitted",
        ),
        McpList::Allowed | McpList::Denied => {
            dropped(at, &format!("\"{}\" must be a list of entries", at.key))
        }
    }
}

/// The one entry as the settings file writes it, so the repaired value keeps the SDK's shape.
fn written(entry: McpEntry) -> JsValue {
    let (kind, value) = match entry {
        McpEntry::ServerName(name) => ("serverName", JsValue::String(name)),
        McpEntry::ServerCommand(command) => (
            "serverCommand",
            JsValue::Array(command.into_iter().map(JsValue::String).collect()),
        ),
        McpEntry::ServerUrl(url) => ("serverUrl", JsValue::String(url)),
    };
    JsValue::Object(JsObject::from([(kind.to_owned(), value)]))
}

fn repair_of(list: McpList, value: &JsValue, at: &RepairAt) -> Repaired {
    let Some(items) = value.as_array() else {
        return not_a_list(list, at);
    };
    let stripped = strip_invalid_now(
        items,
        at,
        |raw| read_mcp_entry(raw, list).map(written),
        &reason(list),
    );
    kept(JsValue::Array(stripped.entries), stripped.issues)
}

/// The repair of `allowedMcpServers`.
#[must_use]
pub fn allowed_mcp_servers(value: &JsValue, at: &RepairAt) -> Repaired {
    repair_of(McpList::Allowed, value, at)
}

/// The repair of `deniedMcpServers`.
#[must_use]
pub fn denied_mcp_servers(value: &JsValue, at: &RepairAt) -> Repaired {
    repair_of(McpList::Denied, value, at)
}

crate::registry::exports! {
    "settings/mcp/entries" {
        "MCP_ENTRY_KINDS" => ok || MCP_ENTRY_KINDS;
        "MCP_LISTS" => ok || MCP_LISTS;
        "readMcpEntry" => ok |raw: JsValue, list: McpList| read_mcp_entry(&raw, list);
        "readMcpEntries" => ok |value: JsValue, list: McpList| read_mcp_entries(&value, list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_read_by_its_list() {
        let raw = |name: &str| {
            JsValue::Object(JsObject::from([(
                "serverName".to_owned(),
                JsValue::String(name.to_owned()),
            )]))
        };
        assert_eq!(read_mcp_entry(&raw("a b"), McpList::Allowed), None);
        assert_eq!(
            read_mcp_entry(&raw("a b"), McpList::Denied),
            Some(McpEntry::ServerName("a b".to_owned()))
        );
        assert_eq!(read_mcp_entry(&raw(" a"), McpList::Denied), None);
    }
}
