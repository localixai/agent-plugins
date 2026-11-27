//! Port of `src/rules/tools/responses/agent-response.ts`, switched.
//!
//! The two members of a foreground or background Agent call's `tool_response` that hooks.md ("Agent")
//! documents. Each type is a `Pick` of the SDK's `AgentOutput` member in the TypeScript, so a field
//! the SDK renames or retypes breaks the typecheck there and not in a hook. Fields the table does not
//! list (`agentType`, `toolStats`, a block's `citations`, the SDK's remote launch) are left out
//! (spec/judgments.md).
//!
//! A response is held as the object the TypeScript builds, its members in the TypeScript's order;
//! `content` and `usage` are zod's output.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::shape::{RESPONSE_ROOT, issues_of, object_of};
use crate::format::issues::{PluginIssue, error, warning};
use crate::js::{JsObject, JsValue};
use crate::protocol::is_record::is_record;
use crate::rules::tools::inputs::field_schemas::{NUM, TEXT, list};
use crate::zod::{self, Schema, Shape, fields};

/// What reading a response gives.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AgentResponseParse {
    /// `undefined` when the response could not be read as either member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<JsObject>,
    /// Each problem, at `tool_response.<field>`.
    pub issues: Vec<PluginIssue>,
}

/// `resolvedModel` and `modelsUsed` of either member, kept as received: the docs describe what
/// Claude Code emits ("consecutive repeats collapsed; set only when the model was swapped"), not a
/// rule for the reader.
fn model_fields(data: &JsValue, response: &mut JsObject) {
    for name in ["resolvedModel", "modelsUsed"] {
        if let Some(value) = data.get(name).filter(|value| !value.is_undefined()) {
            response.insert(name.to_owned(), value.clone());
        }
    }
}

fn models() -> Shape {
    fields! {
        "resolvedModel" => TEXT.optional(),
        "modelsUsed" => list(TEXT.clone()).optional(),
    }
}

static COMPLETED: LazyLock<Schema> = LazyLock::new(|| {
    object_of(fields! {
        "status" => zod::literal("completed"),
        "agentId" => TEXT.clone(),
        "content" => list(object_of(fields! {
            "type" => zod::literal("text").error("expected \"text\""),
            "text" => TEXT.clone(),
        })),
        ..models(),
        "totalTokens" => NUM.clone(),
        "totalDurationMs" => NUM.clone(),
        "totalToolUseCount" => NUM.clone(),
        "usage" => object_of(fields! {
            "input_tokens" => NUM.clone(),
            "output_tokens" => NUM.clone(),
            "cache_creation_input_tokens" => NUM.nullable(),
            "cache_read_input_tokens" => NUM.nullable(),
        }),
    })
});

static LAUNCHED: LazyLock<Schema> = LazyLock::new(|| {
    object_of(fields! {
        "status" => zod::literal("async_launched"),
        "agentId" => TEXT.clone(),
        "description" => TEXT.clone(),
        "prompt" => TEXT.clone(),
        "outputFile" => TEXT.clone(),
        ..models(),
    })
});

type Reader = fn(&JsObject) -> AgentResponseParse;

const fn failed(issues: Vec<PluginIssue>) -> AgentResponseParse {
    AgentResponseParse {
        response: None,
        issues,
    }
}

/// The members of `data` named, in that order, each as zod gave it.
fn picked(data: &JsValue, names: &[&str]) -> JsObject {
    names
        .iter()
        .filter_map(|name| Some(((*name).to_owned(), data.get(name)?.clone())))
        .collect()
}

fn read_completed(raw: &JsObject) -> AgentResponseParse {
    let data = match COMPLETED.safe_parse(&JsValue::from(raw.clone())) {
        Ok(data) => data,
        Err(failure) => return failed(issues_of(RESPONSE_ROOT, &failure)),
    };
    let mut response = picked(&data, &["status", "agentId", "content"]);
    model_fields(&data, &mut response);
    response.extend(picked(
        &data,
        &[
            "totalTokens",
            "totalDurationMs",
            "totalToolUseCount",
            "usage",
        ],
    ));
    AgentResponseParse {
        response: Some(response),
        issues: Vec::new(),
    }
}

/// "A background launch ... `tool_response` carries no usage fields": these are not read there.
const USAGE_FIELDS: [&str; 4] = [
    "totalTokens",
    "totalDurationMs",
    "totalToolUseCount",
    "usage",
];

fn read_launched(raw: &JsObject) -> AgentResponseParse {
    let unexpected: Vec<PluginIssue> = USAGE_FIELDS
        .iter()
        .filter(|field| raw.contains_key(**field))
        .map(|field| {
            warning(
                &format!("{RESPONSE_ROOT}.{field}"),
                "a background launch carries no usage fields, so this one is ignored",
            )
        })
        .collect();
    let data = match LAUNCHED.safe_parse(&JsValue::from(raw.clone())) {
        Ok(data) => data,
        Err(failure) => {
            let mut issues = issues_of(RESPONSE_ROOT, &failure);
            issues.extend(unexpected);
            return failed(issues);
        }
    };
    let mut response = picked(
        &data,
        &["status", "agentId", "description", "prompt", "outputFile"],
    );
    model_fields(&data, &mut response);
    AgentResponseParse {
        response: Some(response),
        issues: unexpected,
    }
}

/// One reader per member of the union: a status is read by the reader of that name.
const READERS: [(&str, Reader); 2] = [
    ("completed", read_completed),
    ("async_launched", read_launched),
];

/// Reads the `tool_response` a PostToolUse hook receives for an Agent call (hooks.md, "Agent"): the
/// `completed` response of a foreground subagent or the `async_launched` response of a background
/// one. A wrong type or a missing field is an issue and nothing throws; `resolvedModel` is accepted
/// whatever the request's `model` said ("which may differ from the requested model").
#[must_use]
pub fn parse_agent_response(raw: &JsValue) -> AgentResponseParse {
    let Some(members) = raw.as_object().filter(|_| is_record(raw)) else {
        return failed(vec![error(RESPONSE_ROOT, "expected an object")]);
    };
    let status = members.get("status");
    let reader = READERS
        .iter()
        .find(|(name, _)| status.and_then(JsValue::as_str) == Some(*name));
    if let Some((_, read)) = reader {
        return read(members);
    }
    let message = if status.is_none_or(JsValue::is_undefined) {
        "is required".to_owned()
    } else {
        let names: Vec<&str> = READERS.iter().map(|(name, _)| *name).collect();
        format!("expected one of {}", names.join(", "))
    };
    failed(vec![error(&format!("{RESPONSE_ROOT}.status"), &message)])
}

crate::registry::exports! {
    "rules/tools/responses/agent-response" {
        "parseAgentResponse" => ok |raw: JsValue| parse_agent_response(&raw);
    }
}
