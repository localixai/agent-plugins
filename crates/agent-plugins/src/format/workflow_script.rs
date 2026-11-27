//! Port of `src/format/workflow-script.ts`, switched.
//!
//! What the kit can say about a workflow script without running it (workflows.md, "Edit a saved
//! script", "Behavior and limits"): the stage titles `phase()` is called with against `meta.phases`,
//! whether the body reads `args`, the calls that throw inside the script (`Date.now()`,
//! `Math.random()`, a no-argument `new Date()`) and `import()`, which fails before the run starts.
//! Comments, strings, template text and regular expressions are never code. A syntax error in the
//! body is the runtime's to report ("Claude Code reports it when you run the workflow"), not the
//! kit's.

use indexmap::IndexSet;
use serde::Serialize;

use super::issues::{PluginIssue, error, warning};
use super::js_tokens::{JsToken, TokenForm};
use super::workflow_meta::{ScriptScope, WorkflowMeta, read_script_head};
use crate::js::utf16::{slice, text_of, units};

/// The stage titles of a script against its `meta.phases`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhaseReport {
    /// The titles of `meta.phases`.
    pub declared: Vec<String>,
    /// The titles `phase()` is called with as a literal, once each, in order.
    pub used: Vec<String>,
    /// Used and not declared: "a `phase()` title with no entry gets a progress group of its own".
    pub own_group: Vec<String>,
    /// Declared and never used.
    pub unused: Vec<String>,
}

/// What the kit can say about a script.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptReport {
    /// Its `meta`, when it reads.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<WorkflowMeta>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
    /// The script up to the end of the `meta` statement; `header + body` is the script.
    pub header: String,
    /// The rest.
    pub body: String,
    /// The stage titles.
    pub phases: PhaseReport,
    /// The body reads the `args` global (workflows.md, "Pass input to a saved workflow").
    pub reads_args: bool,
}

type Tokens<'a> = &'a [JsToken];

/// The token `index` places from the start, `None` before it or past the end.
fn token_at(tokens: Tokens<'_>, index: usize, ahead: usize, back: usize) -> Option<&JsToken> {
    (index + ahead)
        .checked_sub(back)
        .and_then(|at| tokens.get(at))
}

fn is_punct(token: Option<&JsToken>, text: &str) -> bool {
    token.is_some_and(|token| token.form == TokenForm::Punct && token.text == text)
}

fn is_word(token: Option<&JsToken>, text: &str) -> bool {
    token.is_some_and(|token| token.form == TokenForm::Word && token.text == text)
}

fn is_call(tokens: Tokens<'_>, index: usize) -> bool {
    is_punct(token_at(tokens, index, 1, 0), "(") && !is_punct(token_at(tokens, index, 0, 1), ".")
}

/// The titles `phase()` is called with: `titles` as literals, and how many calls had another
/// argument.
struct PhaseCalls {
    titles: Vec<String>,
    computed: usize,
}

fn phase_calls(tokens: Tokens<'_>) -> PhaseCalls {
    let mut titles = Vec::new();
    let mut computed = 0;
    for (index, token) in tokens.iter().enumerate() {
        if !is_word(Some(token), "phase") || !is_call(tokens, index) {
            continue;
        }
        let title = token_at(tokens, index, 2, 0);
        let after = token_at(tokens, index, 3, 0);
        let literal = title
            .filter(|title| title.form == TokenForm::String)
            .and_then(|title| title.value.clone());
        match literal {
            Some(text) if is_punct(after, ")") || is_punct(after, ",") => titles.push(text),
            _ => computed += 1,
        }
    }
    PhaseCalls { titles, computed }
}

fn unique(titles: &[String]) -> Vec<String> {
    titles
        .iter()
        .cloned()
        .collect::<IndexSet<_>>()
        .into_iter()
        .collect()
}

fn phase_report(declared: &[String], calls: &PhaseCalls) -> PhaseReport {
    let used = unique(&calls.titles);
    let own_group = if declared.is_empty() {
        Vec::new()
    } else {
        used.iter()
            .filter(|title| !declared.contains(title))
            .cloned()
            .collect()
    };
    let unused = if calls.computed > 0 {
        Vec::new()
    } else {
        declared
            .iter()
            .filter(|title| !used.contains(title))
            .cloned()
            .collect()
    };
    PhaseReport {
        declared: declared.to_vec(),
        used,
        own_group,
        unused,
    }
}

fn phase_issues(report: &PhaseReport) -> Vec<PluginIssue> {
    let own = report.own_group.iter().map(|title| {
        warning(
            "meta.phases",
            &format!(
                "phase(\"{title}\") has no entry in meta.phases, so it gets a progress group of its own"
            ),
        )
    });
    let unused = report.unused.iter().map(|title| {
        warning(
            "meta.phases",
            &format!("meta.phases lists \"{title}\" but no phase(\"{title}\") call uses it"),
        )
    });
    own.chain(unused).collect()
}

/// `args` as a name: not a property (`ctx.args`) and not an object key (`{ args: 1 }`).
fn reads_args(tokens: Tokens<'_>) -> bool {
    tokens.iter().enumerate().any(|(index, token)| {
        if !is_word(Some(token), "args") || is_punct(token_at(tokens, index, 0, 1), ".") {
            return false;
        }
        let before = token_at(tokens, index, 0, 1).map_or("", |token| token.text.as_str());
        let key = is_punct(token_at(tokens, index, 1, 0), ":") && ["{", ","].contains(&before);
        !key
    })
}

