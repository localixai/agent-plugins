//! Ported from `src/format/manifest-schema.ts`; only the core uses it now, so it has no wrapper.
//!
//! The top-level keys of `.claude-plugin/plugin.json` and their documented types
//! (plugins/manifest-reference.md, "Fields"). Strict where the docs say so ("Unrecognized fields"):
//! `userConfig` options, `channels` entries, inline LSP configurations and monitor entries. Inline
//! hook and MCP configs are checked by their own readers, not here.

use std::sync::LazyLock;

use serde::Deserialize;

use crate::format::lsp_schema::LSP_SOURCES_SCHEMA;
use crate::format::monitor_schema::MONITOR_SOURCES_SCHEMA;
use crate::format::user_config_options::{OPTION_VALUE, OptionsHolder, options_problems};
use crate::format::user_config_refs::USER_CONFIG_KEY_SOURCE as USER_CONFIG_KEY;
use crate::js::{JsValue, url};
use crate::zod::{self, Schema, Shape, fields, issue};

static PATH: LazyLock<Schema> = LazyLock::new(zod::string);
static PATH_OR_PATHS: LazyLock<Schema> =
    LazyLock::new(|| zod::union([PATH.clone(), zod::array(PATH.clone())]));
static OBJECT: LazyLock<Schema> = LazyLock::new(|| zod::record(zod::string(), zod::unknown()));
static FILE_OR_INLINE: LazyLock<Schema> = LazyLock::new(|| {
    zod::union([
        PATH.clone(),
        OBJECT.clone(),
        zod::array(zod::union([PATH.clone(), OBJECT.clone()])),
    ])
});

/// The inline LSP map and monitors array are strict objects of the manifest (spec §2.1 item 4,
/// §9.1.1 item 3, §9.2.1): an unknown key or a bad value in one keeps the plugin from loading.
fn monitors() -> Schema {
    MONITOR_SOURCES_SCHEMA.clone()
}

const OPTION_TYPES: [&str; 5] = ["string", "number", "boolean", "directory", "file"];

static USER_CONFIG_FIELDS: LazyLock<Schema> = LazyLock::new(|| {
    zod::strict_object(fields! {
        "type" => zod::r#enum(OPTION_TYPES),
        "title" => zod::string(),
        "description" => zod::string(),
        "required" => zod::boolean().optional(),
        "default" => zod::union([
            zod::string(),
            zod::number(),
            zod::boolean(),
            zod::array(zod::string()),
        ])
        .optional(),
        "options" => zod::array(OPTION_VALUE.clone()).optional(),
        "multiple" => zod::boolean().optional(),
        "sensitive" => zod::boolean().optional(),
        "min" => zod::number().optional(),
        "max" => zod::number().optional(),
    })
});

/// An option with the `options` rules, each problem at its own path (`options`, `default`).
pub static USER_CONFIG_OPTION_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    USER_CONFIG_FIELDS.super_refine(|option, context| {
        // The refinement runs on an option that passed its fields, so it reads as one.
        let holder = OptionsHolder::deserialize(option.clone()).unwrap_or_default();
        for problem in options_problems(&holder) {
            context.add_issue(
                issue("custom")
                    .path([problem.path.key()])
                    .message(problem.message),
            );
        }
    })
});

static USER_CONFIG: LazyLock<Schema> = LazyLock::new(|| {
    zod::record(
        zod::string().regex(USER_CONFIG_KEY),
        USER_CONFIG_OPTION_SCHEMA.clone(),
    )
});

/// The manifest says an entry that sets both or neither "fails validation".
pub const COMMAND_ENTRY_ONE_OF: &str = "Set exactly one of \"source\" or \"content\"";

/// Whether a member of an object is `undefined` (absent or set to it).
fn is_unset(entry: &JsValue, key: &str) -> bool {
    entry.get(key).is_none_or(JsValue::is_undefined)
}

static COMMAND_ENTRY_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "source" => PATH.optional(),
        "content" => zod::string().optional(),
        "description" => zod::string().optional(),
        "argumentHint" => zod::string().optional(),
        "model" => zod::string().optional(),
        "allowedTools" => zod::array(zod::string()).optional(),
    })
    .refine(
        |entry| is_unset(entry, "source") != is_unset(entry, "content"),
        COMMAND_ENTRY_ONE_OF,
    )
});

