//! Ported from `src/rules/path/anchor.ts`; only the core uses it now, so it has no wrapper.

use serde::{Deserialize, Serialize};

use crate::js::slice_code_units;
use crate::rules::path::glob::is_usable_pattern;
use crate::rules::path::posix::{dirname_of, normalize_absolute, resolve_path, to_posix_path};
use crate::rules::types::{PathContext, RuleSource};

/// The four pattern types of permissions.md ("How path patterns match"): `//path` is an absolute
/// path from the filesystem root, `~/path` is from the home directory, `/path` is relative to the
/// settings source, and `path` or `./path` is relative to the current directory. Each anchor is a
/// root directory (`ANCHORS` in the TypeScript); the registry below is a `match` over the ids, so a
/// fifth type fails to compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AnchorId {
    /// `//path`.
    Root,
    /// `~/path`.
    Home,
    /// `/path`.
    Source,
    /// `path` or `./path`.
    Cwd,
}

/// A path pattern as written: its anchor, the gitignore text after the prefix, whether it is a
/// `!`, and whether it was written `./path` (the current directory named, not a gitignore name at
/// any depth).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PathSpecifier {
    /// The anchor.
    pub anchor: AnchorId,
    /// The gitignore text after the prefix.
    pub pattern: String,
    /// Whether it is a `!` negation.
    pub negated: bool,
    /// Whether it was written `./path`.
    pub dotted: bool,
}

/// The prefixes that anchor a pattern, longest first.
const PREFIXES: [(&str, AnchorId); 3] = [
    ("//", AnchorId::Root),
    ("~/", AnchorId::Home),
    ("/", AnchorId::Source),
];

/// A deny or ask pattern that starts with `!` is a negation. "Claude Code reads a `!` pattern
/// relative to the current directory even when `/`, `~/`, or `//` follows the `!`", so its text is
/// kept whole.
#[must_use]
pub fn read_path_specifier(text: &str) -> PathSpecifier {
    let negated = text.starts_with('!');
    let body = if negated {
        slice_code_units(text, 1, None)
    } else {
        text.to_owned()
    };
    if negated {
        return PathSpecifier {
            anchor: AnchorId::Cwd,
            pattern: body,
            negated,
            dotted: false,
        };
    }
    if let Some((lead, anchor)) = PREFIXES.iter().find(|(lead, _)| body.starts_with(lead)) {
        return PathSpecifier {
            anchor: *anchor,
            pattern: body.get(lead.len()..).unwrap_or_default().to_owned(),
            negated,
            dotted: false,
        };
    }
    let dotted = body.starts_with("./");
    PathSpecifier {
        anchor: AnchorId::Cwd,
        pattern: if dotted {
            body.get(2..).unwrap_or_default().to_owned()
        } else {
            body
        },
        negated,
        dotted,
    }
}

/// Where a `/path` pattern anchors, per settings source (permissions.md, "Where a pattern
/// anchors"): project, local, CLI and session rules at the primary working directory, user
/// settings at `~/.claude`, a `--settings <file>` at the directory of that file. The managed tiers
/// are not in the table; they anchor at the primary working directory like the command line (a
/// judgment).
fn source_root(source: &RuleSource, paths: &PathContext) -> String {
    match source {
        RuleSource::Project
        | RuleSource::Local
        | RuleSource::Cli
        | RuleSource::Session
        | RuleSource::Managed
        | RuleSource::EmbedderManaged => primary_directory(paths),
        RuleSource::User => format!("{}/.claude", home(paths)),
        RuleSource::FlagSettings { .. } => dirname_of(&flag_settings_file(source, paths)),
    }
}

fn folder(path: &str, paths: &PathContext) -> String {
    normalize_absolute(&to_posix_path(path, paths.platform))
}

fn primary_directory(paths: &PathContext) -> String {
    folder(&paths.primary_working_directory, paths)
}

fn home(paths: &PathContext) -> String {
    let directory = folder(&paths.home, paths);
    if directory == "/" {
        String::new()
    } else {
        directory
    }
}

fn flag_settings_file(source: &RuleSource, paths: &PathContext) -> String {
    let file = if let RuleSource::FlagSettings { file } = source {
        file
    } else {
        &paths.primary_working_directory
    };
    resolve_path(
        &primary_directory(paths),
        &to_posix_path(file, paths.platform),
    )
}

/// `text || '/'`.
fn or_root(text: String) -> String {
    if text.is_empty() {
        "/".to_owned()
    } else {
        text
    }
}

fn anchor_root(anchor: AnchorId, source: &RuleSource, paths: &PathContext) -> String {
    match anchor {
        AnchorId::Root => "/".to_owned(),
        AnchorId::Home => or_root(home(paths)),
        AnchorId::Source => or_root(source_root(source, paths)),
        AnchorId::Cwd => folder(&paths.cwd, paths),
    }
}

/// A pattern placed under a directory: the gitignore text is relative to `root`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchored {
    /// The directory the pattern is relative to.
    pub root: String,
    /// The gitignore text.
    pub pattern: String,
    /// Whether the pattern is the current directory's own (no `/`, `~/`, `//` and no `../` climb).
    pub under_cwd: bool,
}

/// `CLIMB`, `/^\.\.\//`.
const CLIMB: &str = "../";

/// A bare name or glob with no slash, the only gitignore form that matches at any depth by itself.
fn is_name_only(pattern: &str) -> bool {
    is_usable_pattern(pattern) && !pattern.contains('/')
}

/// The pattern placed under its anchor's directory.
#[must_use]
pub fn anchor_pattern(
    specifier: &PathSpecifier,
    source: &RuleSource,
    paths: &PathContext,
) -> Anchored {
    let mut root = anchor_root(specifier.anchor, source, paths);
    let mut pattern = specifier.pattern.clone();
    let mut climbed = false;
    // A relative pattern may name a parent (`../shared/**`); the docs are silent, so it is
    // resolved against the current directory like any relative path (spec/judgments.md).
    while specifier.anchor == AnchorId::Cwd && pattern.starts_with(CLIMB) {
        root = dirname_of(&root);
        // `CLIMB` is ASCII and `pattern` starts with it, so the cut is between characters.
        pattern.drain(..CLIMB.len());
        climbed = true;
    }
    // `//`, `~/` and `/` patterns are paths from their root: gitignore anchors them with a slash.
    // So is `./name`: permissions.md reads `Read(./.env)` as "the `.env` file in the current
    // directory", apart from `Read(.env)`, which is any `.env` at or under it (a gitignore name
    // matches at every depth).
    let rooted = specifier.anchor != AnchorId::Cwd || (specifier.dotted && is_name_only(&pattern));
    let text = if rooted {
        format!("/{pattern}")
    } else {
        pattern
    };
    Anchored {
        root,
        pattern: text,
        under_cwd: specifier.anchor == AnchorId::Cwd && !climbed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_prefix_names_the_anchor_and_a_negation_stays_whole() {
        assert_eq!(read_path_specifier("//etc/x").anchor, AnchorId::Root);
        assert_eq!(read_path_specifier("~/x").pattern, "x");
        let negated = read_path_specifier("!/x");
        assert_eq!(
            (negated.anchor, negated.pattern.as_str()),
            (AnchorId::Cwd, "/x")
        );
        assert!(read_path_specifier("./.env").dotted);
    }
}
