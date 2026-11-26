//! Port of `src/skills/file-references.ts`, switched.
//!
//! The files a text names with `@`. Claude Code attaches them for a local skill's or command's body:
//! a synced skill's body "doesn't attach the files that `@` references name the way it does for a
//! local skill" (skills.md, "How Claude Code handles the body of a synced skill"; changelog.md,
//! "Custom slash commands: ... @-mention files", "Added support for @-mentions in slash command
//! arguments"). The kit finds and resolves them; reading and attaching a file is the host's, as for
//! an `@` the user types (common-workflows.md, "Reference files and directories": the Read tool's
//! limits apply, and a directory or an MCP resource is the host's to tell apart).
//!
//! The docs give no syntax for a skill's references. Oracle 2.1.322 (spec/judgments.md): a reference
//! is `@` at the start of the text or after whitespace, then a path up to the next whitespace, or a
//! path in double quotes, which may hold spaces; a backslash does not escape a space; Markdown code
//! spans and fences are not skipped; trailing `` .,;:!?)"` `` and a `#<line range>` are not part of the
//! path; a relative path resolves against the working directory (not the skill's folder), `~/` against
//! the home folder; the same reference twice is attached once.

use serde::{Deserialize, Serialize};

use crate::format::path_text::is_absolute_path;
use crate::format::skill_types::origin_traits;
use crate::js::regexp::RegExp;
use crate::ports::locations::Locations as _;
use crate::registry::Port;

/// A reference found in a text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct FileReference {
    /// The reference as written, `@` included: `@src/a.ts`, `@"Design Docs/api.md"`, `@notes.md#2-3`.
    pub text: String,
    /// The file it names, resolved (`Locations.joinPath`): the host checks it exists and reads it.
    pub path: String,
}

/// Where a reference resolves from (`ReferenceBase`).
#[derive(Deserialize)]
pub struct ReferenceBase {
    /// The working directory a relative path resolves against.
    pub cwd: String,
    /// The host's `Pick<Locations, 'homeDir' | 'joinPath'>`.
    pub locations: Port,
}

/// Where a skill came from: its kind (`SkillOrigin`).
#[derive(Clone, Debug, Deserialize)]
pub struct SkillOrigin {
    /// The origin's kind.
    pub kind: String,
}

const REFERENCE: &str = r#"(?<!\S)@(?:"(?<quoted>[^"\n]+)"|(?<bare>\S*?[^\s.,;:!?)"`]))(?:#L?\d+(?:-L?\d+)?)?[.,;:!?)"`]*(?=\s|$)"#;

async fn resolved(path: &str, base: &ReferenceBase) -> String {
    let locations = &base.locations;
    if path == "~" {
        return locations.home_dir().await;
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return locations.join_path(&locations.home_dir().await, rest).await;
    }
    if is_absolute_path(path) {
        path.to_owned()
    } else {
        locations.join_path(&base.cwd, path).await
    }
}

/// The references of `text`, in order, each once.
pub async fn read_file_references(text: &str, base: &ReferenceBase) -> Vec<FileReference> {
    let mut seen: Vec<String> = Vec::new();
    let mut references = Vec::new();
    let found = RegExp::new(REFERENCE, "gu")
        .ok()
        .and_then(|regexp| regexp.match_all(text).ok())
        .unwrap_or_default();
    for found in found {
        let group = |name: &str| {
            found
                .groups
                .as_ref()
                .and_then(|groups| groups.get(name).cloned().flatten())
        };
        let path = group("quoted")
            .or_else(|| group("bare"))
            .unwrap_or_default();
        let written = found
            .captures
            .first()
            .cloned()
            .flatten()
            .unwrap_or_default();
        if path.is_empty() || seen.contains(&written) {
            continue;
        }
        seen.push(written.clone());
        references.push(FileReference {
            text: written,
            path: resolved(&path, base).await,
        });
    }
    references
}

/// Whether an expansion of a skill from `origin` attaches its references; `isolated`: a cloud or Cowork session.
#[must_use]
pub fn attaches_file_references(origin: &SkillOrigin, isolated: bool) -> bool {
    origin_traits()
        .get(origin.kind.as_str())
        .is_some_and(|traits| traits.attaches_file_references)
        || isolated
}

crate::registry::exports! {
    "skills/file-references" {
        "readFileReferences" => async ok |text: String, base: ReferenceBase|
            read_file_references(&text, &base).await;
        "attachesFileReferences" => ok |origin: SkillOrigin, isolated: bool|
            attaches_file_references(&origin, isolated);
    }
}
