//! Port of `src/marketplace/command-mode.ts`, switched.
//!
//! The `mode` of a `command` source (plugins/marketplace-reference.md, "Copy mode and link mode").

use serde::Serialize;

use crate::format::issues::{PluginIssue, error};

/// Which `mode` Windows refuses.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LinksNeedPosix {
    /// `copy`.
    pub copy: bool,
    /// `link`.
    pub link: bool,
}

/// Which `mode` Windows refuses.
pub const LINKS_NEED_POSIX: LinksNeedPosix = LinksNeedPosix {
    copy: false,
    link: true,
};

/// "Claude Code refuses to install a link-mode plugin on Windows. Declare `"mode": "copy"` there."
#[must_use]
pub fn link_on_windows_issue() -> PluginIssue {
    error(
        "source.mode",
        "A link-mode plugin cannot be installed on Windows. Declare \"mode\": \"copy\" there",
    )
}

crate::registry::exports! {
    "marketplace/command-mode" {
        "LINKS_NEED_POSIX" => ok || LINKS_NEED_POSIX;
        "linkOnWindowsIssue" => ok link_on_windows_issue;
    }
}
