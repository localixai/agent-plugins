//! Layer 4: the managed settings, what they deliver, when and what they refuse
//! (`src/settings/managed/` in the npm package).

pub mod approval;
pub mod combine;
pub mod cross_source;
pub mod delivery;
pub mod limits;
pub mod login_keys;
pub mod sideload;
pub mod startup_failure;
pub mod wsl_chain;
