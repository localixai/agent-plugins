//! Layer 4: the plugin settings: which plugins are on, their ids, the claude.ai sync and the trust
//! warning (`src/settings/plugins/` in the npm package).

pub mod enabled_plugins;
pub mod mod_order;
pub mod plugin_config;
pub mod plugin_id;
pub mod policy;
pub mod suggestions;
pub mod sync_plugins;
pub mod trust_warning;
