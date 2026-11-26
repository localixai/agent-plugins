//! Layer 3 (loading): how a plugin folder becomes components (`src/loading/` in the npm package).

pub mod background_trust;
pub mod channels;
pub mod core;
pub mod merge_hooks;
pub mod plugin_debug;
pub mod plugin_folder;
pub mod plugin_scaffold;
pub mod resolve;
pub mod trust_gates;
pub mod validate;
