//! Port of `src/settings/mcp/lists.ts`, switched.

use serde::Serialize;

use crate::registry::Port;
use crate::settings::mcp::entries::{McpEntry, McpList, read_mcp_entries};
use crate::settings::view::PolicyView;

/// The entries of an allowlist, by the key they use: a server is matched against the entries of
/// its own kind.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(missing_docs, reason = "each field holds the entries of its kind")]
pub struct McpEntriesByKind {
    pub server_name: Vec<McpEntry>,
    pub server_command: Vec<McpEntry>,
    pub server_url: Vec<McpEntry>,
}

/// The two lists.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct McpLists {
    /// The denylist: every file's entries, "regardless" of any lock.
    pub denied: Vec<McpEntry>,
    /// The allowlist in force; `None` when no source sets one, an empty one when it is set empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed: Option<McpEntriesByKind>,
}

fn group_by_kind(entries: Vec<McpEntry>) -> McpEntriesByKind {
    let mut groups = McpEntriesByKind::default();
    for entry in entries {
        match entry {
            McpEntry::ServerName(_) => groups.server_name.push(entry),
            McpEntry::ServerCommand(_) => groups.server_command.push(entry),
            McpEntry::ServerUrl(_) => groups.server_url.push(entry),
        }
    }
    groups
}

/// The two lists over the view. The allowlist in force is the key row's
/// (`KEY_SPECS.allowedMcpServers`): without `allowManagedMcpServersOnly` "allowlists from every
/// settings scope merge", a managed list blocking the parent's; under the lock only the list "from
/// the highest-ranked admin source that sets one" counts, else the parent's (managed-settings.md,
/// "Let an embedding host add policy").
pub async fn read_mcp_lists(view: &dyn PolicyView) -> McpLists {
    let allowed = view.get("allowedMcpServers").await;
    McpLists {
        denied: view
            .each_from_highest("deniedMcpServers")
            .await
            .iter()
            .flat_map(|each| read_mcp_entries(&each.value, McpList::Denied))
            .collect(),
        allowed: allowed.map(|allowed| group_by_kind(read_mcp_entries(&allowed, McpList::Allowed))),
    }
}

crate::registry::exports! {
    "settings/mcp/lists" {
        "readMcpLists" => async ok |view: Port| read_mcp_lists(&view).await;
    }
}
