//! Layer 4: the MCP policy, how provided servers match, connect and read
//! (`src/settings/mcp/` in the npm package).

pub mod allow_deny;
pub mod approvals;
pub mod candidates;
pub mod command_match;
pub mod entries;
pub mod entry_env;
pub mod lists;
pub mod load_settings;
pub mod managed;
pub mod managed_entry;
pub mod managed_file;
pub mod managed_servers;
pub mod managed_timing;
pub mod origins;
pub mod scope;
pub mod toggles;
pub mod url_pattern;
