//! Layer 3: what installing, updating and removing plugins from marketplaces decides, as pure
//! plans and messages a host runs (`src/marketplace/` in the npm package).

pub mod archive_scan;
pub mod command_mode;
pub mod command_output;
pub mod folder_collision;
pub mod git_transport;
pub mod header_map;
pub mod header_policy;
pub mod headless_report;
pub mod http_download;
pub mod install_failures;
pub mod install_hosts;
pub mod lfs_pointer;
pub mod missing_plugin_diagnosis;
pub mod name_conflicts;
pub mod plugin_content;
pub mod plugin_footprint;
pub mod plugin_hints;
pub mod reload_summary;
pub mod remote_plugins;
pub mod seed_registry;
pub mod session_origins;
pub mod submodules;
pub mod telemetry_names;
pub mod types;
pub mod version_hash;