/// A dependency string has at most one `@`: `"a@b@c"` fails the manifest as `Invalid input`
/// (spec J16.17, oracle 2.1.325). Shared with the validator's schema.
pub static DEPENDENCY_TEXT: LazyLock<Schema> = LazyLock::new(|| {
    zod::string().refine(
        |text| {
            let text = text.as_str().unwrap_or_default();
            text.find('@') == text.rfind('@')
        },
        (),
    )
});

static DEPENDENCY: LazyLock<Schema> = LazyLock::new(|| {
    zod::union([
        DEPENDENCY_TEXT.min(1.0),
        zod::object(fields! {
            "name" => zod::string().min(1.0),
            "marketplace" => zod::string().optional(),
            "version" => zod::string().optional(),
        }),
    ])
});

/// A `channels` entry: a channel bound to one of the plugin's MCP servers.
static CHANNEL: LazyLock<Schema> = LazyLock::new(|| {
    zod::strict_object(fields! {
        "server" => zod::string(),
        "displayName" => zod::string().optional(),
        "userConfig" => USER_CONFIG.optional(),
    })
});

static URL: LazyLock<Schema> = LazyLock::new(|| {
    zod::string().refine(
        |value| value.as_str().and_then(url::parse).is_some(),
        "Must parse as a URL",
    )
});

/// `MANIFEST_SCHEMAS`: each documented key's schema, in the order the TypeScript writes them.
pub static MANIFEST_SCHEMAS: LazyLock<Shape> = LazyLock::new(|| {
    fields! {
        "$schema" => zod::string(),
        "name" => zod::string(),
        "displayName" => zod::string(),
        "version" => zod::string(),
        "description" => zod::string(),
        "author" => zod::object(fields! {
            "name" => zod::string(),
            "email" => zod::string().optional(),
            "url" => zod::string().optional(),
        }),
        "homepage" => URL.clone(),
        "repository" => zod::string(),
        "license" => zod::string(),
        "keywords" => zod::array(zod::string()),
        "metadata" => OBJECT.clone(),
        "icon" => zod::string(),
        "documentationUrl" => zod::string(),
        "supportUrl" => zod::string(),
        "privacyPolicyUrl" => zod::string(),
        "termsOfServiceUrl" => zod::string(),
        "defaultEnabled" => zod::boolean(),
        "dependencies" => zod::array(DEPENDENCY.clone()),
        "settings" => OBJECT.clone(),
        "userConfig" => USER_CONFIG.clone(),
        "types" => PATH.clone(),
        "channels" => zod::array(CHANNEL.clone()),
        "skills" => PATH_OR_PATHS.clone(),
        "commands" => zod::union([
            PATH_OR_PATHS.clone(),
            zod::record(zod::string(), COMMAND_ENTRY_SCHEMA.clone()),
        ]),
        "agents" => PATH_OR_PATHS.clone(),
        "hooks" => FILE_OR_INLINE.clone(),
        "mcpServers" => FILE_OR_INLINE.clone(),
        "lspServers" => LSP_SOURCES_SCHEMA.clone(),
        "outputStyles" => PATH_OR_PATHS.clone(),
        "workflows" => PATH_OR_PATHS.clone(),
        "experimental" => zod::object(fields! {
            "themes" => PATH_OR_PATHS.optional(),
            "monitors" => monitors().optional(),
            "evals" => PATH_OR_PATHS.optional(),
        }),
        // Moved under `experimental`; still loaded at the top level, with a validator warning.
        "themes" => PATH_OR_PATHS.clone(),
        "monitors" => monitors(),
    }
});

/// The whole manifest: `name` required, every other documented key optional.
pub static MANIFEST_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(MANIFEST_SCHEMAS.clone())
        .partial()
        .required(&["name"])
});

/// Whether `key` is a documented top-level key (an own key of `MANIFEST_SCHEMAS`).
#[must_use]
pub fn is_manifest_key(key: &str) -> bool {
    MANIFEST_SCHEMAS.contains_key(key)
}
