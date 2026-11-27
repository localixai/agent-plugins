//! Port of `src/format/validate/plugin-paths.ts`, switched.
//!
//! The shape rules of the manifest keys that name component paths (oracle 2.1.319): every path
//! starts with `./`; a key that names one kind of file also fixes its extension. A value of the
//! wrong shape is a bare "Invalid input" at the key; a path that breaks a rule is reported once per
//! rule, at the key (a single string) or at the entry (`skills.0`).

use serde::Deserialize;

use super::finding::{Finding, error_at};
use crate::js::JsValue;
use crate::js::value::JsObject;

/// What a rule says about one path: the messages of the rules it breaks, none when it keeps them.
type StringRule = fn(&str) -> Vec<String>;

const INVALID: &str = "Invalid input";
const START: &str = "Invalid string: must start with \"./\"";

fn starts_with_dot(path: &str) -> Vec<String> {
    if path.starts_with("./") {
        Vec::new()
    } else {
        vec![START.to_owned()]
    }
}

/// `skills` "also accepts `"."`. Both `"."` and `"./"` denote the plugin root" (manifest-reference.md).
fn skills_path(path: &str) -> Vec<String> {
    if path == "." {
        Vec::new()
    } else {
        starts_with_dot(path)
    }
}

fn ends_with(suffix: &str, path: &str) -> Vec<String> {
    if path.ends_with(suffix) {
        Vec::new()
    } else {
        vec![format!("Invalid string: must end with \"{suffix}\"")]
    }
}

/// `/^https:\/\//i`: JavaScript's `i` without `u` folds ASCII only.
fn is_remote(path: &str) -> bool {
    path.get(.."https://".len())
        .is_some_and(|start| start.eq_ignore_ascii_case("https://"))
}

/// `/\.(?:mcpb|dxt)$/`.
fn is_bundle(path: &str) -> bool {
    path.ends_with(".mcpb") || path.ends_with(".dxt")
}

fn declaration_file(path: &str) -> Vec<String> {
    if path.ends_with(".d.ts") {
        Vec::new()
    } else {
        vec!["types must name a TypeScript declaration file ending in '.d.ts'".to_owned()]
    }
}

/// An MCP source: a `.json` file, a `.mcpb` or `.dxt` bundle, or an `https://` bundle URL.
fn mcp_source(path: &str) -> Vec<String> {
    if is_remote(path) {
        return Vec::new();
    }
    let mut found = starts_with_dot(path);
    if !is_bundle(path) {
        found.extend(ends_with(".json", path));
    }
    found
}

fn both(rules: &[StringRule], path: &str) -> Vec<String> {
    rules.iter().flat_map(|rule| rule(path)).collect()
}

const fn is_text(value: &JsValue) -> bool {
    value.is_string()
}

/// Whether an optional field is absent or satisfies `test`.
fn optional(entry: &JsObject, key: &str, test: impl Fn(&JsValue) -> bool) -> bool {
    entry
        .get(key)
        .is_none_or(|value| value.is_undefined() || test(value))
}

/// An inline command: text, a free `source` path or `content`, and a few typed options.
fn command_well_typed(entry: &JsValue) -> bool {
    entry.as_object().is_some_and(|entry| {
        optional(entry, "source", |value| {
            value
                .as_str()
                .is_some_and(|path| starts_with_dot(path).is_empty())
        }) && optional(entry, "content", is_text)
            && optional(entry, "description", is_text)
            && optional(entry, "argumentHint", is_text)
            && optional(entry, "model", is_text)
            && optional(entry, "allowedTools", |value| {
                value
                    .as_array()
                    .is_some_and(|items| items.iter().all(is_text))
            })
    })
}

const COMMAND_CHOICE: &str = "Command must have either \"source\" (file path) or \"content\" (inline markdown), but not both";

/// The map form of `commands`: well-typed entries, each with exactly one of `source` and `content`.
fn command_map_findings(path: &str, map: &JsObject) -> Vec<Finding> {
    if !map.values().all(command_well_typed) {
        return vec![error_at(path, INVALID)];
    }
    map.iter()
        .filter_map(|(name, entry)| {
            let entry = entry.as_object()?;
            let source = entry.get("source").is_none_or(JsValue::is_undefined);
            let content = entry.get("content").is_none_or(JsValue::is_undefined);
            (source == content).then(|| error_at(&format!("{path}.{name}"), COMMAND_CHOICE))
        })
        .collect()
}

/// Checks of the object form, when it has more to say than its being an object.
type MapRule = fn(&str, &JsObject) -> Vec<Finding>;

/// How one path-valued key is read.
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the four flags are the TypeScript's `PathShape`, copied member for member"
)]
pub struct PathShape {
    rule: StringRule,
    /// The value may be an object (an inline declaration).
    inline: bool,
    /// The value may be an array of paths (and, when `inline` and not `strings`, of declarations).
    list: bool,
    /// An array holds paths only: `commands` takes a map of entries, never a list of them.
    strings: bool,
    /// Any problem is reported as one "Invalid input" at the key, as for `commands`.
    opaque: bool,
    /// Checks of the object form, when it has more to say than its being an object. `monitors` takes
    /// an inline array of entries (manifest-reference.md, "`experimental.monitors`"), never one object.
    map: Option<MapRule>,
}

