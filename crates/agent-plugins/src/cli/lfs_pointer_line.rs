//! Port of `src/cli/lfs-pointer-line.ts`, switched.
//!
//! The line `marketplace add` and `marketplace update` print after a git clone that left LFS-tracked
//! files as pointer files (spec §14.8.4, the `git` source row: "the add or update output reports how
//! many"; plugins/host-marketplace.md, "Keep plugin files out of Git LFS"). The host counts them
//! (`lfsPointerIssues` finds them file by file). Judgment: the wording (spec/judgments.md).

use crate::js::number_to_string;

/// The line for `count` pointer files, or none when the clone left none.
#[must_use]
pub fn lfs_pointer_line(count: f64) -> Option<String> {
    if count <= 0.0 {
        return None;
    }
    let files = if count == 1.0 {
        "file arrived as a pointer file"
    } else {
        "files arrived as pointer files"
    };
    Some(format!(
        "{} Git LFS {files}: the clone never downloads LFS content",
        number_to_string(count)
    ))
}

crate::registry::exports! {
    "cli/lfs-pointer-line" {
        "lfsPointerLine" => ok |count: f64| lfs_pointer_line(count);
    }
}
