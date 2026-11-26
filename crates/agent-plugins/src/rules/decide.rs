//! What a permission check decides beside the rules: approvals, hooks, prompts nobody answers
//! (`src/rules/decide/` in the npm package).

pub mod approval_memory;
pub mod context;
pub mod hook_permission_updates;
pub mod hook_rewrite;
pub mod no_approval_surface;
pub mod sed_script;
pub mod unanswered;
pub mod write_block_messages;
