//! Port of `src/settings/general/file-locations.ts`, switched.
//!
//! Where the settings files are and who makes them (settings.md, "Settings files and who they
//! affect", "Find or create your settings files", "Keep personal settings out of a repository" and
//! "Where Claude Code keeps the local file in a git repository"). Pure data and decisions: the host
//! reads the folders, owns the paths' spelling (`joinPath`) and writes the files.

use serde::{Deserialize, Serialize};

use crate::format::config_directory::{Join, config_directory_of};
use crate::format::settings_files::SETTINGS_FILE;
use crate::registry::{Callback, Port};

/// Who makes a file when it is absent: the docs' "create them yourself, or let Claude Code create them".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum FileMaker {
    /// The organization deploys it.
    Organization,
    /// The person makes it.
    You,
    /// Claude Code writes it.
    ClaudeCode,
}

/// Where a file is documented, who makes it and whether the person edits it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileOrigin {
    /// The path as the docs write it, from the home folder (`~/.claude`) or the project folder (bare `.claude`).
    pub documented_path: &'static str,
    /// Who makes it.
    pub made_by: &'static [FileMaker],
    /// The person can edit it. A managed file is deployed: "You don't create or edit it".
    pub you_edit: bool,
}

/// `FILE_ORIGINS`, one member per file scope.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct FileOrigins {
    /// The user file.
    pub user: FileOrigin,
    /// The shared project file.
    pub project: FileOrigin,
    /// The local file.
    pub local: FileOrigin,
    /// A managed file.
    pub managed: FileOrigin,
}

/// `managed-settings.json` and the other managed sources have no one path (managed-settings.md, "Delivery mechanisms").
pub const FILE_ORIGINS: FileOrigins = FileOrigins {
    user: FileOrigin {
        documented_path: SETTINGS_FILE.user,
        made_by: &[FileMaker::You, FileMaker::ClaudeCode],
        you_edit: true,
    },
    project: FileOrigin {
        documented_path: SETTINGS_FILE.project,
        // "a project that already uses Claude Code may have one committed. If not, create it"
        made_by: &[FileMaker::You],
        you_edit: true,
    },
    local: FileOrigin {
        documented_path: SETTINGS_FILE.local,
        made_by: &[FileMaker::You, FileMaker::ClaudeCode],
        you_edit: true,
    },
    managed: FileOrigin {
        documented_path: "managed-settings.json",
        made_by: &[FileMaker::Organization],
        you_edit: false,
    },
};

/// What makes Claude Code write a settings file, and which one (settings.md, "Find or create your
/// settings files", "Use the /config menu"; model-config.md, "Setting your model"). Installing it
/// creates none: it writes `~/.claude/settings.json` "the first time you change an option in the
/// `/config` menu that it stores in user settings, such as the theme", and `.claude/settings.local.json`
/// "the first time you give a standing approval on a permission prompt". The `/config` options that
/// are global config keys go to `~/.claude.json`, which is no settings file.
pub const WRITE_TRIGGERS: [&str; 4] = [
    "config-option",
    "config-option-kept-local",
    "permission-standing-approval",
    "model-default",
];

/// The files Claude Code writes: only the user file and the local file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WrittenScope {
    /// The user file.
    User,
    /// The local file.
    Local,
}

/// `WRITE_TARGET`, one member per trigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct WriteTarget {
    /// `config-option`.
    pub config_option: WrittenScope,
    /// `config-option-kept-local`.
    pub config_option_kept_local: WrittenScope,
    /// `permission-standing-approval`.
    pub permission_standing_approval: WrittenScope,
    /// `model-default`.
    pub model_default: WrittenScope,
}

/// The file each trigger writes.
pub const WRITE_TARGET: WriteTarget = WriteTarget {
    config_option: WrittenScope::User,
    // "A few options, such as Show tips: .claude/settings.local.json"
    config_option_kept_local: WrittenScope::Local,
    permission_standing_approval: WrittenScope::Local,
    // "`/model` saves your choice as the default for new sessions by writing the `model` field in your user settings"
    model_default: WrittenScope::User,
};

/// The host's facts for the files it joins paths for. It holds `joinPath`, so it crosses as a port.
pub struct FileFacts {
    /// `~`.
    pub home_dir: String,
    /// The `CLAUDE_CONFIG_DIR` the session starts with; empty counts as unset.
    pub config_dir_env: Option<String>,
    /// The project folder.
    pub project_dir: String,
    /// Joins a base and a relative path the way the host writes paths.
    pub join_path: Box<dyn Join>,
}

