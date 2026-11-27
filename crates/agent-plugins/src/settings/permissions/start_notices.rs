//! Port of `src/settings/permissions/start-notices.ts`, switched.

use serde::Deserialize;

/// The notice a terminal session shows under the HIPAA configuration, where the built-in `auto` default
/// does not apply (permission-modes.md, "Permission modes with the HIPAA configuration", verbatim).
pub const HIPAA_AUTO_NOTICE: &str =
    "Auto mode isn't the default for your organization · Shift+Tab to switch";

/// What decides the notices of a start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
pub struct NoticeFacts {
    /// The HIPAA configuration is applied and the session is eligible for it.
    pub hipaa: bool,
    /// A terminal session.
    pub terminal: bool,
    /// Nothing chose the mode: no flag and no `defaultMode`.
    #[serde(rename = "builtIn")]
    pub built_in: bool,
}

/// What a session shows at its start, as text: under the HIPAA configuration "A terminal session also
/// shows" the notice when nothing else chose the mode, "and the VS Code extension shows no notice". The
/// first-time notice of the built-in `auto` default has no quoted text and needs a record of earlier
/// starts, so the host shows it (spec/judgments.md).
#[must_use]
pub fn start_notices(facts: &NoticeFacts) -> Vec<&'static str> {
    if facts.hipaa && facts.terminal && facts.built_in {
        vec![HIPAA_AUTO_NOTICE]
    } else {
        Vec::new()
    }
}

crate::registry::exports! {
    "settings/permissions/start-notices" {
        "HIPAA_AUTO_NOTICE" => ok || HIPAA_AUTO_NOTICE;
    }
}
