//! Port of `src/format/agent-flag.ts`, switched.
//!
//! The agents of the `--agents` flag (sub-agents.md, "Choose the subagent scope"): a JSON object whose
//! top-level keys are agent names and whose values are definitions. The host reads and parses the flag
//! (or the file `-p` takes); this reads the parsed value the way Claude Code reads a definition:
//! `prompt` is the system prompt, the other keys are the frontmatter fields of a file, and `color` and
//! `experimental` "aren't accepted here and are ignored rather than rejected".

use serde::Serialize;

use super::agent::{AgentBasics, RecordOrigin, agent_record};
use super::agent_field_read::{AgentFieldsOptions, AgentFieldsRead, read_agent_fields};
use super::agent_record::{AgentRecord, FlagOrigin, FlagSource};
use super::agent_unrecognized::unrecognized_fields;
use super::frontmatter_values::read_text;
use super::issues::{PluginIssue, Severity, error};
use crate::js::{JsObject, JsValue};
use crate::protocol::is_record::is_record;

const FLAG_PATH: &str = "--agents";

/// Fields a CLI-defined agent does not accept; they are dropped without a word.
const NOT_ACCEPTED: [&str; 2] = ["color", "experimental"];

/// Keys read here, not by the field registry.
const OWN_KEYS: [&str; 3] = ["prompt", "description", "name"];

/// What reading the flag gave.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FlagAgentsRead {
    /// The agents that load.
    pub agents: Vec<AgentRecord>,
    /// Errors name `--agents` or `--agents.<name>`; a refused definition is not in `agents`.
    pub issues: Vec<PluginIssue>,
}

/// One definition as read; `dashed` is the error of a name that starts with `-`, which loads nowhere.
struct OneRead {
    agents: Vec<AgentRecord>,
    issues: Vec<PluginIssue>,
    dashed: Option<PluginIssue>,
}

fn refused(path: &str, message: &str, dashed: Option<PluginIssue>) -> OneRead {
    OneRead {
        agents: vec![],
        issues: vec![error(path, message)],
        dashed,
    }
}

/// A value that is no object of definitions at all: nothing to name a `-` for.
fn refused_whole(message: &str) -> FlagAgentsRead {
    FlagAgentsRead {
        agents: vec![],
        issues: vec![error(FLAG_PATH, message)],
    }
}

/// The text of `prompt` (empty allowed), or why the definition is refused.
fn prompt_of(definition: &JsObject) -> Option<String> {
    definition
        .get("prompt")
        .and_then(JsValue::as_str)
        .map(str::to_owned)
}

fn without(definition: &JsObject, keys: &[&str]) -> JsObject {
    definition
        .iter()
        .filter(|(key, _)| !keys.contains(&key.as_str()))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

/// The part of the definition the field registry reads, and which dropped fields it had.
fn fields_of(definition: &JsObject, path: &str) -> AgentFieldsRead {
    let keys: Vec<&str> = OWN_KEYS
        .iter()
        .chain(NOT_ACCEPTED.iter())
        .copied()
        .collect();
    let options = AgentFieldsOptions {
        plugin: false,
        path: path.to_owned(),
    };
    let mut read = read_agent_fields(&without(definition, &keys), &options);
    let dropped = NOT_ACCEPTED
        .into_iter()
        .filter(|field| definition.contains_key(*field));
    read.ignored.extend(dropped);
    read
}

/// A problem with the name itself; a name that starts with `-` is not one (it is checked after the
/// schema). Oracle 2.1.322 (spec/judgments.md): the binary loads a key with `:` or over 256 characters,
/// so neither file rule binds the flag; an empty key is the kit's refusal.
fn name_problem(name: &str) -> Option<&'static str> {
    name.is_empty().then_some("is empty")
}

