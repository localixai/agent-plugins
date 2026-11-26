//! Port of `src/marketplace/relevance-match.ts`, switched.
//!
//! Whether the signals of a `relevance` block match the session (plugins/relevance.md, "Understand
//! how plugin relevance works", "`relevance.signals`", "Working directory matching", "Command name
//! matching", "Manifest dependency matching"): pure, over the facts the host collected. Matching is
//! local; nothing here reports which signal matched. What the docs leave open (the glob dialect, an
//! invalid regular expression, a compound command's separators) is in spec/judgments.md.

use serde::{Deserialize, Serialize};

use crate::format::path_text::without_trailing_slashes;
use crate::format::wildcard::escape_reg_exp;
use crate::js::indexing::length_number;
use crate::js::regexp::RegExp;
use crate::js::{code_unit_length, slice_code_units};

/// A signal of a `relevance` block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SignalName {
    /// `cwd`.
    Cwd,
    /// `cli`.
    Cli,
    /// `hosts`.
    Hosts,
    /// `filesRead`.
    FilesRead,
    /// `manifestDeps`.
    ManifestDeps,
}

/// One `manifestDeps` entry (`RelevanceSignals` of `src/format/marketplace/types.ts`, which is
/// not ported yet).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestDep {
    /// A case-insensitive regular expression over the manifest's path.
    pub file: String,
    /// A regular expression over its contents.
    pub pattern: String,
}

/// The signals of a `relevance` block (`src/format/marketplace/types.ts`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelevanceSignals {
    /// Working-directory patterns.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<Vec<String>>,
    /// Command names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cli: Option<Vec<String>>,
    /// Hostnames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hosts: Option<Vec<String>>,
    /// Globs over the files touched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files_read: Option<Vec<String>>,
    /// Manifests and the dependency each names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manifest_deps: Option<Vec<ManifestDep>>,
}

/// A package manifest Claude read, wrote or edited.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ManifestFact {
    /// Its path.
    pub path: String,
    /// Its contents.
    pub content: String,
    /// Byte size when known; the length of `content` otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f64>,
}

/// What the host collected of the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionFacts {
    /// The session's working directory, absolute.
    pub cwd: String,
    /// The root of the git repository the session is inside, when it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository_root: Option<String>,
    /// The shell commands Claude ran this session.
    pub commands: Vec<String>,
    /// Paths Claude read, wrote or edited this session, and the project's auto-loaded `CLAUDE.md` files.
    pub files_touched: Vec<String>,
    /// The package manifests Claude read, written or edited, and their contents.
    pub manifests: Vec<ManifestFact>,
}

/// "Manifest files larger than 512 KB are skipped".
pub const MANIFEST_SIZE_LIMIT: u32 = 512 * 1024;

fn slashes(path: &str) -> String {
    path.replace('\\', "/")
}

/// A regular expression of the module's own, with its flags.
fn compiled(source: &str, flags: &str) -> Option<RegExp> {
    RegExp::new(source, flags).ok()
}

/// A glob as a regular expression: `**` crosses `/`, `*` and `?` stay in a segment.
fn glob_source(glob: &str) -> String {
    let chars: Vec<char> = glob.chars().collect();
    let mut source = String::new();
    let mut index = 0;
    while let Some(&char) = chars.get(index) {
        if char == '*' && chars.get(index + 1) == Some(&'*') {
            let slash = chars.get(index + 2) == Some(&'/');
            source.push_str(if slash { "(?:.*/)?" } else { ".*" });
            index += if slash { 2 } else { 1 };
        } else if char == '*' {
            source.push_str("[^/]*");
        } else if char == '?' {
            source.push_str("[^/]");
        } else {
            source.push_str(&escape_reg_exp(char.encode_utf8(&mut [0; 4])));
        }
        index += 1;
    }
    source
}