impl FileFacts {
    /// The facts the host's record holds, each member read.
    pub async fn read(facts: Port) -> Self {
        let join_path: Callback = facts.member("joinPath").await;
        Self {
            home_dir: facts.member("homeDir").await,
            config_dir_env: facts.member("configDirEnv").await,
            project_dir: facts.member("projectDir").await,
            join_path: Box::new(join_path),
        }
    }
}

/// The folder holding the home-directory files: `CLAUDE_CONFIG_DIR` when set, else `~/.claude`
/// ("Claude Code then stores your settings, session history, and plugins there instead").
pub async fn config_dir_of(facts: &FileFacts) -> String {
    config_directory_of(
        &facts.home_dir,
        facts.config_dir_env.as_deref(),
        facts.join_path.as_ref(),
    )
    .await
}

/// The user file, `~/.claude/settings.json` or the one under `CLAUDE_CONFIG_DIR`.
pub async fn user_settings_file(facts: &FileFacts) -> String {
    let directory = config_dir_of(facts).await;
    facts
        .join_path
        .join(&directory, SETTINGS_FILE.user_name)
        .await
}

/// The shared project file, in the project folder (the session's primary working directory).
pub async fn project_settings_file(facts: &FileFacts) -> String {
    facts
        .join_path
        .join(&facts.project_dir, SETTINGS_FILE.project)
        .await
}

/// What the host found about the git repository the session starts in.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryFacts {
    /// The repository root; in a worktree, the main checkout's root.
    pub root: String,
    /// The root, its `.git` and its `.claude` entry are all owned by the user.
    pub owned_by_user: bool,
}

/// Where the local file may be placed.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalPlacementFacts {
    /// `~`.
    pub home_dir: String,
    /// A Windows host.
    pub windows: bool,
    /// `undefined` outside a git repository.
    #[serde(default)]
    pub repository: Option<RepositoryFacts>,
}

/// The folder whose `.claude/settings.local.json` is the local file: the repository root. The file
/// stays with `.claude/settings.json` (`undefined` here) "outside a git repository, when the
/// repository root is your home directory, on Windows, or when the repository root or its `.git` or
/// `.claude` entry isn't owned by your user".
#[must_use]
pub fn local_settings_root(facts: &LocalPlacementFacts) -> Option<String> {
    let repository = facts.repository.as_ref()?;
    if facts.windows || !repository.owned_by_user {
        return None;
    }
    if repository.root == facts.home_dir {
        None
    } else {
        Some(repository.root.clone())
    }
}

/// The local files to read, the one that applies first first. Before v2.1.211 the file was kept in
/// the starting directory, and Claude Code "still reads a file an earlier version left there
/// alongside the root file; where both set the same key, the root's value applies, and permission
/// rules from both files apply".
pub async fn local_settings_files(
    facts: &LocalPlacementFacts,
    project_dir: &str,
    join_path: &dyn Join,
) -> Vec<String> {
    let root = local_settings_root(facts);
    let own = join_path.join(project_dir, SETTINGS_FILE.local).await;
    match root {
        Some(root) if root != project_dir => {
            vec![join_path.join(&root, SETTINGS_FILE.local).await, own]
        }
        _ => vec![own],
    }
}

/// The line Claude Code adds to the global git excludes (settings.md, "Keep personal settings out of a repository"):
/// `**/` and `SETTINGS_FILE.local`.
pub const LOCAL_FILE_IGNORE_ENTRY: &str = "**/.claude/settings.local.json";

/// `/^[A-Za-z]:[\\/]/` or a leading `/`.
fn is_absolute(path: &str) -> bool {
    let mut chars = path.chars();
    path.starts_with('/')
        || (chars
            .next()
            .is_some_and(|letter| letter.is_ascii_alphabetic())
            && chars.next() == Some(':')
            && chars
                .next()
                .is_some_and(|slash| slash == '\\' || slash == '/'))
}

/// The facts the global excludes file is found from.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitExcludeFacts {
    /// Git's `core.excludesFile` from the global config, as written; `undefined` when unset.
    #[serde(default)]
    pub core_excludes_file: Option<String>,
    /// `XDG_CONFIG_HOME`.
    #[serde(default)]
    pub xdg_config_home: Option<String>,
    /// `~`.
    pub home_dir: String,
}

