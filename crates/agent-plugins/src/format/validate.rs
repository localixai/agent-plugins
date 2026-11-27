//! What `claude plugin validate` reports and the small readers its checkers share
//! (`src/format/validate/` in the npm package).

pub mod component_frontmatter;
pub mod finding;
pub mod hook_handlers;
pub mod hooks_check;
pub mod inline_sources;
pub mod mcp_check;
pub mod mcp_remote_checks;
pub mod path_claims;
pub mod plugin_fields;
pub mod plugin_manifest;
pub mod plugin_name;
pub mod plugin_paths;
pub mod received;
pub mod shell_command;
pub mod unknown_fields;
pub mod zod_findings;
