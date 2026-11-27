//! Port of `src/format/validate/hook-handlers.ts`, switched.
//!
//! The hook handler schemas of `claude plugin validate` (oracle 2.1.319): five types, each checked
//! for the fields it needs and the types of the common ones. A handler that fails is reported by
//! type and by field, in the order the validator tests them; anything else in it is left alone.

use std::sync::LazyLock;

use super::received::with_article;
use super::shell_command::unquoted_message;
use crate::js::{JsObject, JsValue, string_of};
use crate::protocol::is_record::is_record;
use crate::zod::{self, Schema, Shape, fields};

static TEXT: LazyLock<Schema> = LazyLock::new(zod::string);
static FLAG: LazyLock<Schema> = LazyLock::new(zod::boolean);
static SECONDS: LazyLock<Schema> = LazyLock::new(|| zod::number().positive());

fn common() -> Shape {
    fields! {
        "if" => TEXT.optional(),
        "timeout" => SECONDS.optional(),
        "statusMessage" => TEXT.optional(),
        "once" => FLAG.optional(),
    }
}

static COMMAND: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "command" => TEXT.clone(),
        "args" => zod::array(TEXT.clone()).optional(),
        "shell" => zod::r#enum(["bash", "powershell"]).optional(),
        ..common(),
        "async" => FLAG.optional(),
        "asyncRewake" => FLAG.optional(),
    })
});
static PROMPT: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! { "prompt" => TEXT.clone(), ..common(), "model" => TEXT.optional() })
});
static AGENT: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! { "prompt" => TEXT.clone(), ..common(), "model" => TEXT.optional() })
});
static HTTP: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "url" => zod::url(),
        ..common(),
        "headers" => zod::record(TEXT.clone(), TEXT.clone()).optional(),
    })
});
static MCP_TOOL: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! { "server" => TEXT.clone(), "tool" => TEXT.clone(), ..common() })
});

/// `SCHEMAS[type]`, `None` for a type it has no entry for (`isHandlerType`).
fn schemas(kind: &str) -> Option<&'static Schema> {
    match kind {
        "command" => Some(&COMMAND),
        "prompt" => Some(&PROMPT),
        "agent" => Some(&AGENT),
        "http" => Some(&HTTP),
        "mcp_tool" => Some(&MCP_TOOL),
        _ => None,
    }
}

fn dotted_path(segments: &[JsValue]) -> String {
    segments.iter().map(string_of).collect::<Vec<_>>().join(".")
}

/// Why a handler cannot load, or `undefined` when it can.
#[must_use]
pub fn handler_problem(handler: &JsValue) -> Option<String> {
    if !is_record(handler) {
        return Some(format!(
            "Hook entry must be an object; received {}",
            with_article(handler)
        ));
    }
    let kind = match handler.get("type") {
        None | Some(JsValue::Undefined) => return Some("Hook entry has no \"type\"".to_owned()),
        Some(JsValue::String(kind)) => kind,
        Some(other) => {
            return Some(format!(
                "Hook entry \"type\" must be a string; received {}",
                with_article(other)
            ));
        }
    };
    let Some(schema) = schemas(kind) else {
        return Some(format!("Unknown hook type \"{kind}\""));
    };
    let failure = schema.safe_parse(handler).err()?;
    let fields = failure
        .issues
        .iter()
        .map(|found| format!("{}: {}", dotted_path(found.path()), found.message()))
        .collect::<Vec<_>>()
        .join("; ");
    Some(format!("Invalid {kind} hook ({fields})"))
}

/// The shell-form command of a handler, when it has one: no `args`, not PowerShell.
fn shell_command(handler: &JsObject) -> Option<&str> {
    let command = handler.get("command").and_then(JsValue::as_str)?;
    let args = handler.get("args");
    let shell = handler.get("shell");
    (matches!(args, None | Some(JsValue::Undefined))
        && shell.and_then(JsValue::as_str) != Some("powershell"))
    .then_some(command)
}

/// Handler types that run a shell command.
const SHELL_FORM: &[&str] = &["command"];

/// The unquoted-placeholder warning of a handler that loads, if it earns one.
#[must_use]
pub fn handler_advice(handler: &JsValue) -> Option<String> {
    let members = handler.as_object().filter(|_| is_record(handler))?;
    let kind = members.get("type").and_then(JsValue::as_str)?;
    if !SHELL_FORM.contains(&kind) {
        return None;
    }
    shell_command(members).and_then(unquoted_message)
}

crate::registry::exports! {
    "format/validate/hook-handlers" {
        "handlerProblem" => ok |handler: JsValue| handler_problem(&handler);
        "handlerAdvice" => ok |handler: JsValue| handler_advice(&handler);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::js::json_parse::json_parse;

    // The TypeScript before the switch: `[handlerProblem(h), handlerAdvice(h)]` for each handler,
    // printed with `JSON.stringify`.
    #[test]
    fn problems_and_advice_are_the_typescripts() {
        let cases = [
            (
                r#"{"type":"http","url":"nope","timeout":0}"#,
                Some(
                    "Invalid http hook (url: Invalid URL; timeout: Too small: expected number to be >0)",
                ),
                None,
            ),
            (
                r#"{"type":7}"#,
                Some("Hook entry \"type\" must be a string; received a number"),
                None,
            ),
            (r#"{"type":"command","command":"x"}"#, None, None),
            (
                r#"{"type":"command","command":"x","shell":"powershell"}"#,
                None,
                None,
            ),
            (r#"{"type":"command","command":"echo $FOO"}"#, None, None),
            (
                r#"{"type":"command","command":"echo ${CLAUDE_PLUGIN_ROOT}/x"}"#,
                None,
                Some(
                    "Shell command uses ${CLAUDE_PLUGIN_ROOT} without quotes: echo ${CLAUDE_PLUGIN_ROOT}/x. If the expanded path contains a space the command can split into several words and fail. Wrap the placeholder in double quotes, or use exec form: {\"command\": \"<executable>\", \"args\": [\"${CLAUDE_PLUGIN_ROOT}/...\"]}.",
                ),
            ),
            (
                "[]",
                Some("Hook entry must be an object; received an array"),
                None,
            ),
            (
                "null",
                Some("Hook entry must be an object; received null"),
                None,
            ),
            (
                r#"{"type":"mcp_tool"}"#,
                Some(
                    "Invalid mcp_tool hook (server: Invalid input: expected string, received undefined; tool: Invalid input: expected string, received undefined)",
                ),
                None,
            ),
            (
                r#"{"type":"command","command":"x","args":[1]}"#,
                Some(
                    "Invalid command hook (args.0: Invalid input: expected string, received number)",
                ),
                None,
            ),
            (
                r#"{"type":"toString"}"#,
                Some("Unknown hook type \"toString\""),
                None,
            ),
            (
                r#"{"type":"agent","prompt":"p","headers":{"a":1}}"#,
                None,
                None,
            ),
        ];
        for (text, problem, advice) in cases {
            let handler = json_parse(text).unwrap_or(JsValue::Undefined);
            assert_eq!(handler_problem(&handler).as_deref(), problem, "{text}");
            assert_eq!(handler_advice(&handler).as_deref(), advice, "{text}");
        }
    }
}