/// The global git excludes file: `core.excludesFile` "when your global git config sets it to an
/// absolute or `~`-prefixed path; otherwise it's `$XDG_CONFIG_HOME/git/ignore`, or
/// `~/.config/git/ignore` when `XDG_CONFIG_HOME` is unset".
pub async fn global_git_excludes_file(facts: &GitExcludeFacts, join_path: &dyn Join) -> String {
    if let Some(configured) = facts.core_excludes_file.as_deref() {
        if is_absolute(configured) {
            return configured.to_owned();
        }
        if let Some(rest) = configured.strip_prefix("~/") {
            return join_path.join(&facts.home_dir, rest).await;
        }
    }
    match facts.xdg_config_home.as_deref() {
        None | Some("") => join_path.join(&facts.home_dir, ".config/git/ignore").await,
        Some(xdg) => join_path.join(xdg, "git/ignore").await,
    }
}

/// What the host knows about the first write of the local file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FirstWriteFacts {
    /// The session is in a git repository.
    pub in_git_repository: bool,
    /// The repository already ignores the file (a `.gitignore` or the global excludes).
    pub already_ignored: bool,
    /// Claude Code writes the file now, rather than you having made it by hand.
    pub claude_code_writes: bool,
}

/// Whether Claude Code adds `LOCAL_FILE_IGNORE_ENTRY` now: "The first time Claude Code writes the
/// file in a git repository that doesn't already ignore it". A file you made by hand and Claude Code
/// has not written is yours to ignore.
#[must_use]
pub const fn should_add_git_exclude(facts: FirstWriteFacts) -> bool {
    facts.claude_code_writes && facts.in_git_repository && !facts.already_ignored
}

/// `localSettingsFiles`' facts as the host's record holds them, with its `joinPath`.
async fn local_files_of(facts: Port) -> Vec<String> {
    let join_path: Callback = facts.member("joinPath").await;
    let placement = LocalPlacementFacts {
        home_dir: facts.member("homeDir").await,
        windows: facts.member("windows").await,
        repository: facts.member("repository").await,
    };
    let project_dir: String = facts.member("projectDir").await;
    local_settings_files(&placement, &project_dir, &join_path).await
}

/// `globalGitExcludesFile`'s facts as the host's record holds them, with its `joinPath`.
async fn git_excludes_of(facts: Port) -> String {
    let join_path: Callback = facts.member("joinPath").await;
    let read = GitExcludeFacts {
        core_excludes_file: facts.member("coreExcludesFile").await,
        xdg_config_home: facts.member("xdgConfigHome").await,
        home_dir: facts.member("homeDir").await,
    };
    global_git_excludes_file(&read, &join_path).await
}

crate::registry::exports! {
    "settings/general/file-locations" {
        "FILE_ORIGINS" => ok || FILE_ORIGINS;
        "WRITE_TRIGGERS" => ok || WRITE_TRIGGERS;
        "WRITE_TARGET" => ok || WRITE_TARGET;
        "configDirOf" => async ok |facts: Port| config_dir_of(&FileFacts::read(facts).await).await;
        "userSettingsFile" => async ok |facts: Port|
            user_settings_file(&FileFacts::read(facts).await).await;
        "projectSettingsFile" => async ok |facts: Port|
            project_settings_file(&FileFacts::read(facts).await).await;
        "localSettingsRoot" => ok |facts: LocalPlacementFacts| local_settings_root(&facts);
        "localSettingsFiles" => async ok |facts: Port| local_files_of(facts).await;
        "LOCAL_FILE_IGNORE_ENTRY" => ok || LOCAL_FILE_IGNORE_ENTRY;
        "globalGitExcludesFile" => async ok |facts: Port| git_excludes_of(facts).await;
        "shouldAddGitExclude" => ok |facts: FirstWriteFacts| should_add_git_exclude(facts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::now;

    fn join(base: &str, relative: &str) -> String {
        format!("{base}/{relative}")
    }

    #[test]
    fn a_drive_letter_path_is_absolute() {
        assert!(is_absolute("C:\\ignore"));
        assert!(is_absolute("/etc/ignore"));
        assert!(!is_absolute("C:ignore"));
        let facts = GitExcludeFacts {
            core_excludes_file: Some("~/.gitignore".to_owned()),
            xdg_config_home: None,
            home_dir: "/home/a".to_owned(),
        };
        assert_eq!(
            now(global_git_excludes_file(&facts, &join)),
            Some("/home/a/.gitignore".to_owned())
        );
    }

    #[test]
    fn the_ignore_entry_names_the_local_file() {
        assert_eq!(
            LOCAL_FILE_IGNORE_ENTRY,
            format!("**/{}", SETTINGS_FILE.local)
        );
    }
}
