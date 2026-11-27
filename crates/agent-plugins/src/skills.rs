//! Layer 3 (skills): what a skill is and how it is invoked (`src/skills/` in the npm package).

pub mod arguments;
pub mod execution;
pub mod expand_policy;
pub mod file_references;
pub mod folder_line;
pub mod fork;
pub mod hook_limits;
pub mod invocation;
pub mod lifecycle;
pub mod mention;
pub mod scan;
pub mod shell_refusal;
pub mod shell_result;
pub mod shell_run;
pub mod shell_scan;
pub mod skill_rule;
pub mod stack;
pub mod visibility;
