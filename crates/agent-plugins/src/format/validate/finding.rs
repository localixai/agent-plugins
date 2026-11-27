//! Port of `src/format/validate/finding.ts`, switched.
//!
//! What `claude plugin validate` reports about one file (the oracle of docs/SPEC.md): the file it
//! is about, relative to the folder validated, the place in it (`skills[0]`,
//! `userConfig.key.title`, `frontmatter`), and the message as the official CLI of the same
//! release prints it.

use serde::{Deserialize, Serialize};

use crate::format::issues::Severity;

/// One finding about a file of the plugin.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// How much it weighs.
    pub severity: Severity,
    /// The file, relative to the folder validated.
    pub file: String,
    /// The place in it.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

/// An error about `path` of `file`.
#[must_use]
pub fn error_in(file: &str, path: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        severity: Severity::Error,
        file: file.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

/// A warning about `path` of `file`.
#[must_use]
pub fn warning_in(file: &str, path: &str, message: &str) -> ValidationIssue {
    ValidationIssue {
        severity: Severity::Warning,
        file: file.to_owned(),
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

/// A finding before it is attached to a file: what each checker of one file's content returns.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    /// How much it weighs.
    pub severity: Severity,
    /// The place in the file.
    pub path: String,
    /// What is wrong.
    pub message: String,
}

/// An error about `path`.
#[must_use]
pub fn error_at(path: &str, message: &str) -> Finding {
    Finding {
        severity: Severity::Error,
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

/// A warning about `path`.
#[must_use]
pub fn warning_at(path: &str, message: &str) -> Finding {
    Finding {
        severity: Severity::Warning,
        path: path.to_owned(),
        message: message.to_owned(),
    }
}

/// Attaches the findings of one file to it, each path behind `prefix`.
#[must_use]
pub fn in_file(file: &str, findings: &[Finding], prefix: &str) -> Vec<ValidationIssue> {
    findings
        .iter()
        .map(|found| ValidationIssue {
            severity: found.severity,
            file: file.to_owned(),
            path: format!("{prefix}{}", found.path),
            message: found.message.clone(),
        })
        .collect()
}

crate::registry::exports! {
    "format/validate/finding" {
        "errorIn" => ok |file: String, path: String, message: String| error_in(&file, &path, &message);
    }
}
