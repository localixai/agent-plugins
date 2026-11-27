//! Port of `src/marketplace/folder-collision.ts`, switched.
//!
//! Two plugin ids that share one folder (plugins/troubleshooting.md, "`\"<plugin>\" was not installed:
//! it would share its folder with \"<other>\"`"): "the same once `.` and `@` are written as `-`. On
//! macOS and Windows, ids that differ only in capitals map to the same folder too. Installing both
//! would put one plugin's files in the other's folder, so Claude Code refuses and the installed
//! plugin keeps its files." Pure over ids; the host reads the installed ones and refuses the install.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Where an install was typed (`src/marketplace/install-errors.ts`, which is not ported yet and
/// owns this type): in a shell, or in a session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallSurface {
    /// The `/plugin` panel of a session.
    Session,
    /// `claude plugin install` in a shell.
    Shell,
}

/// The platform whose folder names are compared.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FolderPlatform {
    /// macOS.
    Macos,
    /// Windows.
    Windows,
    /// Linux.
    Linux,
}

/// Which platforms fold capitals when they name a folder.
const fn folds_case(platform: FolderPlatform) -> bool {
    match platform {
        FolderPlatform::Macos | FolderPlatform::Windows => true,
        FolderPlatform::Linux => false,
    }
}

/// The folder name an id maps to: `.` and `@` written as `-`, capitals folded where the platform does.
#[must_use]
pub fn folder_of(id: &str, platform: FolderPlatform) -> String {
    let written = id.replace(['.', '@'], "-");
    if folds_case(platform) {
        written.to_lowercase()
    } else {
        written
    }
}

/// What the two plugins would share: the section quotes both `its folder` and `its saved data`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SharedThing {
    /// `folder`.
    #[serde(rename = "folder")]
    Folder,
    /// `saved data`.
    #[serde(rename = "saved data")]
    SavedData,
}

impl SharedThing {
    /// The words the message says.
    const fn words(self) -> &'static str {
        match self {
            Self::Folder => "folder",
            Self::SavedData => "saved data",
        }
    }
}

/// The ids that cannot be installed together with what they collide with.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCollision {
    /// The id that is refused.
    pub plugin: String,
    /// The id it would share a folder with.
    pub other: String,
    /// The other plugin is installed, so uninstalling it is a way out; false when both arrive in one install.
    pub other_installed: bool,
    /// What the user is told.
    pub message: String,
}

/// The way out when the other plugin is installed, in the words of the surface the install was typed on.
fn way_out(surface: InstallSurface, other: &str) -> String {
    match surface {
        InstallSurface::Shell => format!(
            "Only one of the two can be installed. Run claude plugin uninstall {other}, then install again."
        ),
        InstallSurface::Session => format!(
            "Only one of the two can be installed. Use the uninstall step in /plugin to remove \"{other}\", then install again."
        ),
    }
}

fn collision(
    plugin: &str,
    other: &str,
    other_installed: bool,
    surface: InstallSurface,
    shares: SharedThing,
) -> FolderCollision {
    let refused = format!(
        "\"{plugin}\" was not installed: it would share its {} with \"{other}\"",
        shares.words()
    );
    FolderCollision {
        plugin: plugin.to_owned(),
        other: other.to_owned(),
        other_installed,
        message: if other_installed {
            format!("{refused}. {}", way_out(surface, other))
        } else {
            refused
        },
    }
}

/// What `folder_collisions` takes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderCollisionInput {
    /// The ids being installed.
    pub incoming: Vec<String>,
    /// The ids already installed.
    pub installed: Vec<String>,
    /// The platform whose folder names are compared.
    pub platform: FolderPlatform,
    /// Where the install was typed.
    pub surface: InstallSurface,
    /// Judgment: the docs print `folder` or `saved data` without saying when; the caller names the one it checked.
    #[serde(default)]
    pub shares: Option<SharedThing>,
}

/// The incoming ids that cannot be installed: those that map to the folder of an installed id, and
/// those that map to the folder of an id that arrived earlier in the same install (judgment: the
/// later one is refused). An id that is already installed is not a collision with itself.
#[must_use]
pub fn folder_collisions(input: &FolderCollisionInput) -> Vec<FolderCollision> {
    let shares = input.shares.unwrap_or(SharedThing::Folder);
    let mut folders: HashMap<String, (&str, bool)> = HashMap::new();
    for id in &input.installed {
        folders.insert(folder_of(id, input.platform), (id.as_str(), true));
    }
    let mut refused = Vec::new();
    for id in &input.incoming {
        let key = folder_of(id, input.platform);
        match folders.get(&key) {
            None => {
                folders.insert(key, (id.as_str(), false));
            }
            Some(&(holder, installed)) if holder != id => {
                refused.push(collision(id, holder, installed, input.surface, shares));
            }
            Some(_) => {}
        }
    }
    refused
}

crate::registry::exports! {
    "marketplace/folder-collision" {
        "folderOf" => ok |id: String, platform: FolderPlatform| folder_of(&id, platform);
        "folderCollisions" => ok |input: FolderCollisionInput| folder_collisions(&input);
    }
}
