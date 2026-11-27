//! Port of `src/settings/general/reach.ts`, switched.
//!
//! Whose sessions a settings file reaches (settings.md, "Settings files and who they affect" and
//! "Compare the scope of each settings file"). The file kinds are the four a person can name;
//! `flag` and the rest are not files of a machine.

use serde::{Deserialize, Serialize};

/// The four scopes a settings file has.
pub const FILE_SCOPES: [FileScope; 4] = [
    FileScope::User,
    FileScope::Project,
    FileScope::Local,
    FileScope::Managed,
];

/// One of `FILE_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileScope {
    /// The user's file.
    User,
    /// The project's shared file.
    Project,
    /// The project's local file.
    Local,
    /// A managed file.
    Managed,
}

/// Whether a scope a host holds as a string is one of the four file scopes.
#[must_use]
pub fn is_file_scope(scope: &str) -> bool {
    FILE_SCOPES.iter().any(|one| one.name() == scope)
}

impl FileScope {
    /// The scope's name as the package writes it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Project => "project",
            Self::Local => "local",
            Self::Managed => "managed",
        }
    }
}

/// What a host tells about the file besides its scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReachFacts {
    /// The project file is committed to version control (a teammate's clone and a cloud session
    /// then get it).
    pub committed: bool,
    /// The managed settings come from the claude.ai console (server-managed), not a file or an MDM
    /// policy.
    pub server_managed: bool,
}

/// Where on your own machine a file's settings apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum YourMachine {
    /// Every project.
    #[serde(rename = "every project")]
    EveryProject,
    /// The one project the file sits in.
    #[serde(rename = "this project")]
    ThisProject,
}

/// Who a setting saved in a file reaches.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reach {
    /// On your own machine: every project, or the one project the file sits in.
    pub your_machine: YourMachine,
    /// A teammate's machine, with their own clone of the project.
    pub teammate: bool,
    /// A cloud session on the project.
    pub cloud_session: bool,
}

/// Where a setting saved in a file of this scope applies.
#[must_use]
pub const fn reach_of(scope: FileScope, facts: ReachFacts) -> Reach {
    match scope {
        // "every project on your machine, and nothing on your teammate's or in the cloud session"
        FileScope::User => Reach {
            your_machine: YourMachine::EveryProject,
            teammate: false,
            cloud_session: false,
        },
        // "reaches your teammate's clone and the cloud session only if you commit the file"
        FileScope::Project => Reach {
            your_machine: YourMachine::ThisProject,
            teammate: facts.committed,
            cloud_session: facts.committed,
        },
        // "your acme-app/ only"
        FileScope::Local => Reach {
            your_machine: YourMachine::ThisProject,
            teammate: false,
            cloud_session: false,
        },
        // "every project on every machine your organization deploys it to ... Only server-managed
        // settings reach the cloud session"
        FileScope::Managed => Reach {
            your_machine: YourMachine::EveryProject,
            teammate: true,
            cloud_session: facts.server_managed,
        },
    }
}

crate::registry::exports! {
    "settings/general/reach" {
        "FILE_SCOPES" => ok || FILE_SCOPES;
        "isFileScope" => ok |scope: String| is_file_scope(&scope);
        "reachOf" => ok |scope: FileScope, facts: ReachFacts| reach_of(scope, facts);
    }
}
