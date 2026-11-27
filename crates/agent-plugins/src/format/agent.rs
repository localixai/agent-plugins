//! Port of `src/format/agent.ts`, switched.
//!
//! One agent file read as Claude Code would load it (sub-agents.md, "Frontmatter reference"):
//! a project, user or managed file, or a plugin's.

use super::agent_field_read::{AgentFieldsOptions, AgentFieldsRead, read_agent_fields};
use super::agent_name::{MAX_AGENT_NAME, agent_file_name, agent_name_problem, scoped_agent_name};
use super::agent_record::{
    AgentExtras, AgentOrigin, AgentReadResult, AgentRecord, AgentSource, FileOrigin, FileSource,
    FlagOrigin, PluginOfOrigin, PluginOrigin, SkipReason, UnrecognizedField,
};
use super::agent_unrecognized::unrecognized_fields;
use super::frontmatter::{Frontmatter, parse_frontmatter};
use super::frontmatter_values::read_text;
use super::issues::{PluginIssue, error, warning};
use crate::js::{JsObject, JsValue, code_unit_length};

/// What `agentRecord` is given as its origin: `AgentOrigin | FlagOrigin`.
#[derive(Clone, Copy, Debug)]
pub enum RecordOrigin<'a> {
    /// A file's origin.
    Agent(&'a AgentOrigin),
    /// An agent of the `--agents` flag.
    Flag(&'a FlagOrigin),
}

/// `origin.plugin`, `origin.source` and `origin.path` of a [`RecordOrigin`].
fn origin_parts(origin: RecordOrigin<'_>) -> (Option<&PluginOfOrigin>, AgentSource, &str) {
    match origin {
        RecordOrigin::Agent(AgentOrigin::File(file)) => (None, file_source(file), &file.path),
        RecordOrigin::Agent(AgentOrigin::Plugin(plugin)) => {
            (Some(&plugin.plugin), AgentSource::Plugin, &plugin.path)
        }
        RecordOrigin::Flag(flag) => (None, AgentSource::Flag, &flag.path),
    }
}

const fn file_source(file: &FileOrigin) -> AgentSource {
    match file.source {
        FileSource::Managed => AgentSource::Managed,
        FileSource::Project => AgentSource::Project,
        FileSource::User => AgentSource::User,
    }
}

/// `{ name, description, prompt }` of `agentRecord`.
#[derive(Clone, Debug)]
pub struct AgentBasics {
    /// The agent's name.
    pub name: String,
    /// Its description.
    pub description: String,
    /// Its system prompt.
    pub prompt: String,
}

fn skipped(reason: SkipReason, issues: Vec<PluginIssue>) -> AgentReadResult {
    AgentReadResult::Skipped {
        reason,
        agent: JsValue::Undefined,
        issues,
    }
}

/// `problem` is set exactly when the YAML does not parse or is not a table.
const fn unparsed(front: &Frontmatter) -> bool {
    front.problem.is_some()
}

/// `does not parse: <parser's reason> (line 3, column 4)`, so the author can find it.
fn parse_problem(front: &Frontmatter) -> String {
    let where_ = front.line.map_or_else(String::new, |line| {
        let column = front
            .column
            .map_or_else(|| "undefined".to_owned(), |column| column.to_string());
        format!(" (line {line}, column {column})")
    });
    let problem = front.problem.as_deref().unwrap_or("unknown problem");
    format!("Frontmatter does not parse: {problem}{where_}")
}

/// No field beyond `name` and `description`: the frontmatter did not parse.
fn no_fields() -> AgentFieldsRead {
    AgentFieldsRead::default()
}

/// The agent Claude Code would load. `fields` is what the registry read; `unknown` the top-level
/// fields Claude Code does not recognize. A key of the official `AgentDefinition` goes into
/// `definition`; the keys it has no place for are `extras` (and `hooks`).
#[must_use]
pub fn agent_record(
    origin: RecordOrigin<'_>,
    basics: AgentBasics,
    fields: &AgentFieldsRead,
    unknown: Vec<UnrecognizedField>,
) -> AgentRecord {
    let AgentBasics {
        name,
        description,
        prompt,
    } = basics;
    let mut official = fields.values.clone();
    let color = official.shift_remove("color");
    let isolation = official.shift_remove("isolation");
    let experimental = official.shift_remove("experimental");
    let hooks = official.shift_remove("hooks");
    let (plugin, source, path) = origin_parts(origin);
    let scoped_name = plugin.map_or_else(
        || name.clone(),
        |plugin| scoped_agent_name(&plugin.name, plugin.relative_path.as_deref(), &name),
    );
    let mut definition = JsObject::new();
    definition.insert("description".to_owned(), JsValue::String(description));
    definition.insert("prompt".to_owned(), JsValue::String(prompt));
    definition.extend(official);
    let cache_ttl = experimental
        .as_ref()
        .and_then(|experimental| experimental.get("cacheTtl"))
        .filter(|ttl| !ttl.is_undefined())
        .cloned();
    AgentRecord {
        name,
        scoped_name,
        source,
        path: path.to_owned(),
        definition: JsValue::Object(definition),
        extras: AgentExtras {
            color,
            isolation,
            cache_ttl,
        },
        hooks,
        ignored: fields
            .ignored
            .iter()
            .map(|&field| field.to_owned())
            .collect(),
        unrecognized: unknown,
    }
}

/// `front.fields[key]`, `undefined` when absent.
fn field<'a>(front: &'a Frontmatter, key: &str) -> &'a JsValue {
    front.fields.get(key).unwrap_or(&JsValue::Undefined)
}

