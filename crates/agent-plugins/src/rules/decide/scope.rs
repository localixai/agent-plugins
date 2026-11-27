//! Ported from `src/rules/decide/scope.ts`; only the core uses it now, so it has no wrapper.
//!
//! The working directories and what is inside them.

use crate::rules::path::posix::{normalize_absolute, relative_within, to_posix_path};
use crate::rules::types::PathContext;

/// A host path in the kit's one form: POSIX, absolute, `.` and `..` folded.
#[must_use]
pub fn posix_of(path: &str, paths: &PathContext) -> String {
    normalize_absolute(&to_posix_path(path, paths.platform))
}

/// The working directory and the additional directories, in the kit's path form, each with its real
/// location when it is reached through a link (`PathContext.realDirectories`): a file the symlink
/// check resolves below the real location sits in that working directory (permissions.md, "Writes
/// through a symlink"; a judgment, spec/judgments.md).
fn working_directories(paths: &PathContext) -> Vec<String> {
    std::iter::once(&paths.primary_working_directory)
        .chain(&paths.additional_directories)
        .flat_map(|dir| {
            let spelled = posix_of(dir, paths);
            let real = paths.real_directories.as_ref().and_then(|map| {
                map.entries
                    .iter()
                    .find(|(key, _)| *key == spelled)
                    .map(|(_, real)| real.clone())
            });
            match real {
                None => vec![spelled],
                Some(real) => vec![spelled, real],
            }
        })
        .collect()
}

/// Whether `path` is the working directory or an additional directory, or below one.
#[must_use]
pub fn in_scope(path: &str, paths: &PathContext) -> bool {
    working_directories(paths)
        .iter()
        .any(|root| relative_within(root, path, Some(paths.platform)).is_some())
}
