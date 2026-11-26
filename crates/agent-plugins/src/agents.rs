//! Layer 2 (agents): what a subagent is and runs with (`src/agents/` in the npm package).

pub mod builtin_model;
pub mod catalog;
pub mod handback_denial;
pub mod hooks;
pub mod launch;
pub mod launch_model;
pub mod mcp_scope;
pub mod memory;
pub mod model_choice;
pub mod model_value;
pub mod permission_baseline;
pub mod permission_mode;
pub mod preload_skills;
pub mod prompt_surface;
pub mod subagent_permission;
pub mod teammate;
pub mod tool_filters;
pub mod tool_list;
pub mod workflow;
