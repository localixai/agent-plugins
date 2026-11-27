//! Port of `src/format/agent-fields.ts`, switched.
//!
//! The 19 frontmatter fields of a subagent file (sub-agents.md, "Frontmatter reference"), in the
//! table's order, each with whether a plugin agent applies it and the reader of its raw YAML value.
//! Claude Code "ignores a field it doesn't recognize", so anything else is `unrecognized`.

use indexmap::IndexMap;
use serde::Serialize;

use super::agent_hooks_field::hooks;
use super::agent_scalars::{
    AgentFieldRead, auto_compact_window, bool_field, effort_field, enum_field, experimental_map,
    model_field, positive_int, text_field, token_list,
};
use super::issues::warning;
use crate::js::value::to_js_value;
use crate::js::{JsValue, trim};
use crate::protocol::is_record::is_record;
use crate::protocol::permission_modes::normalize_permission_mode;
use crate::registry::Function;

/// The fields, in the table's order.
pub const AGENT_FIELD_NAMES: [&str; 19] = [
    "name",
    "description",
    "tools",
    "disallowedTools",
    "model",
    "permissionMode",
    "maxTurns",
    "autoCompactWindow",
    "skills",
    "mcpServers",
    "hooks",
    "memory",
    "background",
    "omitClaudeMd",
    "effort",
    "isolation",
    "color",
    "initialPrompt",
    "experimental",
];

/// `color`: "Accepts `red`, `blue`, `green`, `yellow`, `purple`, `orange`, `pink`, or `cyan`".
pub const AGENT_COLORS: [&str; 8] = [
    "red", "blue", "green", "yellow", "purple", "orange", "pink", "cyan",
];

/// `isolation`: "Set to `worktree`"; plugins/components.md: "The only valid `isolation` value".
pub const AGENT_ISOLATIONS: [&str; 1] = ["worktree"];

/// `memory`: "`user`, `project`, or `local`", tied to the official `AgentDefinition`.
pub const AGENT_MEMORY_SCOPES: [&str; 3] = ["user", "project", "local"];

/// plugins/components.md, "Frontmatter fields in plugin agents": the fields a plugin agent ignores.
const IGNORED_IN_PLUGINS: [&str; 4] = ["permissionMode", "mcpServers", "hooks", "initialPrompt"];

/// Whether a plugin agent applies the field (`supported`) or ignores it (`ignored`).
#[must_use]
pub fn plugin_use(field: &str) -> &'static str {
    if IGNORED_IN_PLUGINS.contains(&field) {
        "ignored"
    } else {
        "supported"
    }
}