/// A key that names component paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PathKey {
    /// `skills`.
    Skills,
    /// `commands`.
    Commands,
    /// `agents`.
    Agents,
    /// `hooks`.
    Hooks,
    /// `mcpServers`.
    McpServers,
    /// `lspServers`.
    LspServers,
    /// `outputStyles`.
    OutputStyles,
    /// `workflows`.
    Workflows,
    /// `themes`.
    Themes,
    /// `monitors`.
    Monitors,
    /// `types`.
    Types,
}

/// The shape of each path-valued key.
pub struct PathShapes {
    skills: PathShape,
    commands: PathShape,
    agents: PathShape,
    hooks: PathShape,
    mcp_servers: PathShape,
    lsp_servers: PathShape,
    output_styles: PathShape,
    workflows: PathShape,
    themes: PathShape,
    monitors: PathShape,
    types: PathShape,
}

impl PathShapes {
    /// `PATH_SHAPES[key]`.
    #[must_use]
    pub const fn of(&self, key: PathKey) -> &PathShape {
        match key {
            PathKey::Skills => &self.skills,
            PathKey::Commands => &self.commands,
            PathKey::Agents => &self.agents,
            PathKey::Hooks => &self.hooks,
            PathKey::McpServers => &self.mcp_servers,
            PathKey::LspServers => &self.lsp_servers,
            PathKey::OutputStyles => &self.output_styles,
            PathKey::Workflows => &self.workflows,
            PathKey::Themes => &self.themes,
            PathKey::Monitors => &self.monitors,
            PathKey::Types => &self.types,
        }
    }
}

/// The shapes. A table of Rust functions: it never crosses the boundary, so the wrapper has none and
/// `pathShapeFindings` takes the key.
pub static PATH_SHAPES: PathShapes = PathShapes {
    skills: PathShape {
        rule: skills_path,
        inline: false,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    commands: PathShape {
        rule: starts_with_dot,
        inline: true,
        list: true,
        strings: true,
        opaque: true,
        map: Some(command_map_findings),
    },
    agents: PathShape {
        rule: |path| both(&[starts_with_dot, |path| ends_with(".md", path)], path),
        inline: false,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    hooks: PathShape {
        rule: |path| both(&[starts_with_dot, |path| ends_with(".json", path)], path),
        inline: true,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    mcp_servers: PathShape {
        rule: mcp_source,
        inline: true,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    lsp_servers: PathShape {
        rule: |path| both(&[starts_with_dot, |path| ends_with(".json", path)], path),
        inline: true,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    output_styles: PathShape {
        rule: starts_with_dot,
        inline: false,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    workflows: PathShape {
        rule: starts_with_dot,
        inline: false,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    themes: PathShape {
        rule: starts_with_dot,
        inline: false,
        list: true,
        strings: false,
        opaque: false,
        map: None,
    },
    monitors: PathShape {
        rule: |path| both(&[starts_with_dot, |path| ends_with(".json", path)], path),
        inline: true,
        list: true,
        strings: false,
        opaque: false,
        map: Some(|path, _| vec![error_at(path, INVALID)]),
    },
    types: PathShape {
        rule: |path| both(&[starts_with_dot, declaration_file], path),
        inline: false,
        list: false,
        strings: false,
        opaque: false,
        map: None,
    },
};

/// Whether `key` names one of `PATH_SHAPES` (`Object.hasOwn`).
#[must_use]
pub fn is_path_key(key: &str) -> bool {
    PathKey::deserialize(JsValue::from(key)).is_ok()
}

fn allowed(shape: &PathShape, value: &JsValue) -> bool {
    value.is_string() || (shape.inline && !shape.strings && value.is_object())
}

fn entry_findings(shape: &PathShape, value: &str, at: &str) -> Vec<Finding> {
    (shape.rule)(value)
        .iter()
        .map(|message| error_at(at, message))
        .collect()
}

/// An opaque key reports its first problem as a bare "Invalid input".
fn collapse(path: &str, shape: &PathShape, found: Vec<Finding>) -> Vec<Finding> {
    if shape.opaque && !found.is_empty() {
        vec![error_at(path, INVALID)]
    } else {
        found
    }
}

/// The shape findings of one path-valued key; `path` is what the oracle prints for it.
#[must_use]
pub fn path_shape_findings(path: &str, shape: &PathShape, value: &JsValue) -> Vec<Finding> {
    if let Some(text) = value.as_str() {
        return collapse(path, shape, entry_findings(shape, text, path));
    }
    if shape.inline
        && let Some(members) = value.as_object()
    {
        return shape.map.map_or_else(Vec::new, |map| map(path, members));
    }
    let Some(items) = value.as_array().filter(|_| shape.list) else {
        return vec![error_at(path, INVALID)];
    };
    if !items.iter().all(|entry| allowed(shape, entry)) {
        return vec![error_at(path, INVALID)];
    }
    let found = items
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| match entry.as_str() {
            Some(text) => entry_findings(shape, text, &format!("{path}.{index}")),
            None => Vec::new(),
        })
        .collect();
    collapse(path, shape, found)
}

crate::registry::exports! {
    "format/validate/plugin-paths" {
        "isPathKey" => ok |key: String| is_path_key(&key);
    }
}
