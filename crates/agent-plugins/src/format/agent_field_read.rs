//! Ported from `src/format/agent-field-read.ts`; only the core uses it now, so it has no wrapper.
//!
//! Every field of the agent registry beyond `name` and `description`, read from a frontmatter
//! table: `readAgent` reads those two itself, as their problems decide if the file loads.

use serde::{Deserialize, Serialize};

use super::agent_fields::{AGENT_FIELD_NAMES, plugin_use, read_agent_field};
use super::issues::PluginIssue;
use crate::js::JsObject;

/// The fields `readAgent` reads itself.
const OWN_FIELD_NAMES: [&str; 2] = ["name", "description"];

/// What the registry read from a file's frontmatter.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct AgentFieldsRead {
    /// The fields that came out usable, by name.
    pub values: JsObject,
    /// Fields in the file that Claude Code does not apply for a plugin agent (components.md).
    pub ignored: Vec<&'static str>,
    /// What was wrong.
    pub issues: Vec<PluginIssue>,
}

/// How the fields are read.
#[derive(Clone, Debug, Deserialize)]
pub struct AgentFieldsOptions {
    /// A plugin's agent, which skips the fields a plugin agent ignores.
    pub plugin: bool,
    /// The agent file, for the issues.
    pub path: String,
}

/// Reads every field of the registry beyond `name` and `description` from the frontmatter table.
/// A plugin agent skips the fields it ignores (and names them in `ignored` when the file has them).
#[must_use]
pub fn read_agent_fields(fields: &JsObject, options: &AgentFieldsOptions) -> AgentFieldsRead {
    let mut read = AgentFieldsRead::default();
    for name in AGENT_FIELD_NAMES {
        if OWN_FIELD_NAMES.contains(&name) {
            continue;
        }
        let Some(raw) = fields.get(name) else {
            continue;
        };
        if options.plugin && plugin_use(name) == "ignored" {
            read.ignored.push(name);
            continue;
        }
        let field = read_agent_field(name, &options.path, name, raw);
        if let Some(value) = field.value.filter(|value| !value.is_undefined()) {
            read.values.insert(name.to_owned(), value);
        }
        read.issues.extend(field.issues);
    }
    read
}
