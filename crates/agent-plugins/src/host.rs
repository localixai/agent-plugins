//! The host side of a session (`src/host/` in the npm package): only the modules already in the
//! core are declared.

pub mod background_queue;
pub mod catalog;
pub mod core_servers;
pub mod core_skills;
pub mod core_view;
pub mod facts;
pub mod permission_rules;
pub mod permission_session;
pub mod plugin_dependencies;
pub mod plugin_set;
pub mod saved_approval;
pub mod sdk_output;
pub mod session_core;
pub mod session_core_dispatch;
pub mod session_core_hooks;
pub mod session_directories;
pub mod session_engine;
pub mod session_hook_scope;
pub mod session_permission_updates;
pub mod session_registry;
pub mod session_start_repeats;
pub mod state;
