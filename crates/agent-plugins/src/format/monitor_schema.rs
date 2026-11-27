//! Ported from `src/format/monitor-schema.ts`; only the core uses it now, so it has no wrapper.
//!
//! A monitor entry and the monitors source as Claude Code's schema check reads them, one rule set
//! for the loader and the validator (plugins/manifest-reference.md, "monitors": "Each entry is a
//! strict object"; spec §9.2.1–§9.2.2). The messages are the oracle's (2.1.325): zod's wording for a
//! missing or empty field, and for `when` and the names the texts the spec quotes. A missing field
//! aborts the entry, so inside the source union it is a bare `Invalid input` at the key.

use std::sync::LazyLock;

use crate::format::strict_keys::closed_object;
use crate::js::JsValue;
use crate::zod::{self, Schema, fields};

/// `"always"`: a monitor that starts with the session.
pub const ALWAYS: &str = "always";
const ON_SKILL_INVOKE: &str = "on-skill-invoke:";

static WHEN_PREFIX: LazyLock<String> =
    LazyLock::new(|| format!("Invalid string: must start with \"{ON_SKILL_INVOKE}\""));
/// A `when` of `on-skill-invoke:` with no skill.
pub const WHEN_SKILL: &str = "on-skill-invoke: must specify a skill name";
/// Two entries of one plugin share a name.
pub const UNIQUE_NAMES: &str = "Monitor names must be unique within a plugin";

static TEXT: LazyLock<Schema> = LazyLock::new(|| zod::string().min(1.0));

/// The skill after the prefix, empty when there is none or no prefix (spec J9.8: read as written).
#[must_use]
pub fn skill_of(when: &str) -> String {
    when.strip_prefix(ON_SKILL_INVOKE)
        .unwrap_or_default()
        .to_owned()
}

/// The text of a value the string schema before it already passed.
fn text_of(value: &JsValue) -> &str {
    value.as_str().unwrap_or_default()
}

/// `"always"` or `"on-skill-invoke:<skill>"` with a non-empty skill. A value that is neither breaks
/// both rules and is reported twice, as the oracle does.
static WHEN: LazyLock<Schema> = LazyLock::new(|| {
    zod::string()
        .refine(
            |value| text_of(value) == ALWAYS || text_of(value).starts_with(ON_SKILL_INVOKE),
            WHEN_PREFIX.as_str(),
        )
        .refine(
            |value| text_of(value) == ALWAYS || !skill_of(text_of(value)).is_empty(),
            WHEN_SKILL,
        )
});

/// `monitorEntrySchema`.
pub static MONITOR_ENTRY_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    closed_object(fields! {
        "name" => TEXT.clone(),
        "command" => TEXT.clone(),
        "description" => TEXT.clone(),
        "when" => WHEN.exact_optional(),
    })
});

/// `new Set(entries.map((entry) => entry.name)).size === entries.length`.
fn names_unique(entries: &JsValue) -> bool {
    let list = entries.as_array().map_or(&[][..], Vec::as_slice);
    let mut names: Vec<&JsValue> = Vec::new();
    for entry in list {
        let name = entry.get("name").unwrap_or(&JsValue::Undefined);
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names.len() == list.len()
}

/// Two entries of one plugin never share a name; reported once, at the array.
static MONITOR_LIST_SCHEMA: LazyLock<Schema> =
    LazyLock::new(|| zod::array(MONITOR_ENTRY_SCHEMA.clone()).refine(names_unique, UNIQUE_NAMES));

/// `experimental.monitors` (and the top-level `monitors` that still loads): a path or the inline
/// array. A path is any string here; its `./` and `.json` rules are the path checks'.
pub static MONITOR_SOURCES_SCHEMA: LazyLock<Schema> =
    LazyLock::new(|| zod::union([zod::string(), MONITOR_LIST_SCHEMA.clone()]));
