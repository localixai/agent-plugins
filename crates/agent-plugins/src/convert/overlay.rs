//! Port of `src/convert/overlay.ts`, switched.
//!
//! A plugin folder as it would be once a conversion's files are written (Agent Plugins Extended
//! §18.4.1 item 3): reads of a written path answer with its text, the folders above it exist, and
//! every other read goes to the folder itself. Nothing is written; it lets the target's own loader
//! and validator judge the output before a host writes it.

use indexmap::{IndexMap, IndexSet};

use super::notes::ConvertedFile;
use crate::ports::files::{DirectoryEntry, FileStat, PluginFiles, TextFileReader};
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Bytes, Port};

fn parents_of(path: &str) -> Vec<String> {
    let parts: Vec<&str> = path.split('/').collect();
    let count = parts.len().saturating_sub(1);
    (0..count)
        .map(|index| {
            parts
                .iter()
                .take(index + 1)
                .copied()
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect()
}

fn child_of<'a>(folder: &str, path: &'a str) -> Option<&'a str> {
    let prefix = if folder == "." {
        String::new()
    } else {
        format!("{folder}/")
    };
    path.strip_prefix(prefix.as_str())?.split('/').next()
}

fn file_stat(text: &str) -> FileStat {
    FileStat {
        is_file: true,
        is_directory: false,
        is_symlink: false,
        size: u64::try_from(text.len()).unwrap_or(u64::MAX),
        real_path: None,
        check_failed: None,
    }
}

fn folder_stat() -> FileStat {
    FileStat {
        is_file: false,
        is_directory: true,
        is_symlink: false,
        size: 0,
        real_path: None,
        check_failed: None,
    }
}

/// The folder with the written files over it.
#[derive(Debug)]
pub struct Overlay<F> {
    base: F,
    texts: IndexMap<String, String>,
    folders: IndexSet<String>,
}

/// A folder as it would be once the files are written.
pub fn overlay_files<F: PluginFiles>(base: F, written: &[ConvertedFile]) -> Overlay<F> {
    let texts = written
        .iter()
        .map(|file| (file.path.clone(), file.text.clone()))
        .collect();
    let folders = written
        .iter()
        .flat_map(|file| parents_of(&file.path))
        .collect();
    Overlay {
        base,
        texts,
        folders,
    }
}

impl<F> Overlay<F> {
    fn written_entries(&self, folder: &str) -> Vec<DirectoryEntry> {
        let mut names: IndexMap<&str, bool> = IndexMap::new();
        for path in self.texts.keys().chain(self.folders.iter()) {
            if let Some(name) = child_of(folder, path).filter(|name| !name.is_empty()) {
                let joined = if folder == "." {
                    name.to_owned()
                } else {
                    format!("{folder}/{name}")
                };
                names.insert(name, self.texts.contains_key(&joined));
            }
        }
        names
            .into_iter()
            .map(|(name, is_file)| DirectoryEntry {
                name: name.to_owned(),
                is_file,
                is_directory: !is_file,
            })
            .collect()
    }
}

impl<F: PluginFiles> TextFileReader for Overlay<F> {
    fn read_text<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<String>> {
        Box::pin(async move {
            match self.texts.get(path) {
                Some(text) => Some(text.clone()),
                None => self.base.read_text(path, signal).await,
            }
        })
    }
}

impl<F: PluginFiles> PluginFiles for Overlay<F> {
    fn read_bytes<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<u8>>> {
        Box::pin(async move {
            match self.texts.get(path) {
                Some(text) => Some(text.clone().into_bytes()),
                None => self.base.read_bytes(path, signal).await,
            }
        })
    }

    fn list<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<Vec<DirectoryEntry>>> {
        Box::pin(async move {
            let listed = self.base.list(path, signal).await;
            let added = self.written_entries(path);
            if listed.is_none() && !self.folders.contains(path) && path != "." {
                return None;
            }
            let mut entries: Vec<DirectoryEntry> = listed
                .unwrap_or_default()
                .into_iter()
                .filter(|entry| !added.iter().any(|one| one.name == entry.name))
                .collect();
            entries.extend(added);
            Some(entries)
        })
    }

    fn stat<'a>(
        &'a self,
        path: &'a str,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, Option<FileStat>> {
        Box::pin(async move {
            if let Some(text) = self.texts.get(path) {
                return Some(file_stat(text));
            }
            match self.base.stat(path, signal).await {
                Some(stat) => Some(stat),
                None => self.folders.contains(path).then(folder_stat),
            }
        })
    }

    fn exists<'a>(&'a self, path: &'a str, signal: Option<&'a Signal>) -> BoxFuture<'a, bool> {
        Box::pin(async move {
            self.texts.contains_key(path)
                || self.folders.contains(path)
                || self.base.exists(path, signal).await
        })
    }
}

crate::registry::methods! {
    Overlay<Port> {
        "readText" => async ok |this, path: String, signal: Option<Signal>|
            this.read_text(&path, signal.as_ref()).await;
        "readBytes" => async ok |this, path: String, signal: Option<Signal>|
            this.read_bytes(&path, signal.as_ref()).await.map(Bytes);
        "stat" => async ok |this, path: String, signal: Option<Signal>|
            this.stat(&path, signal.as_ref()).await;
        "exists" => async ok |this, path: String, signal: Option<Signal>|
            this.exists(&path, signal.as_ref()).await;
        "list" => async ok |this, path: String, signal: Option<Signal>|
            this.list(&path, signal.as_ref()).await;
    }
}

crate::registry::exports! {
    "convert/overlay" {
        "overlayFiles" => ok |base: Port, written: Vec<ConvertedFile>| crate::registry::Handle::new(overlay_files(base, &written));
    }
}
