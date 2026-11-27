//! Port of `src/settings/plugins/trust-warning.ts`, switched.
//!
//! The trust warning `/plugin` shows before a plugin installs, with the organization's message read
//! through the host's `PolicyView`.

use crate::js::trim;
use crate::registry::Port;
use crate::settings::view::PolicyView;

/// Judgment (spec/judgments.md): the docs say the organization's text is "added" and "appended" and
/// name no separator; a blank line keeps it apart from the standard warning.
const TRUST_MESSAGE_JOINER: &str = "\n\n";
/// plugins/security.md § Trust warning before you install, verbatim.
pub const STANDARD_TRUST_WARNING: &str = "Make sure you trust a plugin before installing, updating, or using it. Anthropic does not control what MCP servers, files, or other software are included in plugins and cannot verify that they will work as intended or that they won't change. See each plugin's homepage for more information.";

/// The trust warning `/plugin` shows before a plugin installs (settings-reference.md,
/// `pluginTrustMessage`; plugins/security.md, "Trust warning before you install"). The published
/// warning is the default; a host can supply its own wording, unchanged and first.
/// Only managed settings carry the key. An empty or blank message adds nothing (a judgment).
pub async fn trust_warning(view: &dyn PolicyView, standard: &str) -> String {
    let message = view.get("pluginTrustMessage").await;
    match message.as_ref().and_then(|message| message.as_str()) {
        Some(text) if !trim(text).is_empty() => {
            format!("{standard}{TRUST_MESSAGE_JOINER}{text}")
        }
        _ => standard.to_owned(),
    }
}

crate::registry::exports! {
    "settings/plugins/trust-warning" {
        "trustWarning" => async ok |view: Port, standard: Option<String>|
            trust_warning(&view, standard.as_deref().unwrap_or(STANDARD_TRUST_WARNING)).await;
    }
}
