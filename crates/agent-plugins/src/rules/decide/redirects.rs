//! Ported from `src/rules/decide/redirects.ts`; only the core uses it now, so it has no wrapper.

use super::context::DecideContext;
use super::protected_paths::is_protected_path;
use super::scope::in_scope;
use crate::js::{JsObject, JsValue, slice_code_units};
use crate::rules::bash::split::scan_command;
use crate::rules::bash::words::{RedirectKind, ShellCommand, ShellWord};
use crate::rules::evaluate::evaluate_rules;
use crate::rules::path::posix::{HostPaths, absolute_path};
use crate::rules::types::{PathContext, PermissionBehavior, PreparedCall};

/// Redirections (permissions.md, "Redirections"): "Claude Code checks the redirect target against
/// your file rules as if Claude wrote or read that file directly". An output target (`>`, `>>`,
/// `2>`, and the files a `tee` writes) goes through the `Edit` rules, protected paths and the
/// working directories; an input target (`<`) through the `Read` rules and the working
/// directories. "Targets with no file behind them aren't checked: `/dev/null`, file-descriptor
/// forms such as `2>&1` and `<&3`, and here-docs and here-strings." Judgment (spec/judgments.md): a
/// relative target after a `cd` in the same command, output or input, cannot be told from the
/// text, so it needs approval; so does a target a variable or substitution hides.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the package's shape: one fact per flag"
)]
pub struct RedirectFacts {
    /// A target is denied by an `Edit` rule (output) or a `Read` rule (input): "deny rules block
    /// in every mode".
    pub denied: bool,
    /// A target needs approval, whatever allow rule covers the command.
    pub approval: bool,
    /// An output target is a protected path.
    pub protected_write: bool,
    /// A redirect writes a file that only the working directories cover, no `Edit` allow rule: "as
    /// if Claude wrote ... that file directly", which only a mode that accepts such an edit lets
    /// through.
    pub in_scope_write: bool,
}

/// `NO_REDIRECTS`.
pub const NO_REDIRECTS: RedirectFacts = RedirectFacts {
    denied: false,
    approval: false,
    protected_write: false,
    in_scope_write: false,
};

/// Where a target comes from, which sets how it is judged:
/// - `redirect`: a redirection; a glob or an unsure text needs approval, a write in scope is an
///   edit;
/// - `tee`: a file `tee` writes, judged as a redirect but `Bash(tee *)` covers one in scope;
/// - `operand`: a file a recognized file command names (`file_targets.rs`): a Bash rule covers one
///   in scope; a glob is judged by its directory, a `..` after a directory segment needs approval;
/// - `unchecked`: a file a command names that Claude Code does not path-check (`less`, `chmod`):
///   only a `Read` or `Edit` deny or ask rule applies (docs/SECURITY.md).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetSource {
    /// `redirect`.
    Redirect,
    /// `tee`.
    Tee,
    /// `operand`.
    Operand,
    /// `unchecked`.
    Unchecked,
}

/// `Target['kind']`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TargetKind {
    /// `output`.
    Output,
    /// `input`.
    Input,
}

/// A file a command names or redirects to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    /// `word`.
    pub word: ShellWord,
    /// `kind`.
    pub kind: TargetKind,
    /// A `cd` ran earlier in the command (an operand is read against the starting directory).
    pub after_cd: bool,
    /// `source`.
    pub source: TargetSource,
}

impl Target {
    /// A target of `word`.
    #[must_use]
    pub fn new(word: &ShellWord, kind: TargetKind, after_cd: bool, source: TargetSource) -> Self {
        Self {
            word: word.clone(),
            kind,
            after_cd,
            source,
        }
    }
}

/// The directions of each redirection: `<>` both reads and writes; descriptors and here-documents
/// name no file.
const fn directions_of(kind: RedirectKind) -> &'static [TargetKind] {
    match kind {
        RedirectKind::Output => &[TargetKind::Output],
        RedirectKind::Input => &[TargetKind::Input],
        RedirectKind::Both => &[TargetKind::Input, TargetKind::Output],
        RedirectKind::Descriptor | RedirectKind::Here => &[],
    }
}

const NO_FILE: &str = "/dev/null";

fn redirects_in(command: &ShellCommand, after_cd: bool) -> Vec<Target> {
    let mut targets = Vec::new();
    for redirect in &command.redirects {
        let named = redirect
            .target
            .as_ref()
            .filter(|target| target.value != NO_FILE);
        let Some(target) = named else { continue };
        for kind in directions_of(redirect.kind) {
            targets.push(Target::new(target, *kind, after_cd, TargetSource::Redirect));
        }
    }
    targets
}

