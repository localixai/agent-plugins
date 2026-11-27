//! Port of `src/settings/general/claude-json.ts`, switched.
//!
//! What the host must do with the text of `~/.claude.json`, the file Claude Code writes for itself
//! (settings.md, "Fix a broken settings file"; errors.md, "Configuration error in ~/.claude.json"
//! and "Claude configuration file was empty and no backup holds anything"). It is no settings file
//! and no layer: the kit only decides, the host reads, copies and prints.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::js::json_parse::json_parse;
use crate::js::trim;

/// What the host read of the file.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeJsonFacts {
    /// The file's text; `undefined` when there is no file (Claude Code then starts with defaults).
    pub text: Option<String>,
    /// Where the file is, as the host writes it, for the messages.
    pub path: String,
    /// The `.claude.json.backup.<timestamp>` file that holds something, when there is one.
    pub backup: Option<String>,
    /// The file the broken copy goes to: `~/.claude/backups/.claude.json.corrupted.<timestamp>`.
    pub corrupted_copy: String,
    /// An interactive session asks; a non-interactive start such as `claude -p` exits.
    pub interactive: bool,
}

/// What follows a corrupted file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Afterwards {
    /// A non-interactive start exits with code 1.
    Exit,
    /// An interactive one asks whether to exit and fix or reset to the default configuration.
    Ask,
}

/// What to do with the file.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "outcome",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum ClaudeJsonOutcome {
    /// The file parses, or there is none: nothing to report.
    Usable,
    /// Blank and no backup holds anything: start with defaults and print `message`.
    StartsWithDefaults {
        /// The line to print.
        message: String,
    },
    /// Unparseable: the file is left unchanged, copied to `copy_to`, and `afterwards` follows.
    ConfigurationError {
        /// The issue.
        issue: PluginIssue,
        /// The lines Claude Code prints.
        output: Vec<String>,
        /// Where the copy goes.
        copy_to: String,
        /// `exit` or `ask`.
        afterwards: Afterwards,
    },
}

fn parse_failure(text: &str) -> Option<String> {
    json_parse(text).err()
}

/// The lines Claude Code prints for a corrupted file, the backup's `cp` line only when a backup
/// holds something.
fn output_of(facts: &ClaudeJsonFacts, reason: &str) -> Vec<String> {
    let restore = match &facts.backup {
        None => Vec::new(),
        Some(backup) => vec![
            format!("A backup file exists at: {backup}"),
            format!(
                "You can manually restore it by running: cp -f \"{backup}\" \"{}\"",
                facts.path
            ),
        ],
    };
    let mut lines = vec![
        format!(
            "Claude configuration file at {} is corrupted: {reason}",
            facts.path
        ),
        format!(
            "The corrupted file has been backed up to: {}",
            facts.corrupted_copy
        ),
    ];
    lines.extend(restore);
    lines.push(String::new());
    lines.push(format!("Configuration error in {}: {reason}", facts.path));
    lines
}

/// What the host must do with the text of `~/.claude.json`.
#[must_use]
pub fn read_claude_json(facts: &ClaudeJsonFacts) -> ClaudeJsonOutcome {
    let Some(text) = &facts.text else {
        return ClaudeJsonOutcome::Usable;
    };
    if trim(text).is_empty() && facts.backup.is_none() {
        return ClaudeJsonOutcome::StartsWithDefaults {
            message: format!(
                "Claude configuration file at {} was empty and no backup holds anything: starting with defaults.",
                facts.path
            ),
        };
    }
    let reason = if trim(text).is_empty() {
        Some("Unexpected end of JSON input".to_owned())
    } else {
        parse_failure(text)
    };
    let Some(reason) = reason else {
        return ClaudeJsonOutcome::Usable;
    };
    ClaudeJsonOutcome::ConfigurationError {
        issue: error(
            &facts.path,
            &format!("Configuration error in {}: {reason}", facts.path),
        ),
        output: output_of(facts, &reason),
        copy_to: facts.corrupted_copy.clone(),
        afterwards: if facts.interactive {
            Afterwards::Ask
        } else {
            Afterwards::Exit
        },
    }
}

crate::registry::exports! {
    "settings/general/claude-json" {
        "readClaudeJson" => ok |facts: ClaudeJsonFacts| read_claude_json(&facts);
    }
}