const THROWS: &str =
    "throws inside a workflow script, so that a relaunched run repeats the same agent() calls";

/// How much a finding weighs.
#[derive(Clone, Copy)]
enum Severity {
    Error,
    Warning,
}

/// One finding of a scan over the tokens: where it is and what it says.
struct Finding {
    at: usize,
    severity: Severity,
    message: String,
}

/// The script's globals the scan looks for by name: the kit never calls them (the layers gate
/// forbids it), it only reads their names in a script.
const CLOCK: &str = "Date";
const RANDOM_OWNER: &str = "Math";

fn static_call(owner: &str, member: &str, tokens: Tokens<'_>) -> Vec<Finding> {
    let message =
        format!("{owner}.{member}() {THROWS}; pass a timestamp or a seed in through args instead");
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            is_word(Some(token), owner)
                && !is_punct(token_at(tokens, *index, 0, 1), ".")
                && is_punct(token_at(tokens, *index, 1, 0), ".")
                && is_word(token_at(tokens, *index, 2, 0), member)
                && is_punct(token_at(tokens, *index, 3, 0), "(")
        })
        .map(|(_, token)| Finding {
            at: token.start,
            severity: Severity::Warning,
            message: message.clone(),
        })
        .collect()
}

/// A no-argument construction of the clock global: with `()` and nothing inside, or without
/// parentheses and not followed by a member.
fn new_date(tokens: Tokens<'_>) -> Vec<Finding> {
    let message =
        format!("a no-argument new {CLOCK}() {THROWS}; pass a timestamp in through args instead");
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            if !is_word(Some(token), "new") || !is_word(token_at(tokens, *index, 1, 0), CLOCK) {
                return false;
            }
            let open = is_punct(token_at(tokens, *index, 2, 0), "(");
            if open {
                is_punct(token_at(tokens, *index, 3, 0), ")")
            } else {
                !is_punct(token_at(tokens, *index, 2, 0), ".")
            }
        })
        .map(|(_, token)| Finding {
            at: token.start,
            severity: Severity::Warning,
            message: message.clone(),
        })
        .collect()
}

/// "a script that contains `import()` fails before the run starts".
fn import_calls(tokens: Tokens<'_>) -> Vec<Finding> {
    let message = "import() fails before the run starts: a workflow script cannot load modules; put work that needs a library in an agent's task";
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| is_word(Some(token), "import") && is_call(tokens, *index))
        .map(|(_, token)| Finding {
            at: token.start,
            severity: Severity::Error,
            message: message.to_owned(),
        })
        .collect()
}

/// The scans over the tokens, in order (`SCANS`).
fn scans(tokens: Tokens<'_>) -> Vec<Finding> {
    let mut found = static_call(CLOCK, "now", tokens);
    found.extend(static_call(RANDOM_OWNER, "random", tokens));
    found.extend(new_date(tokens));
    found.extend(import_calls(tokens));
    found
}

/// Where each line of `source` starts, in one pass.
fn line_starts(source: &[u16]) -> Vec<usize> {
    let newline = u16::from(b'\n');
    std::iter::once(0)
        .chain(
            source
                .iter()
                .enumerate()
                .filter(|(_, unit)| **unit == newline)
                .map(|(at, _)| at + 1),
        )
        .collect()
}

/// The 1-based line of offset `at`: the count of line starts at or before it (binary search).
fn line_of(starts: &[usize], at: usize) -> usize {
    starts.partition_point(|start| *start <= at)
}

fn scan_issues(source: &[u16], tokens: Tokens<'_>) -> Vec<PluginIssue> {
    let starts = line_starts(source);
    let mut found = scans(tokens);
    found.sort_by_key(|finding| finding.at);
    found
        .into_iter()
        .map(|found| {
            let path = format!("script:{}", line_of(&starts, found.at));
            match found.severity {
                Severity::Error => error(&path, &found.message),
                Severity::Warning => warning(&path, &found.message),
            }
        })
        .collect()
}

/// Inspects a workflow script. Never runs it and never throws.
#[must_use]
pub fn inspect_workflow_script(source: &str) -> ScriptReport {
    let head = read_script_head(source, ScriptScope::Whole);
    if head.object.is_none() {
        return ScriptReport {
            meta: None,
            issues: head.issues,
            header: String::new(),
            body: source.to_owned(),
            phases: PhaseReport::default(),
            reads_args: false,
        };
    }
    let tokens = head.tokens.get(head.body_index..).unwrap_or_default();
    let declared = head.meta.as_ref().map_or(&[][..], |meta| &meta.phases[..]);
    let phases = phase_report(declared, &phase_calls(tokens));
    let text = units(source);
    let mut issues = head.issues.clone();
    issues.extend(phase_issues(&phases));
    issues.extend(scan_issues(&text, tokens));
    ScriptReport {
        issues,
        header: text_of(slice(&text, 0, head.head_end)),
        body: text_of(slice(&text, head.head_end, text.len())),
        reads_args: reads_args(tokens),
        phases,
        meta: head.meta,
    }
}

crate::registry::exports! {
    "format/workflow-script" {
        "inspectWorkflowScript" => ok |source: String| inspect_workflow_script(&source);
    }
}
