//! The core's hook output readers (`src/hooks/output/` in the npm package).

pub mod background;
pub mod caps;
pub mod defer;
pub mod fields;
pub mod finalize;
pub mod merge_rules;
pub mod messages;
pub mod parse;
pub mod permission_request_outcome;
pub mod permission_update;
pub mod pre_tool_use_outcome;
pub mod reminder_tags;
pub mod schemas;
pub mod schemas_common;
pub mod schemas_elicitation;
pub mod schemas_misc;
pub mod schemas_session;
pub mod schemas_tool;
pub mod terminal_sequence;
pub mod text_cap;
pub mod tool_output;
pub mod tool_shape;
pub mod tool_shapes;
pub mod watch_paths;
pub mod worktree_path;
