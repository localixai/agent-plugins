//! Port of `src/rules/decide/write-block-messages.ts`, switched.
//!
//! The refusals of `permissions.blockWritesOutsideWorkingDirectories`, verbatim (errors.md, "Setting
//! blocks writes outside the working directories" and "Writes to Claude Code settings files are
//! blocked"; settings-reference.md). `decide` answers with the reason (`outside-working-directory`,
//! `settings-file`); the host shows Claude the text the tool call fails with.

use serde::{Deserialize, Serialize};

/// What a write the block refuses is refused for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WriteBlockReason {
    /// `outside-working-directory`.
    OutsideWorkingDirectory,
    /// `settings-file`.
    SettingsFile,
}

/// The facts the texts name: the path written, and the working directories it is outside of.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteBlockRefusalInput {
    /// The path written.
    pub path: String,
    /// The working directories it is outside of.
    pub working_directories: Vec<String>,
}

/// `REFUSALS[reason](input)`.
fn refusal(reason: WriteBlockReason, input: &WriteBlockRefusalInput) -> String {
    let WriteBlockRefusalInput {
        path,
        working_directories,
    } = input;
    match reason {
        WriteBlockReason::OutsideWorkingDirectory => format!(
            "{path} is outside {}; the permissions.blockWritesOutsideWorkingDirectories setting blocks writes outside the working directories. Ask the user to add the directory with /add-dir, or to remove that setting.",
            working_directories.join(", ")
        ),
        WriteBlockReason::SettingsFile => format!(
            "{path}: Writes to Claude Code settings files are blocked while permissions.blockWritesOutsideWorkingDirectories is on, because a settings file can lift that block. While a managed settings folder cannot be resolved, any .json file counts as one."
        ),
    }
}

/// The text a write the block refuses fails with. The working directories are joined with `, `
/// (judgment, spec/judgments.md: the docs show the placeholder `<dirs>` only).
#[must_use]
pub fn write_block_refusal(reason: WriteBlockReason, input: &WriteBlockRefusalInput) -> String {
    refusal(reason, input)
}

/// "The refusal can also read" this, for a write outside the working directories.
#[must_use]
pub fn write_block_short_refusal(path: &str) -> String {
    format!(
        "{path}: Writes outside the working directories are blocked (permissions.blockWritesOutsideWorkingDirectories). Add the directory with /add-dir, or remove that setting."
    )
}

crate::registry::exports! {
    "rules/decide/write-block-messages" {
        "writeBlockRefusal" => ok |reason: WriteBlockReason, input: WriteBlockRefusalInput| write_block_refusal(reason, &input);
        "writeBlockShortRefusal" => ok |path: String| write_block_short_refusal(&path);
    }
}
