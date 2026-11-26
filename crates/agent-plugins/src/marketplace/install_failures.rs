//! Port of `src/marketplace/install-failures.ts`, switched.
//!
//! The messages of an install that fails after the plugin was fetched (plugins/troubleshooting.md,
//! "Plugin archive integrity check failed", "An npm plugin source must name a registry package",
//! "This plugin uses a source type your Claude Code version does not support", "Plugin `<name>` has a
//! corrupt manifest file or has an invalid manifest file", "Could not move the new copy of this
//! plugin version into `<path>`", "does not load (...), so Claude Code ignores the whole file"). The
//! docs quote some of them whole and some in fragments; every fragment they quote is here verbatim,
//! and the words that join the fragments are the kit's (spec/judgments.md).

use serde::{Deserialize, Serialize};

/// What became of the copy installed before a failed move (`src/marketplace/ports.ts`, which is
/// not ported yet and owns this type). `none`: there was no earlier copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PreviousCopy {
    /// `none`.
    #[serde(rename = "none")]
    None,
    /// `moved-back`.
    #[serde(rename = "moved-back")]
    MovedBack,
    /// `removed-first`.
    #[serde(rename = "removed-first")]
    RemovedFirst,
    /// `not-moved-back`.
    #[serde(rename = "not-moved-back")]
    NotMovedBack,
    /// `could-not-be-moved-back`.
    #[serde(rename = "could-not-be-moved-back")]
    CouldNotBeMovedBack,
}

/// Why a commit did not finish (`src/marketplace/ports.ts`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CommitFailureKind {
    /// The new copy or the version folder stayed busy.
    Busy,
    /// Windows: another program holds the installed copy itself.
    Held,
    /// The cache folder cannot be written or the disk is full.
    Permission,
    /// The install was aborted.
    Aborted,
}

/// Why `StagedDirectory.commit` did not finish (`src/marketplace/ports.ts`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFailure {
    /// Why.
    pub failure: CommitFailureKind,
    /// The file-system code, such as `ENOTEMPTY`, `EBUSY`, `EACCES`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// What became of the copy installed before.
    pub previous: PreviousCopy,
    /// Set-aside folders inside the cache the failed move left behind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left_on_disk: Option<Vec<String>>,
}

/// The message of a `sha256` pin that does not match, as the docs print it whole.
#[must_use]
pub fn archive_integrity_message(url: &str, expected: &str, actual: &str) -> String {
    format!(
        "Plugin archive integrity check failed for {url}: expected sha256 {expected}, got {actual}. The archive was not installed. Verify the sha256 in the marketplace entry, or that the URL serves the intended file."
    )
}

/// What an npm `package` value refused before any fetch says after the reason ("names the value and the reason").
const NPM_REMINDER: &str = "An npm plugin source must name a registry package (name or name@version) or link to a tarball file. For a plugin in a git repository, use a \"github\", \"url\" or \"git-subdir\" source.";

/// `"<value>" was not installed: <reason>. An npm plugin source must name a registry package ...`.
/// The docs' example reason is `it is not an http or https link`; `reason` is a clause that starts
/// with `it`.
#[must_use]
pub fn npm_refusal_message(value: &str, reason: &str) -> String {
    format!("\"{value}\" was not installed: {reason}. {NPM_REMINDER}")
}

/// The reason the docs give for a value that is not a package name and not a link, such as `github:acme/formatter`.
pub const NOT_A_LINK: &str = "it is not an http or https link";

/// The message of a source type this version cannot fetch, with the line the docs say follows it.
pub const UNSUPPORTED_SOURCE_TYPE: &str = "This plugin uses a source type your Claude Code version does not support. Update Claude Code and try again.";

// ---- the manifest of a fetched plugin ----

/// `Failed to install plugin "<name>@<marketplace>": <message>`: the shell's prefix, with the plugin's real name.
#[must_use]
pub fn failed_to_install_plugin(id: &str, message: &str) -> String {
    format!("Failed to install plugin \"{id}\": {message}")
}

/// What is wrong with `.claude-plugin/plugin.json` of a plugin that was fetched: not JSON, or JSON
/// that fails the schema. `name` is what the message calls the plugin: "In the shell, the `<name>`
/// in this line can be a temporary directory name", so the caller passes what it has.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ManifestProblem {
    /// `{ parse }`.
    Parse {
        /// The JSON parse error.
        parse: String,
    },
    /// `{ validation }`.
    Validation {
        /// The validation errors.
        validation: Vec<String>,
    },
}

/// What is wrong with the manifest of a fetched plugin, in a sentence.
#[must_use]
pub fn manifest_failure_message(name: &str, problem: &ManifestProblem) -> String {
    match problem {
        ManifestProblem::Parse { parse } => {
            format!("Plugin {name} has a corrupt manifest file: JSON parse error: {parse}")
        }
        ManifestProblem::Validation { validation } => format!(
            "Plugin {name} has an invalid manifest file: Validation errors: {}",
            validation.join(", ")
        ),
    }
}

// ---- moving the new copy into the cache ----

const fn previous_sentence(previous: PreviousCopy) -> &'static str {
    match previous {
        PreviousCopy::None => "",
        PreviousCopy::MovedBack => "The previously installed copy was moved back.",
        PreviousCopy::RemovedFirst => "The previously installed copy had to be removed first.",
        PreviousCopy::NotMovedBack => "The previously installed copy was not moved back.",
        PreviousCopy::CouldNotBeMovedBack => {
            "The previously installed copy could not be moved back."
        }
    }
}

