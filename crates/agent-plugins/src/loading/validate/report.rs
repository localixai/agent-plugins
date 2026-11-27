//! Port of `src/loading/validate/report.ts`, switched.

use serde::{Deserialize, Serialize};

/// What `validatePlugin` looked at: the folder's marketplace, its plugin, only its components, or nothing it knows.
pub const VALIDATION_TARGETS: [ValidationTarget; 4] = [
    ValidationTarget::Marketplace,
    ValidationTarget::Plugin,
    ValidationTarget::Components,
    ValidationTarget::None,
];

/// One of `VALIDATION_TARGETS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ValidationTarget {
    /// The folder's marketplace.
    Marketplace,
    /// Its plugin.
    Plugin,
    /// Only its components.
    Components,
    /// Nothing it knows.
    None,
}

crate::registry::exports! {
    "loading/validate/report" {
        "VALIDATION_TARGETS" => ok || VALIDATION_TARGETS;
    }
}