/// `permissionMode`: a mode, or `manual` for `default` (the one alias the docs name).
fn permission_mode(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<String> {
    let text = text_field(path, key, raw);
    let Some(written) = text.value else {
        return text;
    };
    if let Some(mode) = normalize_permission_mode(&written) {
        return AgentFieldRead {
            value: Some(mode.to_owned()),
            issues: Vec::new(),
        };
    }
    let message =
        format!("Invalid \"{key}\": \"{written}\" is not a permission mode; the field is ignored");
    AgentFieldRead {
        value: None,
        issues: vec![warning(path, &message)],
    }
}

/// An inline definition: a table with at least one key whose every value is a table (the server's
/// config). The config itself is the MCP reader's to validate, so only its outer shape is checked
/// here (judgment, spec/judgments.md).
fn is_inline_server(value: &JsValue) -> bool {
    value.as_object().is_some_and(|members| {
        is_record(value) && !members.is_empty() && members.values().all(is_record)
    })
}

/// `mcpServers`: "a server name referencing an already-configured server ... or an inline
/// definition with the server name as key and a full MCP server config as value". An entry of
/// another shape is dropped.
fn mcp_servers(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<Vec<JsValue>> {
    let items = match raw {
        JsValue::Undefined | JsValue::Null => {
            return AgentFieldRead {
                value: None,
                issues: Vec::new(),
            };
        }
        JsValue::Array(items) => items,
        JsValue::Bool(_) | JsValue::Number(_) | JsValue::String(_) | JsValue::Object(_) => {
            let message = format!(
                "Invalid \"{key}\": it is not a list of server names and inline definitions; the field is ignored"
            );
            return AgentFieldRead {
                value: None,
                issues: vec![warning(path, &message)],
            };
        }
    };
    let entries: Vec<JsValue> = items
        .iter()
        .filter_map(|entry| match entry.as_str() {
            Some(name) => Some(trim(name))
                .filter(|name| !name.is_empty())
                .map(JsValue::from),
            None => is_inline_server(entry).then(|| entry.clone()),
        })
        .collect();
    let issues = if entries.len() == items.len() {
        Vec::new()
    } else {
        let message = format!(
            "Invalid \"{key}\": some entries are neither a name nor a definition; they are ignored"
        );
        vec![warning(path, &message)]
    };
    AgentFieldRead {
        value: Some(entries),
        issues,
    }
}

/// A field's read with its value as the host holds it.
fn erased<T: Serialize>(read: AgentFieldRead<T>) -> AgentFieldRead<JsValue> {
    AgentFieldRead {
        value: read.value.and_then(|value| to_js_value(&value).ok()),
        issues: read.issues,
    }
}

/// Reads one field's raw YAML value (`AGENT_FIELDS[field].read`); `path` is the agent file, for the
/// issues. An unknown field reads to nothing.
#[must_use]
pub fn read_agent_field(
    field: &str,
    path: &str,
    key: &str,
    raw: &JsValue,
) -> AgentFieldRead<JsValue> {
    match field {
        "name" | "description" | "initialPrompt" => erased(text_field(path, key, raw)),
        "tools" | "disallowedTools" | "skills" => erased(token_list(path, key, raw)),
        "model" => erased(model_field(path, key, raw)),
        "permissionMode" => erased(permission_mode(path, key, raw)),
        "maxTurns" => erased(positive_int(path, key, raw)),
        "autoCompactWindow" => erased(auto_compact_window(path, key, raw)),
        "mcpServers" => erased(mcp_servers(path, key, raw)),
        "hooks" => erased(hooks(path, key, raw)),
        "memory" => erased(enum_field(&AGENT_MEMORY_SCOPES)(path, key, raw)),
        "background" | "omitClaudeMd" => erased(bool_field(path, key, raw)),
        "effort" => erased(effort_field(path, key, raw)),
        "isolation" => erased(enum_field(&AGENT_ISOLATIONS)(path, key, raw)),
        "color" => erased(enum_field(&AGENT_COLORS)(path, key, raw)),
        "experimental" => erased(experimental_map(path, key, raw)),
        _ => AgentFieldRead {
            value: None,
            issues: Vec::new(),
        },
    }
}

/// The fields a plugin agent ignores (plugins/components.md), in the table's order.
#[must_use]
pub fn plugin_ignored_fields() -> Vec<&'static str> {
    AGENT_FIELD_NAMES
        .into_iter()
        .filter(|field| plugin_use(field) == "ignored")
        .collect()
}

/// The `read` of one field's spec as the host holds it: `(path, key, raw) => AgentFieldRead`.
struct FieldReader(&'static str);

crate::registry::methods! {
    FieldReader {
        "call" => ok |this, path: String, key: String, raw: JsValue|
            read_agent_field(this.0, &path, &key, &raw);
    }
}

/// One field's spec as the host holds it (`AgentFieldSpec`).
#[derive(Debug, Serialize)]
struct FieldSpec {
    plugin: &'static str,
    read: Function,
}

/// `AGENT_FIELDS` as the host holds it: one spec per field name.
fn agent_fields() -> IndexMap<&'static str, FieldSpec> {
    AGENT_FIELD_NAMES
        .into_iter()
        .map(|field| {
            let spec = FieldSpec {
                plugin: plugin_use(field),
                read: Function::new(FieldReader(field)),
            };
            (field, spec)
        })
        .collect()
}

crate::registry::exports! {
    "format/agent-fields" {
        "AGENT_FIELD_NAMES" => ok || AGENT_FIELD_NAMES;
        "AGENT_COLORS" => ok || AGENT_COLORS;
        "AGENT_ISOLATIONS" => ok || AGENT_ISOLATIONS;
        "AGENT_MEMORY_SCOPES" => ok || AGENT_MEMORY_SCOPES;
        "AGENT_FIELDS" => ok agent_fields;
        "PLUGIN_IGNORED_FIELDS" => ok plugin_ignored_fields;
    }
}
