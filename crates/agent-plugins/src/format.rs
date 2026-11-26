//! Layer 2: pure readers and validators of plugin files and the small shared owners they use
//! (`src/format/` in the npm package).

pub mod canonical_json;
pub mod code_points;
pub mod config_directory;
pub mod core;
pub mod defined;
pub mod env_flag;
pub mod env_number;
pub mod frontmatter;
pub mod frontmatter_values;
pub mod frontmatter_write;
pub mod helper_credentials;
pub mod helper_headers;
pub mod host_errors;
pub mod issues;
pub mod link_rule;
pub mod listing;
pub mod marketplace;
pub mod mcp_schema_check;
pub mod mcp_tool;
pub mod messages;
pub mod model_id;
pub mod names;
pub mod path_text;
pub mod permission_rule_shape;
pub mod read_json;
pub mod remote_mcp_credentials;
pub mod theme;
pub mod theme_tokens;
pub mod token_estimate;
pub mod utf8;
pub mod validate;
pub mod versions;
pub mod wildcard;