struct CauseWording {
    why: fn(&str) -> String,
    /// What the message says of the earlier copy, or instead of it.
    outcome: fn(PreviousCopy) -> &'static str,
    /// Whether the way out is to run the install again once other programs let go of the folder.
    retry: bool,
}

fn busy_why(code: &str) -> String {
    format!(
        "the new copy or the version folder stayed busy while the install ran ({code}) \u{2014} usually a scanner still reading the freshly downloaded files, another program using that folder, or another process re-creating it."
    )
}

// On Windows "the message instead says that copy `could not be replaced` and that `It was not
// replaced and the new copy was discarded`, so the version you had is still installed."
fn held_why(code: &str) -> String {
    format!(
        "another program holds the installed copy itself, so that copy could not be replaced ({code})."
    )
}

const fn held_outcome(_: PreviousCopy) -> &'static str {
    "It was not replaced and the new copy was discarded."
}

fn permission_why(code: &str) -> String {
    format!(
        "the plugin cache folder could not be written ({code}). Check the plugin cache folder's permissions and free disk space."
    )
}

/// The wording of a cause, `None` for an abort: it is not a failed move.
const fn cause(kind: CommitFailureKind) -> Option<CauseWording> {
    match kind {
        CommitFailureKind::Busy => Some(CauseWording {
            why: busy_why,
            outcome: previous_sentence,
            retry: true,
        }),
        CommitFailureKind::Held => Some(CauseWording {
            why: held_why,
            outcome: held_outcome,
            retry: true,
        }),
        CommitFailureKind::Permission => Some(CauseWording {
            why: permission_why,
            outcome: previous_sentence,
            retry: false,
        }),
        CommitFailureKind::Aborted => None,
    }
}

const RETRY: &str = "Run the install again once other Claude Code sessions or programs using that folder have finished.";

/// `Could not move the new copy of this plugin version into <path>: <why>`, then what became of the
/// copy installed before ("The message says what happened to the copy that was installed before"),
/// then the way out. A `Left on disk` list names the set-aside folders. An abort is not a failed
/// move, so it has no message.
#[must_use]
pub fn move_failure_message(path: &str, failure: &CommitFailure) -> Option<String> {
    let wording = cause(failure.failure)?;
    let left = failure.left_on_disk.as_deref().unwrap_or_default();
    let code = failure.code.as_deref().unwrap_or("unknown");
    let left_line = if left.is_empty() {
        String::new()
    } else {
        format!("Left on disk: {}.", left.join(", "))
    };
    let parts = [
        format!(
            "Could not move the new copy of this plugin version into {path}: {}",
            (wording.why)(code)
        ),
        (wording.outcome)(failure.previous).to_owned(),
        if wording.retry { RETRY } else { "" }.to_owned(),
        left_line,
    ];
    Some(
        parts
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

// ---- a settings file that does not load ----

/// Why a settings file does not load: a value Claude Code does not accept (`keys` names the settings
/// in file order) or a top level that is not an object.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum IgnoredSettings {
    /// `{ invalid }`.
    Invalid {
        /// The settings, in file order.
        invalid: Vec<String>,
    },
    /// `{ notObject: true }`.
    NotObject {
        /// Always true.
        #[serde(rename = "notObject")]
        not_object: bool,
    },
}

fn invalid_values(keys: &[String]) -> String {
    let (first, others) = keys
        .split_first()
        .map_or(("", &[][..]), |(first, others)| (first.as_str(), others));
    if others.is_empty() {
        return format!("its \"{first}\" is not valid");
    }
    let noun = if others.len() == 1 { "value" } else { "values" };
    format!(
        "its \"{first}\" and {} other {noun} are not valid",
        others.len()
    )
}

const FOLLOW_UP: &str = "including anything this command wrote there. Fix the file, then run this command again if its change is missing. If a newer Claude Code wrote the file, update Claude Code instead.";

/// The warning after the success line of `claude plugin install`, `enable`, `disable` and
/// `claude plugin marketplace add`: `⚠ <path> does not load (<why>), so Claude Code ignores the whole
/// file, ...`. With several bad values the text names the first and counts the others.
#[must_use]
pub fn settings_ignored_warning(path: &str, problem: &IgnoredSettings) -> String {
    let why = match problem {
        IgnoredSettings::Invalid { invalid } => invalid_values(invalid),
        IgnoredSettings::NotObject { .. } => "it is not a JSON object".to_owned(),
    };
    format!(
        "\u{26a0} {path} does not load ({why}), so Claude Code ignores the whole file, {FOLLOW_UP}"
    )
}

crate::registry::exports! {
    "marketplace/install-failures" {
        "archiveIntegrityMessage" => ok |url: String, expected: String, actual: String| archive_integrity_message(&url, &expected, &actual);
        "npmRefusalMessage" => ok |value: String, reason: String| npm_refusal_message(&value, &reason);
        "UNSUPPORTED_SOURCE_TYPE" => ok || UNSUPPORTED_SOURCE_TYPE;
        "failedToInstallPlugin" => ok |id: String, message: String| failed_to_install_plugin(&id, &message);
        "manifestFailureMessage" => ok |name: String, problem: ManifestProblem| manifest_failure_message(&name, &problem);
        "moveFailureMessage" => ok |path: String, failure: CommitFailure| move_failure_message(&path, &failure);
        "settingsIgnoredWarning" => ok |path: String, problem: IgnoredSettings| settings_ignored_warning(&path, &problem);
    }
}
