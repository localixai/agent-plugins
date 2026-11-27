//! Port of `src/rules/path/symlink.ts`, switched.
//!
//! Symlinks (permissions.md, "Symlinks"): "the permission check covers two paths: the one Claude
//! requested and the file it resolves to". Matching reads `PreparedCall.resolvedPaths`; this file
//! fills it through the host's `PathResolver`.
//!
//! The call `prepareCall` is given is kept as the value the host passed (`{ ...call,
//! resolvedPaths }` keeps every member it has, not only those a `ToolCall` names), and answered
//! as an equal value, not the very object.

use serde::{Deserialize, Serialize};

use crate::js::{JsObject, JsValue};
use crate::ports::path_resolver::PathResolver;
use crate::ports::{PortFailure, Signal};
use crate::registry::{Map, Port};
use crate::rules::path::anchor::anchor_pattern;
use crate::rules::path::file_rules::path_specifier_of;
use crate::rules::path::posix::{
    dirname_of, has_glob_syntax, normalize_absolute, resolve_path, to_posix_path,
};
use crate::rules::path::tools::requested_path;
use crate::rules::restricts::restricts;
use crate::rules::types::{PathContext, PermissionRule, Platform, ToolCall};

/// Where `path` really is. A file about to be written does not exist, but "a directory on the way
/// to the file" may be a link, so the nearest existing parent is resolved and the rest appended.
/// A `PortFailure` (a loop) is returned, never swallowed: "the Read, Edit, and Write tools refuse
/// the operation".
async fn real_location(
    path: &str,
    resolver: &dyn PathResolver,
    signal: Option<&Signal>,
) -> Option<Result<String, PortFailure>> {
    let mut missing: Vec<String> = Vec::new();
    let mut current = path.to_owned();
    loop {
        if let Some(real) = resolver.realpath(&current, signal).await {
            return Some(real.map(|real| resolve_path(&real, &missing.join("/"))));
        }
        if current == "/" {
            return None;
        }
        let parent = dirname_of(&current);
        let cut = if parent == "/" { 1 } else { parent.len() + 1 };
        missing.insert(0, current.get(cut..).unwrap_or_default().to_owned());
        current = parent;
    }
}

/// `{ signal }`, the options of both exports.
#[derive(Debug, Default, Deserialize)]
pub struct ResolveOptions {
    /// Cancels the host's resolving.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// `PreparedCall | PortFailure`.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Prepared {
    /// The call, with the real location of its path when that is another.
    Call(JsValue),
    /// The host could not tell where the path leads.
    Failure(PortFailure),
}

/// A call with the real location of its path, ready for the path rules. A call whose path is not
/// a link comes back as it was; one with no path is untouched and the resolver is not asked.
pub async fn prepare_call(
    call: JsValue,
    paths: &PathContext,
    resolver: &dyn PathResolver,
    signal: Option<&Signal>,
) -> Prepared {
    let requested = ToolCall::deserialize(call.clone())
        .ok()
        .and_then(|read| requested_path(&read, paths));
    let Some(requested) = requested else {
        return Prepared::Call(call);
    };
    match real_location(&requested, resolver, signal).await {
        None => Prepared::Call(call),
        Some(Err(failure)) => Prepared::Failure(failure),
        Some(Ok(real)) if real == requested => Prepared::Call(call),
        Some(Ok(real)) => {
            let mut members = if let JsValue::Object(members) = call {
                members
            } else {
                JsObject::new()
            };
            members.insert(
                "resolvedPaths".to_owned(),
                JsValue::Array(vec![JsValue::String(real)]),
            );
            Prepared::Call(JsValue::Object(members))
        }
    }
}

/// The directories a deny or ask rule is written through: its anchor (the current directory of a
/// `./` rule, the home of a `~/` rule, the settings source's folder of a `/` rule) and, for a
/// pattern that is not the current directory's own, the directory it names before its first
/// wildcard (`/vendor/pkg/*.js` names `vendor/pkg`). A `./` rule may match at any depth below its
/// anchor, so the anchor alone stands for it.
fn rule_directories(rule: &PermissionRule, paths: &PathContext) -> Vec<String> {
    let Some(specifier) = path_specifier_of(rule).filter(|specifier| !specifier.negated) else {
        return Vec::new();
    };
    let anchored = anchor_pattern(&specifier, &rule.source, paths);
    if anchored.under_cwd {
        return vec![anchored.root];
    }
    let literal: Vec<&str> = anchored
        .pattern
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();
    let kept = match literal.iter().position(|segment| has_glob_syntax(segment)) {
        None => &literal[..],
        Some(stop) => literal.get(..stop).unwrap_or_default(),
    };
    if kept.is_empty() {
        vec![anchored.root]
    } else {
        let named = resolve_path(&anchored.root, &kept.join("/"));
        vec![anchored.root, named]
    }
}

/// The working directories in the kit's path form, which the scope checks read
/// (`decide/scope.rs`).
fn working_directories(paths: &PathContext) -> Vec<String> {
    std::iter::once(&paths.primary_working_directory)
        .chain(&paths.additional_directories)
        .map(|directory| normalize_absolute(&to_posix_path(directory, paths.platform)))
        .collect()
}

/// The real location of every directory a `//`, `~/`, `/` or `./` deny or ask rule is written
/// through, and of every working directory: "On macOS and Linux, a deny or ask rule written
/// through a symlinked directory ... also applies at the directory's real location. For example,
/// on macOS, where `/etc` resolves to `/private/etc`, `Read(//etc/**)` blocks `/private/etc/hosts`
/// too." The anchor counts as such a directory, so a `./private/**` rule under a current directory
/// reached through a link (`/var/proj` is `/private/var/proj`) covers the real spelling too, and a
/// working directory reached through a link holds the files that resolve below its real location
/// (judgments, spec/judgments.md). Only the directories that are links come back. A host puts the
/// map in `PathContext.realDirectories`. Windows has none ("On macOS and Linux").
pub async fn resolve_rule_directories(
    rules: &[PermissionRule],
    paths: &PathContext,
    resolver: &dyn PathResolver,
    signal: Option<&Signal>,
) -> Map<String, String> {
    let mut found = Map {
        entries: Vec::new(),
    };
    if paths.platform != Platform::Posix {
        return found;
    }
    let mut asked: Vec<String> = Vec::new();
    let restricting = rules.iter().filter(|rule| restricts(rule.behavior));
    let directories = restricting
        .flat_map(|rule| rule_directories(rule, paths))
        .chain(working_directories(paths));
    for directory in directories {
        if directory != "/" && !asked.contains(&directory) {
            asked.push(directory);
        }
    }
    for directory in asked {
        if let Some(Ok(real)) = resolver.realpath(&directory, signal).await
            && real != directory
        {
            found.entries.push((directory, real));
        }
    }
    found
}

crate::registry::exports! {
    "rules/path/symlink" {
        "prepareCall" => async ok |call: JsValue, paths: PathContext, resolver: Port, options: Option<ResolveOptions>| {
            let options = options.unwrap_or_default();
            prepare_call(call, &paths, &resolver, options.signal.as_ref()).await
        };
        "resolveRuleDirectories" => async ok |rules: Vec<PermissionRule>, paths: PathContext, resolver: Port, options: Option<ResolveOptions>| {
            let options = options.unwrap_or_default();
            resolve_rule_directories(&rules, &paths, &resolver, options.signal.as_ref()).await
        };
    }
}
