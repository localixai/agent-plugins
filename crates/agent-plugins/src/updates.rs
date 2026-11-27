//! Layer 3: what an update pass touches and what it leaves to the user (`src/updates/` in the npm
//! package).

pub mod auto_update_precedence;
pub mod auto_update_skips;
pub mod command_source_rerun;
pub mod env_flags;
pub mod private_marketplace;
pub mod refresh_before_install;
pub mod update_targets;
