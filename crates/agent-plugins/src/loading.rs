//! Layer 3 (loading): how a plugin folder becomes components (`src/loading/` in the npm package).

pub mod background_trust;
pub mod bin;
pub mod channels;
pub mod command_debug;
pub mod core;
pub mod declared_shape;
pub mod ignored_folder;
pub mod local_settings_trust;
pub mod manifest_gate;
pub mod merge_hooks;
pub mod plugin_debug;
pub mod plugin_folder;
pub mod plugin_scaffold;
pub mod ports;
pub mod repository_trust;
pub mod resolve;
pub mod trust_gates;
pub mod validate;
pub mod workspace_trust;
