//! Port of `src/marketplace/add-failures.ts`, switched.
//!
//! What `claude plugin marketplace add` and an install say when the source cannot be fetched
//! (plugins/troubleshooting.md, "Invalid git URL", "Path does not exist: `<path>`", "Marketplace file
//! not found at ...", "Git clone timed out after 120s", `Marketplace "<name>" not found`). Pure: the
//! host found the failure, the kit words it. The texts the docs quote are the docs'; the rest is the
//! output of `claude plugin marketplace add` 2.1.319, recorded in spec/judgments.md as `oracle 2.1.319`.

use serde::{Deserialize, Serialize};

use crate::format::marketplace::ids::{INSTALL_RULE, is_plugin_id_part};
use crate::js::regexp::{RegExp, RegExpMatch};
use crate::js::{math_round, number_to_string, trim};

const FAILED: &str = "Failed to add marketplace: ";

/// What the host found.
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "kebab-case",
    rename_all_fields = "camelCase"
)]
pub enum AddFailure {
    /// A local path with nothing at it; the host resolved it against its working directory.
    PathMissing {
        /// The path.
        path: String,
    },
    /// A path to a file that is not a `.json` file.
    NotJson {
        /// The path.
        path: String,
    },
    /// The marketplace was cloned, found, or downloaded but holds no `marketplace.json` at `path`.
    FileMissing {
        /// The path.
        path: String,
    },
    /// A relative `source` whose directory is missing; `path` is "the absolute location Claude Code checked".
    SourcePathMissing {
        /// The path.
        path: String,
    },
    /// The clone ran past `timeoutMs` (`CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS`, default 120000).
    CloneTimeout {
        /// The limit, in milliseconds.
        timeout_ms: f64,
    },
}

fn wording(failure: &AddFailure) -> String {
    match failure {
        AddFailure::PathMissing { path } => format!("Path does not exist: {path}"),
        AddFailure::NotJson { path } => {
            format!("File path must point to a .json file (marketplace.json), but got: {path}")
        }
        AddFailure::SourcePathMissing { path } => format!("Source path does not exist: {path}"),
        AddFailure::FileMissing { path } => format!("{FAILED}Marketplace file not found at {path}"),
        AddFailure::CloneTimeout { timeout_ms } => format!(
            "{FAILED}Git clone timed out after {}s. Set CLAUDE_CODE_PLUGIN_GIT_TIMEOUT_MS to a larger number of milliseconds to allow more time",
            number_to_string(math_round(timeout_ms / 1000.0))
        ),
    }
}

/// The line a failed add ends with.
#[must_use]
pub fn add_failure_message(failure: &AddFailure) -> String {
    wording(failure)
}

const SUPPORTED: &str = "Supported: https, http, ssh (also written git+ssh or ssh+git), user@host:path, or a local file:// address with no host. git:// isn't supported because it isn't encrypted.";

const REMOTE_URL: &str = r"^(?:https?|ssh|git\+ssh|ssh\+git):\/\/([^/?#]*)";
const SCP_FORM: &str = r"^([^\s/@:]+)@([^\s/@:]+):(\S+)$";

const UNREADABLE: &str = "it could not be read as a URL";

/// One run of a constant pattern; a pattern that does not compile matches nothing.
fn exec(pattern: &str, flags: &str, text: &str) -> Option<RegExpMatch> {
    RegExp::new(pattern, flags).ok()?.exec(text)
}

/// The text of capture `index`, `None` for a group that did not take part.
fn capture(found: &RegExpMatch, index: usize) -> Option<&str> {
    found.captures.get(index)?.as_deref()
}

fn dashed(text: Option<&str>) -> bool {
    text.unwrap_or("").starts_with('-')
}

/// One reader per address form: the reason it is refused, `Ok(None)` when it passes, `Err(())` when
/// the form is not its own (`null` in the TypeScript).
type Reading = Result<Option<String>, ()>;

fn read_scp(address: &str) -> Reading {
    let Some(scp) = exec(SCP_FORM, "", address) else {
        return Err(());
    };
    Ok((dashed(capture(&scp, 1)) || dashed(capture(&scp, 2))).then(|| UNREADABLE.to_owned()))
}

fn read_remote(address: &str) -> Reading {
    let Some(remote) = exec(REMOTE_URL, "i", address) else {
        return Err(());
    };
    let host = capture(&remote, 1)
        .and_then(|text| text.split('@').next_back())
        .unwrap_or("");
    Ok((host.is_empty() || dashed(Some(host))).then(|| UNREADABLE.to_owned()))
}

