//! Layer 4: the managed settings, what they deliver, when and what they refuse
//! (`src/settings/managed/` in the npm package).

pub mod admin_source;
pub mod applicability;
pub mod approval;
pub mod combine;
pub mod compose;
pub mod cross_source;
pub mod delivery;
pub mod drop_in;
pub mod helper_run;
pub mod limits;
pub mod load;
pub mod login_keys;
pub mod parent;
pub mod policy_helper;
pub mod sideload;
pub mod startup_failure;
pub mod wsl_chain;