/// What `--agent <name>` finds a flag agent by without its scope: the last `:` segment, as for a
/// plugin agent (oracle 2.1.322: `--agent c` runs `a:b:c`). A name without `:` is its own.
fn bare_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn read_one(name: &str, definition: &JsValue) -> OneRead {
    let path = format!("{FLAG_PATH}.{name}");
    let dashed = name.starts_with('-').then(|| {
        error(
            FLAG_PATH,
            &format!("{name}: agent names must not start with '-'"),
        )
    });
    if let Some(problem) = name_problem(name) {
        return refused(
            &path,
            &format!("Invalid agent name \"{name}\": it {problem}"),
            None,
        );
    }
    let Some(definition) = definition.as_object().filter(|_| is_record(definition)) else {
        return refused(&path, "The definition is not an object", dashed);
    };
    let raw_description = definition.get("description").unwrap_or(&JsValue::Undefined);
    let Some(description) = read_text("description", raw_description).value else {
        return refused(
            &path,
            &format!("Agent \"{name}\" has no \"description\""),
            dashed,
        );
    };
    let Some(prompt) = prompt_of(definition) else {
        let message = format!("Agent \"{name}\" has no \"prompt\" (it may be an empty string)");
        return refused(&path, &message, dashed);
    };
    let fields = fields_of(definition, &path);
    if dashed.is_some() {
        return OneRead {
            agents: vec![],
            issues: fields.issues,
            dashed,
        };
    }
    let origin = FlagOrigin {
        path,
        source: FlagSource::Flag,
    };
    let basics = AgentBasics {
        name: bare_name(name).to_owned(),
        description,
        prompt,
    };
    let unknown_keys: Vec<&str> = std::iter::once("prompt").chain(NOT_ACCEPTED).collect();
    let mut agent = agent_record(
        RecordOrigin::Flag(&origin),
        basics,
        &fields,
        unrecognized_fields(&without(definition, &unknown_keys)),
    );
    name.clone_into(&mut agent.scoped_name);
    OneRead {
        agents: vec![agent],
        issues: fields.issues,
        dashed,
    }
}

/// errors.md: "When there are more than 20 problem lines, Claude Code prints the first 20 and replaces the rest with `…and N more`."
const MAX_PROBLEM_LINES: usize = 20;

fn capped(problems: Vec<PluginIssue>) -> Vec<PluginIssue> {
    if problems.len() <= MAX_PROBLEM_LINES {
        return problems;
    }
    let more = problems.len() - MAX_PROBLEM_LINES;
    let mut kept: Vec<PluginIssue> = problems.into_iter().take(MAX_PROBLEM_LINES).collect();
    kept.push(error(FLAG_PATH, &format!("…and {more} more")));
    kept
}

fn is_error(issue: &PluginIssue) -> bool {
    issue.severity == Severity::Error
}

/// Reads the parsed `--agents` value. An invalid value loads no agent at all: Claude Code exits with
/// code 1 and prints the first failing check only (§5.5.1), so `agents` is empty while any error is
/// reported, and the host refuses to start the session. Judgment (spec/judgments.md): the
/// docs do not say which keys a definition must
/// have, so `description` and `prompt` are required (they are the two fields every subagent has).
#[must_use]
pub fn read_flag_agents(raw: &JsValue) -> FlagAgentsRead {
    let Some(raw) = raw.as_object().filter(|_| is_record(raw)) else {
        return refused_whole("The value is not a JSON object of agent names to definitions");
    };
    let reads: Vec<OneRead> = raw
        .iter()
        .map(|(name, definition)| read_one(name, definition))
        .collect();
    let issues: Vec<PluginIssue> = reads.iter().flat_map(|read| read.issues.clone()).collect();
    let schema: Vec<PluginIssue> = issues.iter().filter(|i| is_error(i)).cloned().collect();
    // errors.md orders the checks: the schema problems of every definition first, and the `-` names only
    // when none is left ("you see the second only after you fix the first").
    let dashed: Vec<PluginIssue> = reads
        .iter()
        .filter_map(|read| read.dashed.clone())
        .collect();
    let reported = if schema.is_empty() { dashed } else { schema };
    let agents = if reported.is_empty() {
        reads.into_iter().flat_map(|read| read.agents).collect()
    } else {
        vec![]
    };
    let mut all = capped(reported);
    all.extend(issues.into_iter().filter(|i| !is_error(i)));
    FlagAgentsRead {
        agents,
        issues: all,
    }
}

crate::registry::exports! {
    "format/agent-flag" {
        "readFlagAgents" => ok |raw: JsValue| read_flag_agents(&raw);
    }
}
