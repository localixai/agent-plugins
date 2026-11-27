//! Ported from `src/rules/decide/shell-words.ts`; only the core uses it now, so it has no wrapper.
//!
//! Where a command's operands lead, read against the directory a `cd` moved to.

use super::context::ProtectedPathOptions;
use crate::rules::bash::words::ShellWord;
use crate::rules::path::posix::{HostPaths, absolute_path};
use crate::rules::types::PathContext;

/// The state of the walk over a command's subcommands, in the order they run.
pub struct Walk<'a> {
    /// The folders and platform the paths are read against.
    pub paths: &'a PathContext,
    /// The session's protected-path options.
    pub options: &'a ProtectedPathOptions,
    /// Where the host's symlink check says the call's paths lead.
    pub resolved: &'a [String],
    /// The directory later operands are read against: moved by a `cd`.
    pub base: String,
    /// A `cd` moved `base`.
    pub changed_directory: bool,
    /// A `git` subcommand ran.
    pub used_git: bool,
}

fn host_paths(walk: &Walk<'_>) -> HostPaths {
    HostPaths {
        cwd: walk.base.clone(),
        home: walk.paths.home.clone(),
        platform: walk.paths.platform,
    }
}

/// Whether any word holds a glob outside quotes.
#[must_use]
pub fn has_unquoted_glob(words: &[ShellWord]) -> bool {
    words.iter().any(|word| word.globs)
}

/// The absolute POSIX path of an operand, or `None` when a variable, substitution or glob hides it.
#[must_use]
pub fn operand_path(operand: &ShellWord, walk: &Walk<'_>) -> Option<String> {
    if operand.value.is_empty() || operand.expands || operand.globs {
        return None;
    }
    Some(absolute_path(&operand.value, &host_paths(walk)))
}

/// Where a file-reading operand reads: the path itself, the directory a glob starts in, or
/// `None` for an operand whose value a variable or substitution hides.
#[must_use]
pub fn read_path(operand: &ShellWord, walk: &Walk<'_>) -> Option<String> {
    if operand.expands {
        return None;
    }
    let text = operand.value.as_str();
    // `text.search(/[*?[]/)`: the first glob character, when the operand globs.
    let glob = if operand.globs {
        text.find(['*', '?', '['])
    } else {
        None
    };
    let literal = match glob {
        None => text.to_owned(),
        Some(glob) => {
            let directory = text
                .get(..glob)
                .and_then(|before| before.rfind('/'))
                .and_then(|slash| text.get(..=slash))
                .unwrap_or_default();
            if directory.is_empty() {
                ".".to_owned()
            } else {
                directory.to_owned()
            }
        }
    };
    Some(absolute_path(&literal, &host_paths(walk)))
}
