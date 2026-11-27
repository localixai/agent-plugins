//! Port of `src/loading/background-trust.ts`, switched.
//!
//! The trust check before a background session starts or restarts (permissions.md, "Project allow
//! rules and workspace trust"; errors.md, "Workspace not trusted when dispatching a background
//! session"; Agent Plugins Extended §13.15 item 4). Pure over what the host knows: whether the
//! directory exists, whether it is trusted (`workspaceTrustState`), and whether a dialog can appear
//! (a terminal in the session's own directory).

use serde::{Deserialize, Serialize};

/// What the host knows about the session's directory.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four facts the docs' check reads"
)]
pub struct BackgroundTrustFacts {
    /// The session's directory, as the message names it.
    pub directory: String,
    /// The directory was found on disk.
    pub resolved: bool,
    /// It is the home directory, whose trust is never saved.
    pub is_home: bool,
    /// The dialog was accepted for its key (or a covering parent) and is on record.
    pub trusted: bool,
    /// A terminal in the session's own directory, where the dialog can appear.
    pub can_show_dialog: bool,
}

/// `start`: go ahead. `dialog`: show the trust dialog first and start once it is accepted. `refuse`: exit with `message`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "lowercase")]
pub enum BackgroundTrustVerdict {
    /// Go ahead.
    Start,
    /// Show the dialog first.
    Dialog,
    /// Exit with `message`.
    Refuse {
        /// What the command prints.
        message: String,
    },
}

fn refuse(message: String) -> BackgroundTrustVerdict {
    BackgroundTrustVerdict::Refuse { message }
}

/// What happens before a background session runs in `facts.directory`. A directory not found on
/// disk is refused (`<path> could not be resolved on disk`); a trusted one starts, the home directory
/// never counting as trusted from an earlier session; else a terminal there shows the dialog, and
/// where none can appear the command exits (`` Workspace not trusted. Run `claude` in <path> once and
/// accept the trust prompt, then retry. ``, or `The home directory is trusted one session at a time`).
#[must_use]
pub fn background_session_trust(facts: &BackgroundTrustFacts) -> BackgroundTrustVerdict {
    if !facts.resolved {
        return refuse(format!("{} could not be resolved on disk", facts.directory));
    }
    if facts.trusted && !facts.is_home {
        return BackgroundTrustVerdict::Start;
    }
    if facts.can_show_dialog {
        return BackgroundTrustVerdict::Dialog;
    }
    refuse(if facts.is_home {
        "The home directory is trusted one session at a time".to_owned()
    } else {
        format!(
            "Workspace not trusted. Run `claude` in {} once and accept the trust prompt, then retry.",
            facts.directory
        )
    })
}

crate::registry::exports! {
    "loading/background-trust" {
        "backgroundSessionTrust" => ok |facts: BackgroundTrustFacts| background_session_trust(&facts);
    }
}
