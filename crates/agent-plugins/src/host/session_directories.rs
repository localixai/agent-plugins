//! Port of `src/host/session-directories.ts`, switched.
//!
//! The directories of a live session: the working directory and the directories added with
//! `/add-dir`. `SessionOptions` gives the first values; `/cd` (permissions.md, "Move the session
//! to another directory") and `/add-dir` change them while the session, its hooks and its `once`
//! keys go on. Every part of the session reads them through here, never from the options.
//!
//! The options hold the session's ports (functions), so they cross as the host's object and the
//! two members read here are asked for.

use std::sync::{Mutex, PoisonError};

use serde::Deserialize;

use crate::registry::{Handle, Port};

/// The members of `SessionOptions` read here.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryOptions {
    /// The working directory the session starts in.
    #[serde(default)]
    pub cwd: String,
    /// The directories added at launch.
    #[serde(default)]
    pub added_directories: Option<Vec<String>>,
}

/// `SessionOptions` as it crosses: the host's object when it holds a function, else data.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SessionOptions {
    /// The options hold a function.
    Host(Port),
    /// The options are data.
    Data(DirectoryOptions),
}

impl SessionOptions {
    async fn read(&self) -> DirectoryOptions {
        match self {
            Self::Data(data) => data.clone(),
            Self::Host(port) => DirectoryOptions {
                cwd: port.member::<String>("cwd").await,
                added_directories: port.member::<Option<Vec<String>>>("addedDirectories").await,
            },
        }
    }
}

#[derive(Debug)]
struct Directories {
    cwd: String,
    added: Vec<String>,
}

/// The directories of one session.
#[derive(Debug)]
pub struct SessionDirectories {
    state: Mutex<Directories>,
}

impl SessionDirectories {
    fn with<R>(&self, body: impl FnOnce(&mut Directories) -> R) -> R {
        body(&mut self.state.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// The working directory.
    pub fn cwd(&self) -> String {
        self.with(|state| state.cwd.clone())
    }

    /// The added directories, in the order they were added.
    pub fn added(&self) -> Vec<String> {
        self.with(|state| state.added.clone())
    }

    /// `/add-dir`: `false` when the directory was already added or is the working directory.
    pub fn add(&self, path: String) -> bool {
        self.with(|state| {
            if path == state.cwd || state.added.contains(&path) {
                return false;
            }
            state.added.push(path);
            true
        })
    }

    /// `/cd`.
    pub fn move_to(&self, next: String) {
        self.with(|state| state.cwd = next);
    }
}

/// The directories of a session that starts with `options`.
pub async fn create_session_directories(options: &SessionOptions) -> SessionDirectories {
    let read = options.read().await;
    SessionDirectories {
        state: Mutex::new(Directories {
            cwd: read.cwd,
            added: read.added_directories.unwrap_or_default(),
        }),
    }
}

crate::registry::methods! {
    SessionDirectories {
        "cwd" => ok |this| this.cwd();
        "added" => ok |this| this.added();
        "add" => ok |this, path: String| this.add(path);
        "moveTo" => ok |this, next: String| this.move_to(next);
    }
}

crate::registry::exports! {
    "host/session-directories" {
        "createSessionDirectories" => async ok |options: SessionOptions|
            Handle::new(create_session_directories(&options).await);
    }
}
