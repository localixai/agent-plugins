//! Ported from `src/format/hooks-handlers.ts`; only the core uses it now, so it has no wrapper.
//!
//! Hook handlers (hooks.md, "Hook handler fields"): the five types and the fields each accepts,
//! bound to the official `HookHandlerOf<T>` so an SDK update that adds a field fails the typecheck
//! here. The common fields apply to every type; the rest are the type's own.
//!
//! A handler (the official `HookHandler`) is held as the object zod gives, a `JsValue`: its type
//! is the SDK's and stays in the TypeScript wrapper.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error, warning};
use crate::js::{JsValue, is_whitespace, string_of};
use crate::protocol::hook_config::HOOK_HANDLER_TYPES;
use crate::protocol::is_record::is_record;
use crate::zod::{self, Issue, Schema, Shape, fields};

// Judgment (spec/judgments.md): a timeout is a positive, finite number of seconds.
const SECONDS_MESSAGE: &str = "expected a positive, finite number of seconds";
static SECONDS: LazyLock<Schema> = LazyLock::new(|| {
    zod::number()
        .error(SECONDS_MESSAGE)
        .positive_with(SECONDS_MESSAGE)
        .finite()
});
static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
static FLAG: LazyLock<Schema> = LazyLock::new(zod::boolean);
static TEXT_LIST: LazyLock<Schema> = LazyLock::new(|| zod::array(TEXT.clone()));

/// `type` is the discriminator; the other common fields are optional on every type.
fn common() -> Shape {
    fields! {
        "if" => TEXT.exact_optional(),
        "timeout" => SECONDS.exact_optional(),
        "statusMessage" => TEXT.exact_optional(),
        "once" => FLAG.exact_optional(),
    }
}

static ON_FAILURE: LazyLock<Schema> = LazyLock::new(|| zod::r#enum(["continue", "block"]));

static COMMAND_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "type" => zod::literal("command"),
        // Judgment: an empty command names nothing to run.
        "command" => TEXT.min(1.0),
        "args" => TEXT_LIST.exact_optional(),
        "async" => FLAG.exact_optional(),
        "asyncRewake" => FLAG.exact_optional(),
        "shell" => zod::r#enum(["bash", "powershell"]).exact_optional(),
        "onFailure" => ON_FAILURE.exact_optional(),
        ..common(),
    })
});

static HTTP_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "type" => zod::literal("http"),
        "url" => TEXT.clone(),
        "headers" => zod::record(TEXT.clone(), TEXT.clone()).exact_optional(),
        "allowedEnvVars" => TEXT_LIST.exact_optional(),
        "onFailure" => ON_FAILURE.exact_optional(),
        ..common(),
    })
});

static MCP_TOOL_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "type" => zod::literal("mcp_tool"),
        "server" => TEXT.clone(),
        "tool" => TEXT.clone(),
        "input" => zod::record(TEXT.clone(), zod::unknown()).exact_optional(),
        ..common(),
    })
});

static PROMPT_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "type" => zod::literal("prompt"),
        "prompt" => TEXT.clone(),
        "model" => TEXT.exact_optional(),
        // In the official type, not in the docs' field table: what `ok: false` does (hooks.md, "Prompt-based hooks").
        "continueOnBlock" => FLAG.exact_optional(),
        ..common(),
    })
});

static AGENT_SCHEMA: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "type" => zod::literal("agent"),
        "prompt" => TEXT.clone(),
        "model" => TEXT.exact_optional(),
        ..common(),
    })
});

/// The schema of each handler type, one entry per member of the official union.
fn handler_schema(kind: &str) -> Option<&'static Schema> {
    match kind {
        "command" => Some(&COMMAND_SCHEMA),
        "http" => Some(&HTTP_SCHEMA),
        "mcp_tool" => Some(&MCP_TOOL_SCHEMA),
        "prompt" => Some(&PROMPT_SCHEMA),
        "agent" => Some(&AGENT_SCHEMA),
        _ => None,
    }
}

fn quoted(names: &[&str]) -> String {
    names
        .iter()
        .map(|name| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Exec form (hooks.md, "Exec form and shell form"): `command` is an executable name or path only.
fn exec_form_issues(handler: &JsValue, path: &str) -> Vec<PluginIssue> {
    let Some(command) = handler.get("command").and_then(JsValue::as_str) else {
        return Vec::new();
    };
    if matches!(handler.get("args"), None | Some(JsValue::Undefined)) {
        return Vec::new();
    }
    // `/[/\\]/` and `/\s/`, `\s` being JavaScript's whitespace (RS6).
    let bare = !command.contains(['/', '\\']);
    if bare && command.chars().any(is_whitespace) {
        vec![warning(
            &format!("{path}.command"),
            &format!(
                "\"{command}\" holds whitespace beside \"args\": exec form spawns an executable of that name, so the spawn will fail. Move the extra tokens into \"args\""
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Reported at the field, whatever depth inside it the value is wrong (`args`, not `args.0`).
fn field_problem(path: &str, issue: &Issue) -> PluginIssue {
    match issue.path().first() {
        None | Some(JsValue::Undefined) => {
            error(path, &format!("Invalid hook handler: {}", issue.message()))
        }
        Some(field) => {
            let field = string_of(field);
            error(
                &format!("{path}.{field}"),
                &format!("Invalid \"{field}\": {}", issue.message()),
            )
        }
    }
}

/// What reading one handler gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HandlerRead {
    /// Undefined when the handler has an error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handler: Option<JsValue>,
    /// What is wrong with it.
    pub issues: Vec<PluginIssue>,
}

/// Reads one handler object at `path` (`hooks.PreToolUse[0].hooks[1]`).
#[must_use]
pub fn read_handler(raw: &JsValue, path: &str) -> HandlerRead {
    if !is_record(raw) {
        return HandlerRead {
            handler: None,
            issues: vec![error(path, "A hook handler must be an object")],
        };
    }
    let Some(schema) = raw
        .get("type")
        .and_then(JsValue::as_str)
        .and_then(handler_schema)
    else {
        let message = format!(
            "Invalid \"type\": expected one of {}",
            quoted(HOOK_HANDLER_TYPES)
        );
        return HandlerRead {
            handler: None,
            issues: vec![error(&format!("{path}.type"), &message)],
        };
    };
    // A field no handler type has, or one only another type has, is dropped without a warning
    // (Agent Plugins Extended §7.2.3, Judgment J7.10; oracle 2.1.325 reports nothing for it).
    match schema.safe_parse(raw) {
        Err(failure) => HandlerRead {
            handler: None,
            issues: failure
                .issues
                .iter()
                .map(|issue| field_problem(path, issue))
                .collect(),
        },
        Ok(handler) => {
            let issues = exec_form_issues(&handler, path);
            HandlerRead {
                handler: Some(handler),
                issues,
            }
        }
    }
}