/// The files a `tee` writes: its operands that are not options.
fn tee_targets(command: &ShellCommand, after_cd: bool) -> Vec<Target> {
    if command.words.first().map(|word| word.value.as_str()) != Some("tee") {
        return Vec::new();
    }
    command
        .words
        .iter()
        .skip(1)
        .filter(|word| !word.value.starts_with('-') && word.value != NO_FILE)
        .map(|word| Target::new(word, TargetKind::Output, after_cd, TargetSource::Tee))
        .collect()
}

/// The files a command's redirections and `tee`s read or write, in the order they run.
#[must_use]
pub fn redirect_targets(command: &str) -> Vec<Target> {
    let mut after_cd = false;
    let mut targets = Vec::new();
    for subcommand in &scan_command(command, None).commands {
        targets.extend(redirects_in(subcommand, after_cd));
        targets.extend(tee_targets(subcommand, after_cd));
        after_cd = after_cd
            || subcommand
                .words
                .first()
                .is_some_and(|word| word.value == "cd");
    }
    targets
}

fn relative_after_cd(text: &str, after_cd: bool) -> bool {
    after_cd && !text.starts_with('/') && !text.starts_with('~')
}

/// What differs between the two directions: the rules that judge the file, and when its text is
/// not enough.
struct Direction {
    tool: &'static str,
    unsure: fn(&str, bool) -> bool,
    checks_protected: bool,
}

const fn directions(kind: TargetKind) -> Direction {
    match kind {
        TargetKind::Output => Direction {
            tool: "Edit",
            unsure: |text, after_cd| text.starts_with('~') || relative_after_cd(text, after_cd),
            checks_protected: true,
        },
        TargetKind::Input => Direction {
            tool: "Read",
            unsure: relative_after_cd,
            checks_protected: false,
        },
    }
}

/// The absolute path of a target, or `None` when a variable or substitution hides it.
#[must_use]
pub fn literal_of(word: &ShellWord, paths: &PathContext) -> Option<String> {
    (!word.expands).then(|| absolute_path(&word.value, &HostPaths::from(paths)))
}

/// What the rules say about a file as the tool would see it, and whether a rule naming paths
/// allowed it.
#[derive(Default)]
struct RuleSays {
    behavior: Option<PermissionBehavior>,
    path_allowed: bool,
}

fn rule_says(literal: Option<&str>, tool: &str, ctx: &DecideContext) -> Result<RuleSays, String> {
    let Some(literal) = literal else {
        return Ok(RuleSays::default());
    };
    let input = JsObject::from([("file_path".to_owned(), JsValue::from(literal))]);
    let call = PreparedCall::of(tool, JsValue::Object(input));
    let verdict = evaluate_rules(&ctx.rules, &call, &ctx.match_context)?;
    // Oracle 2.1.322: a bare `Read` or `Edit` allow does not cover a Bash command's file outside.
    let path_allowed = verdict.behavior == Some(PermissionBehavior::Allow)
        && verdict.rule.is_some_and(|rule| rule.specifier.is_some());
    Ok(RuleSays {
        behavior: verdict.behavior,
        path_allowed,
    })
}

/// `sub/..`, `/tmp/../etc`: "may follow a symlink outside the working directory" (oracle 2.1.322).
fn is_name(segment: &str) -> bool {
    !segment.is_empty() && segment != "." && segment != ".."
}
fn traverses_after_directory(text: &str) -> bool {
    let segments: Vec<&str> = text.split('/').collect();
    segments.iter().enumerate().any(|(at, segment)| {
        *segment == ".."
            && segments
                .get(..at)
                .unwrap_or_default()
                .iter()
                .any(|s| is_name(s))
    })
}

/// The directory a glob operand lists (`/etc/*` is `/etc`), the operand itself otherwise.
fn listed_text(word: &ShellWord) -> String {
    if !word.globs {
        return word.value.clone();
    }
    let head = match word.value.find(['*', '?', '[']) {
        Some(at) => word.value.get(..at).unwrap_or_default().to_owned(),
        // `search` answers -1, and `slice(0, -1)` is every code unit but the last.
        None => {
            let last = word.value.encode_utf16().count().saturating_sub(1);
            slice_code_units(&word.value, 0, Some(last))
        }
    };
    match head.rfind('/') {
        None => ".".to_owned(),
        Some(0) => "/".to_owned(),
        Some(cut) => head.get(..cut).unwrap_or_default().to_owned(),
    }
}

/// How a target of each source is judged (`TargetSource`).
struct SourceJudgment {
    /// The text the rules read: an operand glob is judged by the directory it lists.
    text: fn(&ShellWord) -> String,
    /// Whether the text alone needs approval, before any rule; `literal` is `None` when it
    /// expands.
    unsure: fn(&Target, Option<&str>) -> bool,
    /// A file outside the working directories needs approval unless a path rule allows it.
    checks_outside: bool,
    /// An output target is checked against the protected paths.
    checks_protected: bool,
    /// An output target in scope with no `Edit` allow is an edit, for a mode that accepts one.
    edits_in_scope: bool,
}