/// A project, user, managed or `--add-dir` agent file (sub-agents.md, "Subagent files Claude Code
/// skips"). No name is documentation and is skipped silently; the other problems skip the file and
/// are reported (Claude Code writes them to the debug log).
fn read_file_agent(front: &Frontmatter, origin: &AgentOrigin, path: &str) -> AgentReadResult {
    let invalid = |message: &str| skipped(SkipReason::Invalid, vec![error(path, message)]);
    if unparsed(front) {
        return invalid(&parse_problem(front));
    }
    let name = read_text("name", field(front, "name"));
    if let Some(problem) = name.problem {
        return invalid(&format!("Invalid \"name\": it {}", problem.reason));
    }
    let Some(name) = name.value else {
        return skipped(SkipReason::Documentation, vec![]);
    };
    if let Some(problem) = agent_name_problem(&name) {
        return invalid(&format!("Invalid \"name\": \"{name}\" {problem}"));
    }
    let description = read_text("description", field(front, "description"));
    if let Some(problem) = description.problem {
        return invalid(&format!("Invalid \"description\": it {}", problem.reason));
    }
    let Some(description) = description.value else {
        return invalid(&format!(
            "Agent \"{name}\" has a name but no \"description\""
        ));
    };
    let options = AgentFieldsOptions {
        plugin: false,
        path: path.to_owned(),
    };
    let fields = read_agent_fields(&front.fields, &options);
    let basics = AgentBasics {
        name,
        description,
        prompt: front.body.clone(),
    };
    let agent = agent_record(
        RecordOrigin::Agent(origin),
        basics,
        &fields,
        unrecognized_fields(&front.fields),
    );
    AgentReadResult::Loaded {
        agent: Box::new(agent),
        issues: fields.issues,
    }
}

/// A plugin agent's name: the frontmatter's when usable, else the file's (components.md).
fn plugin_name(front: &Frontmatter, origin: &PluginOrigin) -> (String, Vec<PluginIssue>) {
    let fallback = agent_file_name(&origin.path);
    let given = read_text("name", field(front, "name"));
    if let Some(problem) = given.problem {
        let message = format!(
            "Invalid \"name\": it {}; the agent is named after its file",
            problem.reason
        );
        return (fallback, vec![warning(&origin.path, &message)]);
    }
    let Some(given) = given.value else {
        return (fallback, vec![]);
    };
    if code_unit_length(&given) > MAX_AGENT_NAME {
        let message = format!(
            "\"name\" is longer than {MAX_AGENT_NAME} characters and is ignored; the agent is named after its file"
        );
        return (fallback, vec![warning(&origin.path, &message)]);
    }
    (given, vec![])
}

/// A plugin agent file (plugins/components.md, "Agents" and "Frontmatter fields in plugin
/// agents"): it always loads. A missing, too long or unusable name falls back to the file name;
/// frontmatter that does not parse loads with every field ignored and the description
/// `Agent from <plugin> plugin`.
fn read_plugin_agent(
    front: &Frontmatter,
    origin: &AgentOrigin,
    plugin: &PluginOrigin,
) -> AgentReadResult {
    let fallback_description = format!("Agent from {} plugin", plugin.plugin.name);
    if unparsed(front) {
        let message = format!(
            "{}; the agent loads with every field ignored",
            parse_problem(front)
        );
        let name = agent_file_name(&plugin.path);
        let basics = AgentBasics {
            name,
            description: fallback_description,
            prompt: front.body.clone(),
        };
        let agent = agent_record(RecordOrigin::Agent(origin), basics, &no_fields(), vec![]);
        return AgentReadResult::Loaded {
            agent: Box::new(agent),
            issues: vec![warning(&plugin.path, &message)],
        };
    }
    let (name, mut issues) = plugin_name(front, plugin);
    let description = read_text("description", field(front, "description"));
    if description.value.is_none() {
        // Judgment (spec/judgments.md): the docs give the fallback description for unparseable
        // frontmatter only; the same text stands in for a missing or unusable one.
        let reason = description.problem.as_ref().map_or_else(
            || "it is missing".to_owned(),
            |problem| format!("it {}", problem.reason),
        );
        let message =
            format!("Invalid \"description\": {reason}; using \"{fallback_description}\"");
        issues.push(warning(&plugin.path, &message));
    }
    let options = AgentFieldsOptions {
        plugin: true,
        path: plugin.path.clone(),
    };
    let fields = read_agent_fields(&front.fields, &options);
    let basics = AgentBasics {
        name,
        description: description.value.unwrap_or(fallback_description),
        prompt: front.body.clone(),
    };
    let agent = agent_record(
        RecordOrigin::Agent(origin),
        basics,
        &fields,
        unrecognized_fields(&front.fields),
    );
    issues.extend(fields.issues);
    AgentReadResult::Loaded {
        agent: Box::new(agent),
        issues,
    }
}

/// Read one agent file (sub-agents.md, "Frontmatter reference"): the Markdown body is the system
/// prompt, `name` and `description` are required. Never throws; a file Claude Code would skip is
/// `skipped`, with the reason it names.
#[must_use]
pub fn read_agent(text: &str, origin: &AgentOrigin) -> AgentReadResult {
    let front = parse_frontmatter(text);
    match origin {
        AgentOrigin::File(file) => read_file_agent(&front, origin, &file.path),
        AgentOrigin::Plugin(plugin) => read_plugin_agent(&front, origin, plugin),
    }
}

crate::registry::exports! {
    "format/agent" {
        "readAgent" => ok |text: String, origin: AgentOrigin| read_agent(&text, &origin);
    }
}