fn read_file(address: &str) -> Reading {
    let Some(file) = exec(r"^file:\/\/([^/]*)", "i", address) else {
        return Err(());
    };
    Ok((capture(&file, 1) != Some("")).then(|| "a file:// address must have no host".to_owned()))
}

fn read_other(address: &str) -> String {
    let protocol = exec(r"^([A-Za-z][A-Za-z0-9+.-]*):", "", address)
        .and_then(|found| capture(&found, 1).map(str::to_owned));
    protocol.map_or_else(
        || UNREADABLE.to_owned(),
        |protocol| {
            format!(
                "the protocol \"{}:\" is not supported",
                protocol.to_lowercase()
            )
        },
    )
}

const READERS: [fn(&str) -> Reading; 3] = [read_scp, read_remote, read_file];

/// Why an address is refused, or `None` when it is one Claude Code runs git with.
fn refusal_reason(address: &str) -> Option<String> {
    for read in READERS {
        if let Ok(reason) = read(address) {
            return reason;
        }
    }
    Some(read_other(address))
}

/// "Claude Code checks every git address before running git": a protocol it doesn't support, and an
/// address git could read as naming a different server than the one shown, are refused with `Invalid
/// git URL` and the reason. The address itself is not echoed (the oracle prints `[redacted URL]`).
#[must_use]
pub fn git_url_refusal(address: &str) -> Option<String> {
    refusal_reason(trim(address))
        .map(|reason| format!("{FAILED}Invalid git URL: [redacted URL] ({reason}). {SUPPORTED}"))
}

/// The accepted forms a bare name or hostname is told about (`Invalid marketplace source format`).
pub const ACCEPTED_SOURCE_FORMS: [&str; 4] = [
    "a GitHub owner/repo shorthand",
    "an https:// or http:// URL",
    "a user@host:path SSH URL",
    "a local path starting with ./, ../, / or ~",
];

const OFFICIAL: &str = "claude-plugins-official";
const CLAUDE_AI_PREFIX: &str = "claudeai-";

/// What a missing marketplace is told.
#[derive(Clone, Debug, Serialize)]
pub struct MissingMarketplace {
    /// The line.
    pub message: String,
    /// What to run next, as the docs give it; `undefined` when only the marketplace's owner can say.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

/// `Marketplace "<name>" not found` on `/plugin install <plugin>@<name>`: Claude Code has "no index to
/// look a marketplace name up in", so the fix is the marketplace's source, except for the official
/// marketplace and for a `claudeai-` name, which have a command.
#[must_use]
pub fn missing_marketplace(name: &str) -> MissingMarketplace {
    let message = format!("Marketplace \"{name}\" not found");
    if name == OFFICIAL {
        return MissingMarketplace {
            message,
            fix: Some("/plugin marketplace add anthropics/claude-plugins-official".to_owned()),
        };
    }
    if name.starts_with(CLAUDE_AI_PREFIX) {
        return MissingMarketplace {
            message,
            fix: Some(format!("claude plugin marketplace add --claudeai {name}")),
        };
    }
    MissingMarketplace { message, fix: None }
}

/// Adding a marketplace whose `name` is not valid as the part after `@` in a plugin id is refused and
/// registers nothing; the text is the docs' ("Cannot add marketplace ..."). An empty name is left to the
/// reader's own "must have a name".
#[must_use]
pub fn id_part_refusal(name: &str) -> Option<String> {
    if name.is_empty() || is_plugin_id_part(name) {
        return None;
    }
    Some(format!(
        "Cannot add marketplace \"{name}\": Claude Code cannot install plugins from a marketplace with this name. {INSTALL_RULE} The name is set by \"name\" in the marketplace's marketplace.json; ask its maintainer to change it."
    ))
}

crate::registry::exports! {
    "marketplace/add-failures" {
        "addFailureMessage" => ok |failure: AddFailure| add_failure_message(&failure);
        "gitUrlRefusal" => ok |address: String| git_url_refusal(&address);
        "ACCEPTED_SOURCE_FORMS" => ok || ACCEPTED_SOURCE_FORMS;
        "missingMarketplace" => ok |name: String| missing_marketplace(&name);
        "idPartRefusal" => ok |name: String| id_part_refusal(&name);
    }
}
