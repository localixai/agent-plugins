//! What a permission check decides beside the rules: approvals, hooks, prompts nobody answers
//! (`src/rules/decide/` in the npm package).

pub mod approval_memory;
pub mod auto_dropped;
pub mod behavior;
pub mod block_exceptions;
pub mod classifier;
pub mod context;
pub mod critical_removal;
#[expect(
    clippy::module_inception,
    reason = "the port of `src/rules/decide/decide.ts` sits at its source's path"
)]
pub mod decide;
pub mod facts;
pub mod file_targets;
pub mod fs_operands;
pub mod hook_permission_updates;
pub mod hook_rewrite;
pub mod hooks_and_mods;
pub mod modes;
pub mod no_approval_surface;
pub mod powershell_edits;
pub mod powershell_removal;
pub mod protected_paths;
pub mod read_only_set;
pub mod redirects;
pub mod rules_stage;
pub mod scope;
pub mod sed_script;
pub mod shell_commands;
pub mod shell_words;
pub mod unanswered;
pub mod write_block_messages;