/// "Forward-slash normalized and case-insensitive."
#[must_use]
pub fn relevance_glob_matches(glob: &str, path: &str) -> bool {
    compiled(&format!("^{}$", glob_source(&slashes(glob))), "i")
        .is_some_and(|mut regexp| regexp.test(&slashes(path)))
}

/// `infra`, `infra/` and `infra/**` are one pattern: the directory and everything under it.
fn directory_base(pattern: &str) -> String {
    compiled(r"(?:\/\*\*)?\/*$", "").map_or_else(
        || slashes(pattern),
        |mut regexp| regexp.replace(&slashes(pattern), ""),
    )
}

fn relative_to(root: &str, path: &str) -> Option<String> {
    let slashed = slashes(root);
    let base = without_trailing_slashes(&slashed);
    let full = slashes(path);
    if full.to_lowercase() == base.to_lowercase() {
        return Some(String::new());
    }
    full.to_lowercase()
        .starts_with(&format!("{}/", base.to_lowercase()))
        .then(|| slice_code_units(&full, code_unit_length(base) + 1, None))
}

/// A `cwd` pattern: matched against the absolute working directory and, inside a repository, against
/// its path relative to the root; every pattern "matches the directory itself and everything under it".
#[must_use]
pub fn cwd_matches(pattern: &str, cwd: &str, repository_root: Option<&str>) -> bool {
    let base = directory_base(pattern);
    let under = |path: &str| -> bool {
        compiled(&format!("^{}(?:/.*)?$", glob_source(&base)), "i")
            .is_some_and(|mut regexp| regexp.test(path))
    };
    if under(&slashes(cwd)) {
        return true;
    }
    let relative = repository_root.and_then(|root| relative_to(root, cwd));
    relative.is_some_and(|relative| !relative.is_empty() && under(&relative))
}

const ENV_ASSIGNMENT: &str = "^[A-Za-z_][A-Za-z0-9_]*=";
const LEADING_COMMAND: &str = r"^(.*?)(?:&&|\|\||[;|&\n]|$)";

/// "One command name for each shell command Claude runs: the first token after any leading environment
/// variable assignments and `sudo`. Compound commands contribute only their leading command", so
/// `cd infra && terraform plan` records `cd`.
#[must_use]
pub fn command_name(command: &str) -> Option<String> {
    let leading = compiled(LEADING_COMMAND, "s")
        .and_then(|mut regexp| regexp.exec(command))
        .and_then(|found| found.captures.get(1).cloned().flatten())
        .unwrap_or_default();
    let tokens: Vec<String> = compiled(r"\s+", "")
        .map(|regexp| regexp.split(&leading))
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .filter(|token| !token.is_empty())
        .collect();
    let rest: Vec<&String> = (0..tokens.len())
        .filter(|index| !is_skipped(&tokens, *index))
        .filter_map(|index| tokens.get(index))
        .collect();
    rest.first().map(|token| (*token).clone())
}

/// Leading assignments and `sudo` words are skipped; the first other token stops the skipping.
fn is_skipped(tokens: &[String], index: usize) -> bool {
    let mut assignment = compiled(ENV_ASSIGNMENT, "");
    tokens
        .get(..=index)
        .unwrap_or_default()
        .iter()
        .all(|token| {
            assignment.as_mut().is_some_and(|regexp| regexp.test(token)) || token == "sudo"
        })
}

const URL_HOST: &str = r"https?:\/\/([A-Za-z0-9.-]+)";

/// The hostnames of the `http://` and `https://` URLs in a Bash command, lowercased.
#[must_use]
pub fn command_hosts(command: &str) -> Vec<String> {
    compiled(URL_HOST, "gi")
        .and_then(|regexp| regexp.match_all(command).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|found| {
            found
                .captures
                .get(1)
                .cloned()
                .flatten()
                .unwrap_or_default()
                .to_lowercase()
        })
        .collect()
}

fn regex_test(source: &str, flags: &str, text: &str) -> bool {
    RegExp::new(source, flags).is_ok_and(|mut regexp| regexp.test(text))
}

