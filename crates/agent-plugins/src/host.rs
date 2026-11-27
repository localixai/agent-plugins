//! The host side of a session (`src/host/` in the npm package): only the modules already in the
//! core are declared.

pub mod background_queue;
pub mod core_skills;
pub mod permission_rules;
pub mod permission_session;
pub mod saved_approval;
pub mod sdk_output;
pub mod session_core;
pub mod session_directories;
pub mod session_engine;
pub mod session_folder_resolution;
pub mod session_hook_scope;
pub mod session_host_skills;
pub mod session_mentions;
pub mod session_permission_updates;
pub mod session_registry;
pub mod session_skill_catalog;
pub mod session_skill_entries;
pub mod session_skill_readers;
pub mod session_start_repeats;
pub mod session_tool_check;
pub mod session_workflows;
