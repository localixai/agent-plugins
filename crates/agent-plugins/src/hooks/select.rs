//! The core's hook selection rules (`src/hooks/select/` in the npm package).

pub mod dedupe;
pub mod event_catalog;
pub mod gates;
pub mod if_guard;
pub mod lock_gates;
pub mod matcher;
pub mod matcher_targets;
pub mod matcher_values;
pub mod mcp_client;
pub mod mcp_tools;
pub mod model_switch;
pub mod once;
#[expect(
    clippy::module_inception,
    reason = "the port of `src/hooks/select/select.ts` sits at its source's path"
)]
pub mod select;
pub mod watch_list;
