//! The core's hook output readers (`src/hooks/output/` in the npm package).

pub mod background;
pub mod caps;
pub mod classifier_note;
pub mod defer;
pub mod fields;
pub mod finalize;
pub mod merge;
pub mod merge_gates;
pub mod merge_rules;
pub mod merge_simple;
pub mod messages;
pub mod permission_request_outcome;
pub mod pre_tool_use_outcome;
pub mod reminder_tags;
pub mod terminal_sequence;
pub mod text_cap;
pub mod tool_output;
pub mod tool_shape;
pub mod tool_shapes;
pub mod watch_paths;
pub mod worktree_path;
