//! Layer 3 (loading), resolving a plugin's components (`src/loading/resolve/` in the npm package).

pub mod bundle_scope;
pub mod data_id;
pub mod field_map;
pub mod headers_helper;
pub mod headers_helper_origins;
pub mod headers_helper_run;
pub mod headers_helper_trust;
pub mod lsp_startup;
pub mod monitor_trigger;
pub mod powershell_no_program;
pub mod shell_guard;
pub mod shell_guard_hook;
pub mod shell_guard_monitor;
pub mod shell_prefix;
pub mod subprocess_scrub;
pub mod user_config;
pub mod user_config_values;
pub mod variables;
pub mod without_plugin;
