//! The core's hook selection rules (`src/hooks/select/` in the npm package).

pub mod dedupe;
pub mod event_catalog;
pub mod gates;
pub mod lock_gates;
pub mod matcher_targets;
pub mod matcher_values;
pub mod mcp_client;
pub mod model_switch;
pub mod once;
pub mod watch_list;
