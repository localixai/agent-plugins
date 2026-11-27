//! Port of `src/settings/locations.ts`, switched.
//!
//! Where Claude Code looks for the settings files and for output styles (settings.md, "Where Claude
//! Code keeps the local file in a git repository"; permissions.md, "Where output styles, hooks, and
//! local settings load from"). Pure over paths the host knows: the kit reads no file and joins paths
//! through the host's own `join` (`Locations.joinPath`).

use serde::Serialize;

use crate::format::config_directory::{Join, config_directory_of};
use crate::format::settings_files::SETTINGS_FILE;
use crate::registry::{Callback, Port};
use crate::settings::general::file_locations::{
    LocalPlacementFacts, RepositoryFacts, local_settings_root,
};

/// What the host knows about the session's folders. It holds `join`, so it crosses as a port.
pub struct SettingsLocationsInput {
    /// The session's primary working directory: where `.claude/settings.json` is read from.
    pub primary_working_directory: String,
    /// `~`.
    pub home: String,
    /// The `CLAUDE_CONFIG_DIR` directory, which replaces `~/.claude`; empty counts as unset.
    pub configuration_directory: Option<String>,
    /// The working directory and each of its parents, nearest first, up to the filesystem root.
    pub directories: Vec<String>,
    /// The root of the git repository, the main checkout's root in a worktree; absent outside git.
    pub repository_root: Option<String>,
    /// A Windows host.
    pub windows: bool,
    /// The repository root or its `.git` or `.claude` entry is owned by another user.
    pub repository_not_owned: Option<bool>,
    /// `base` and a `/`-separated relative path, joined the way the host writes paths.
    pub join: Box<dyn Join>,
}

impl SettingsLocationsInput {
    /// The input the host's record holds, each member read.
    pub async fn read(input: Port) -> Self {
        let join: Callback = input.member("join").await;
        Self {
            primary_working_directory: input.member("primaryWorkingDirectory").await,
            home: input.member("home").await,
            configuration_directory: input.member("configDirectory").await,
            directories: input.member("directories").await,
            repository_root: input.member("repositoryRoot").await,
            windows: input.member("windows").await,
            repository_not_owned: input.member("repositoryNotOwned").await,
            join: Box::new(join),
        }
    }
}

/// The settings files and the output styles' folders.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsLocations {
    /// `~/.claude/settings.json`, or `$CLAUDE_CONFIG_DIR/settings.json`: every project on the machine.
    pub user: String,
    /// `.claude/settings.json` of the primary working directory, with no parent-directory fallback.
    pub project: String,
    /// `.claude/settings.local.json`, at the repository root unless it stays with the shared file.
    pub local: String,
    /// The folders output styles are discovered in, nearest first: "the current working directory and
    /// its parents, your user directory at `~/.claude/`" (managed settings add theirs elsewhere).
    pub output_styles: Vec<String>,
}

/// The local file's folder, by the one placement rule (`localSettingsRoot`), else the working directory.
fn local_base_of(input: &SettingsLocationsInput) -> String {
    let repository = input.repository_root.as_ref().map(|root| RepositoryFacts {
        root: root.clone(),
        owned_by_user: input.repository_not_owned != Some(true),
    });
    let facts = LocalPlacementFacts {
        home_dir: input.home.clone(),
        windows: input.windows,
        repository,
    };
    local_settings_root(&facts).unwrap_or_else(|| input.primary_working_directory.clone())
}

async fn user_directory(input: &SettingsLocationsInput) -> String {
    config_directory_of(
        &input.home,
        input.configuration_directory.as_deref(),
        input.join.as_ref(),
    )
    .await
}

/// The settings files and the output styles' folders of a session.
pub async fn settings_locations(input: &SettingsLocationsInput) -> SettingsLocations {
    let join = input.join.as_ref();
    let cwd = &input.primary_working_directory;
    let local_base = local_base_of(input);
    let user = join
        .join(&user_directory(input).await, SETTINGS_FILE.user_name)
        .await;
    let project = join.join(cwd, SETTINGS_FILE.project).await;
    let local = join.join(&local_base, SETTINGS_FILE.local).await;
    let mut output_styles = Vec::with_capacity(input.directories.len() + 1);
    for directory in &input.directories {
        output_styles.push(join.join(directory, ".claude/output-styles").await);
    }
    output_styles.push(
        join.join(&user_directory(input).await, "output-styles")
            .await,
    );
    SettingsLocations {
        user,
        project,
        local,
        output_styles,
    }
}

crate::registry::exports! {
    "settings/locations" {
        "settingsLocations" => async ok |input: Port|
            settings_locations(&SettingsLocationsInput::read(input).await).await;
    }
}
