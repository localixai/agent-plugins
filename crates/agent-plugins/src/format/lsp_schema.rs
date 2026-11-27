//! Port of `src/format/lsp-schema.ts` at sha256:4bd4202a60eb.
//!
//! The LSP server configuration as Claude Code's schema check reads it, one rule set for the loader
//! and the validator (plugins/manifest-reference.md, "lspServers": "Each server config is a strict
//! object"; spec §9.1.2, the oracle table of §9.1.3). The messages are the oracle's (2.1.325): zod's
//! own wording, and for `command` and `extensionToLanguage` the two texts it adds. A type error, a
//! fraction or a key without its dot aborts the entry, so inside the `lspServers` union it is a bare
//! `Invalid input` at the key; a too-small number, an empty map, a command with spaces and an
//! unknown key stay at their own path.

use std::sync::LazyLock;

use crate::format::strict_keys::closed_object;
use crate::js::is_whitespace;
use crate::zod::{self, Schema, fields};

/// `stdio` (default) or `socket`: "Claude Code accepts `socket` but runs every server over stdio".
pub const LSP_TRANSPORTS: [&str; 2] = ["stdio", "socket"];

const COMMAND_SPACES: &str = "Command should not contain spaces. Use args array for arguments.";
const NO_MAPPING: &str = "extensionToLanguage must have at least one mapping";

static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
static STRINGS: LazyLock<Schema> = LazyLock::new(|| zod::record(TEXT.clone(), TEXT.clone()));
static POSITIVE: LazyLock<Schema> = LazyLock::new(|| zod::number().int().positive());
static FLAG: LazyLock<Schema> = LazyLock::new(zod::boolean);

/// Judgment (spec §9.1.2): any whitespace counts as a space; a path that starts with `/` may hold them.
static COMMAND: LazyLock<Schema> = LazyLock::new(|| {
    TEXT.min(1.0).refine(
        |value| {
            let value = value.as_str().unwrap_or_default();
            value.starts_with('/') || !value.chars().any(is_whitespace)
        },
        COMMAND_SPACES,
    )
});

/// Every key starts with a dot (`.go`), and there is at least one.
static EXTENSIONS: LazyLock<Schema> = LazyLock::new(|| {
    zod::record(TEXT.regex(r"^\."), TEXT.clone()).refine(
        |map| map.as_object().is_some_and(|members| !members.is_empty()),
        NO_MAPPING,
    )
});

/// `lspServerSchema`.
pub static LSP_SERVER_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    closed_object(fields! {
        "command" => COMMAND.clone(),
        "extensionToLanguage" => EXTENSIONS.clone(),
        "args" => zod::array(TEXT.clone()).exact_optional(),
        "transport" => zod::r#enum(LSP_TRANSPORTS).exact_optional(),
        "env" => STRINGS.exact_optional(),
        "initializationOptions" => zod::unknown().exact_optional(),
        "settings" => zod::unknown().exact_optional(),
        "workspaceFolder" => TEXT.exact_optional(),
        "startupTimeout" => POSITIVE.exact_optional(),
        "shutdownTimeout" => POSITIVE.exact_optional(),
        "requestTimeout" => POSITIVE.exact_optional(),
        "restartOnCrash" => FLAG.exact_optional(),
        "maxRestarts" => zod::number().int().min(0.0).exact_optional(),
        "diagnostics" => FLAG.exact_optional(),
    })
});

/// A map of server name to configuration: `.lsp.json`, a named file, or the inline object.
static LSP_MAP_SCHEMA: LazyLock<Schema> =
    LazyLock::new(|| zod::record(TEXT.clone(), LSP_SERVER_SCHEMA.clone()));

/// The manifest's `lspServers`: a path, an inline map, or an array of either. A path is any string
/// here; its `./` and `.json` rules are the path checks' (validate/plugin-paths.ts).
pub static LSP_SOURCES_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::union([
        TEXT.clone(),
        LSP_MAP_SCHEMA.clone(),
        zod::array(zod::union([TEXT.clone(), LSP_MAP_SCHEMA.clone()])),
    ])
});

crate::registry::exports! {
    "format/lsp-schema" {
        "LSP_TRANSPORTS" => ok || LSP_TRANSPORTS;
    }
}
