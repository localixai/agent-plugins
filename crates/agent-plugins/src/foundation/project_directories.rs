//! Ported from `src/foundation/project-directories.ts`; only the core uses it now, so it has no
//! wrapper.
//!
//! The project directories whose `.claude` folder is read: the working directory and every parent up
//! to the repository root, nearest first. In a linked worktree whose top has no `.claude/skills`, the
//! main checkout's loads as well, last (v2.1.277 and later; skills.md, "Load skills in monorepos and
//! subdirectories"). Judgment (spec/judgments.md): worktrees.md titles that bullet "skills, agents, and
//! commands", so the same rule gives the agents and commands folders, and the main checkout is never
//! a base of a skill-folder plugin. Shared by the skills and the agents scans.

use crate::ports::folder_scan::FolderScan;
use crate::ports::{BoxFuture, Signal};

/// The project directories whose `.claude` folder is read, nearest first.
pub fn project_directories<'a>(
    folders: &'a dyn FolderScan,
    cwd: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, Vec<String>> {
    Box::pin(async move {
        let chain = folders.repository(cwd, signal).await;
        let (Some(main_checkout), Some(top)) =
            (chain.main_checkout.as_deref(), chain.directories.last())
        else {
            return chain.directories;
        };
        let has = match folders.open(top, signal).await {
            Some(folder) => folder.files().await.exists(".claude/skills", signal).await,
            None => false,
        };
        if has {
            return chain.directories;
        }
        let mut directories = chain.directories.clone();
        directories.push(main_checkout.to_owned());
        directories
    })
}