type Matcher = fn(&RelevanceSignals, &SessionFacts) -> bool;

const MATCHERS: [(SignalName, Matcher); 5] = [
    (SignalName::Cwd, |signals, facts| {
        signals
            .cwd
            .iter()
            .flatten()
            .any(|pattern| cwd_matches(pattern, &facts.cwd, facts.repository_root.as_deref()))
    }),
    (SignalName::Cli, |signals, facts| {
        let names: Vec<Option<String>> = facts
            .commands
            .iter()
            .map(|command| command_name(command))
            .collect();
        signals
            .cli
            .iter()
            .flatten()
            .any(|name| names.contains(&Some(name.clone())))
    }),
    (SignalName::Hosts, |signals, facts| {
        let seen: Vec<String> = facts
            .commands
            .iter()
            .flat_map(|command| command_hosts(command))
            .collect();
        signals
            .hosts
            .iter()
            .flatten()
            .any(|host| seen.contains(&host.to_lowercase()))
    }),
    (SignalName::FilesRead, |signals, facts| {
        signals.files_read.iter().flatten().any(|glob| {
            facts
                .files_touched
                .iter()
                .any(|path| relevance_glob_matches(glob, path))
        })
    }),
    (SignalName::ManifestDeps, |signals, facts| {
        signals.manifest_deps.iter().flatten().any(|dep| {
            facts.manifests.iter().any(|manifest| {
                manifest
                    .size
                    .unwrap_or_else(|| length_number(code_unit_length(&manifest.content)))
                    <= f64::from(MANIFEST_SIZE_LIMIT)
                    && regex_test(&dep.file, "i", &manifest.path)
                    && regex_test(&dep.pattern, "", &manifest.content)
            })
        })
    }),
];

const ORDER: [SignalName; 5] = [
    SignalName::Cwd,
    SignalName::Cli,
    SignalName::Hosts,
    SignalName::FilesRead,
    SignalName::ManifestDeps,
];

/// The signals that match, in the docs' order. Empty when none is set: "only if at least one signal is set".
#[must_use]
pub fn matching_signals(
    signals: Option<&RelevanceSignals>,
    facts: &SessionFacts,
) -> Vec<SignalName> {
    let Some(signals) = signals else {
        return Vec::new();
    };
    ORDER
        .into_iter()
        .filter(|name| {
            MATCHERS
                .iter()
                .find(|(each, _)| each == name)
                .is_some_and(|(_, matcher)| matcher(signals, facts))
        })
        .collect()
}

/// `cwd` is "the only signal that can match at session start, before the user sends a first message".
#[must_use]
pub fn matches_at_session_start(
    signals: Option<&RelevanceSignals>,
    cwd: &str,
    repository_root: Option<&str>,
) -> bool {
    signals
        .and_then(|signals| signals.cwd.as_ref())
        .into_iter()
        .flatten()
        .any(|pattern| cwd_matches(pattern, cwd, repository_root))
}

crate::registry::exports! {
    "marketplace/relevance-match" {
        "MANIFEST_SIZE_LIMIT" => ok || MANIFEST_SIZE_LIMIT;
        "relevanceGlobMatches" => ok |glob: String, path: String| relevance_glob_matches(&glob, &path);
        "cwdMatches" => ok |pattern: String, cwd: String, repository_root: Option<String>| cwd_matches(&pattern, &cwd, repository_root.as_deref());
        "commandName" => ok |command: String| command_name(&command);
        "commandHosts" => ok |command: String| command_hosts(&command);
        "matchingSignals" => ok |signals: Option<RelevanceSignals>, facts: SessionFacts| matching_signals(signals.as_ref(), &facts);
        "matchesAtSessionStart" => ok |signals: Option<RelevanceSignals>, cwd: String, repository_root: Option<String>| matches_at_session_start(signals.as_ref(), &cwd, repository_root.as_deref());
    }
}
