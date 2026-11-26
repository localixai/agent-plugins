//! What `claude plugin validate` reports and the small readers its checkers share
//! (`src/format/validate/` in the npm package).

pub mod finding;
pub mod mcp_remote_checks;
pub mod path_claims;
pub mod plugin_name;
pub mod plugin_paths;
pub mod received;
pub mod shell_command;
pub mod unknown_fields;