fn redirect_unsure(target: &Target, literal: Option<&str>) -> bool {
    literal.is_none()
        || target.word.globs
        || (directions(target.kind).unsure)(&target.word.value, target.after_cd)
}

const REDIRECTED: SourceJudgment = SourceJudgment {
    text: |word| word.value.clone(),
    unsure: redirect_unsure,
    checks_outside: true,
    checks_protected: true,
    edits_in_scope: true,
};

const UNCHECKED: SourceJudgment = SourceJudgment {
    text: |word| word.value.clone(),
    unsure: |_, _| false,
    checks_outside: false,
    checks_protected: false,
    edits_in_scope: false,
};

const fn sources(source: TargetSource) -> SourceJudgment {
    match source {
        TargetSource::Redirect => REDIRECTED,
        TargetSource::Tee => SourceJudgment {
            edits_in_scope: false,
            ..REDIRECTED
        },
        TargetSource::Operand => SourceJudgment {
            text: listed_text,
            unsure: |target, literal| {
                literal.is_none() || traverses_after_directory(&target.word.value)
            },
            checks_outside: true,
            ..UNCHECKED
        },
        TargetSource::Unchecked => UNCHECKED,
    }
}

/// The absolute file a target names as its source reads it, and what the rules and the text say
/// of it: `{ literal, behavior, approval }`.
type Located = (Option<String>, Option<PermissionBehavior>, bool);

fn located(target: &Target, ctx: &DecideContext) -> Result<Located, String> {
    let paths = ctx.paths();
    let source = sources(target.source);
    let literal = (!target.word.expands)
        .then(|| absolute_path(&(source.text)(&target.word), &HostPaths::from(paths)));
    let says = rule_says(literal.as_deref(), directions(target.kind).tool, ctx)?;
    let outside = source.checks_outside
        && literal
            .as_deref()
            .is_some_and(|literal| !in_scope(literal, paths))
        && !says.path_allowed;
    let approval = says.behavior == Some(PermissionBehavior::Ask)
        || (source.unsure)(target, literal.as_deref())
        || outside;
    Ok((literal, says.behavior, approval))
}

/// What one target needs: a denial, an approval, a protected write, a write in scope, or nothing.
fn judge_target(target: &Target, ctx: &DecideContext) -> Result<RedirectFacts, String> {
    let (literal, behavior, approval) = located(target, ctx)?;
    let source = sources(target.source);
    let writes = directions(target.kind).checks_protected && source.checks_protected;
    Ok(RedirectFacts {
        denied: behavior == Some(PermissionBehavior::Deny),
        approval,
        in_scope_write: !approval
            && writes
            && source.edits_in_scope
            && behavior != Some(PermissionBehavior::Allow),
        protected_write: writes
            && literal
                .as_deref()
                .is_some_and(|path| is_protected_path(path, ctx.paths(), ctx.options())),
    })
}

/// A path the host's symlink check resolved, outside the working directories and no path rule
/// allowing it.
fn resolved_outside(path: &str, ctx: &DecideContext) -> Result<bool, String> {
    Ok(!in_scope(path, ctx.paths())
        && !rule_says(Some(path), "Read", ctx)?.path_allowed
        && !rule_says(Some(path), "Edit", ctx)?.path_allowed)
}

/// What the files a Bash command names and redirects to ask of the permission check (`targets`,
/// from `commandTargets`). Where the host's symlink check says a named file leads (`resolved`) is
/// judged as a named file outside: "Each path also goes through the symlink check".
///
/// # Errors
///
/// What a rule's matcher throws.
pub fn target_facts(
    targets: &[Target],
    ctx: &DecideContext,
    resolved: &[String],
) -> Result<RedirectFacts, String> {
    let judged = targets
        .iter()
        .map(|target| judge_target(target, ctx))
        .collect::<Result<Vec<_>, _>>()?;
    let names = targets
        .iter()
        .any(|target| sources(target.source).checks_outside);
    let approval = judged.iter().any(|fact| fact.approval);
    let mut resolved_out = false;
    for path in resolved.iter().filter(|_| names && !approval) {
        resolved_out = resolved_outside(path, ctx)?;
        if resolved_out {
            break;
        }
    }
    Ok(RedirectFacts {
        denied: judged.iter().any(|fact| fact.denied),
        approval: approval || resolved_out,
        protected_write: judged.iter().any(|fact| fact.protected_write),
        in_scope_write: judged.iter().any(|fact| fact.in_scope_write),
    })
}
