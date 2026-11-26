//! Port of `src/loading/declared-shape.ts`, switched.
//!
//! What a manifest path of each kind must be (plugins/manifest-reference.md, "Path-only fields":
//! "`agents` entries must be `.md` files, and `skills` entries must be directories. The other three
//! accept a directory or a file"; `commands` takes a flat file or a directory). The message of the
//! mismatch, or `None` when the path is the right shape. The wording is the kit's (judgment).
//!
//! The table crosses as functions of the core's (`registry::Function`), one per kind, as the
//! TypeScript's table holds a function per kind.

use indexmap::IndexMap;
use serde::Deserialize;

use crate::format::messages::{agents_entry_not_file, skills_entry_not_directory};
use crate::ports::files::FileStat;
use crate::registry::Function;

/// A kind a manifest lists paths of (`ListedKind` of `src/loading/walk.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListedKind {
    /// `skills`.
    Skills,
    /// `agents`.
    Agents,
    /// `commands`.
    Commands,
    /// `outputStyles`.
    OutputStyles,
    /// `workflows`.
    Workflows,
    /// `themes`.
    Themes,
}

/// A path of any shape is right.
const fn either(_: &FileStat, _: &str) -> Option<String> {
    None
}

/// The message of a path of `kind` that is the wrong shape (`DECLARED_SHAPE[kind](stat, written)`).
#[must_use]
pub fn declared_shape(kind: ListedKind, stat: &FileStat, written: &str) -> Option<String> {
    match kind {
        ListedKind::Skills => (!stat.is_directory).then(|| skills_entry_not_directory(written)),
        ListedKind::Agents => stat.is_directory.then(|| agents_entry_not_file(written)),
        ListedKind::Commands
        | ListedKind::OutputStyles
        | ListedKind::Workflows
        | ListedKind::Themes => either(stat, written),
    }
}

/// One kind's entry of the table, a function the host calls.
#[derive(Debug)]
struct Shape(ListedKind);

crate::registry::methods! {
    Shape {
        "call" => ok |this, stat: FileStat, written: String| declared_shape(this.0, &stat, &written);
    }
}

/// The table, by kind, in the TypeScript's order.
const DECLARED_SHAPE: [(&str, ListedKind); 6] = [
    ("skills", ListedKind::Skills),
    ("agents", ListedKind::Agents),
    ("commands", ListedKind::Commands),
    ("outputStyles", ListedKind::OutputStyles),
    ("workflows", ListedKind::Workflows),
    ("themes", ListedKind::Themes),
];

crate::registry::exports! {
    "loading/declared-shape" {
        "DECLARED_SHAPE" => ok || DECLARED_SHAPE
            .map(|(name, kind)| (name, Function::new(Shape(kind))))
            .into_iter()
            .collect::<IndexMap<_, _>>();
    }
}
