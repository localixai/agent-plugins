//! The `claude` command line as pure plans over the host's facts (`src/cli/` in the npm package).

pub mod background_session;
pub mod clean_configuration;
pub mod command_acceptance;
pub mod command_table;
pub mod flags;
pub mod install_refusals;
pub mod lfs_pointer_line;
pub mod marketplace_list;
pub mod marketplace_update;
pub mod plugin_details;
pub mod plugin_list;
pub mod plugin_ref;
pub mod records_refusal;
pub mod result;
pub mod uninstall_data;
pub mod uninstall_flags;
pub mod uninstall_refusal;
pub mod validate_advice;
pub mod validate_run;
