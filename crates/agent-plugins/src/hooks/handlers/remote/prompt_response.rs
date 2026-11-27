//! Port of `src/hooks/handlers/remote/prompt-response.ts`, switched.
//!
//! The answer of a `prompt` or `agent` hook (hooks.md, "Response schema"):
//! `{ "ok": true | false, "reason": "...", "impossible": true | false }`. `reason` is "required when
//! `ok` is `false`"; `impossible` is optional and "agent hooks ... ignore it".

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::js::json_parse::json_parse;
use crate::js::{JsValue, Pattern, SPACE_CLASS, trim};
use crate::zod::{self, Schema, fields};

static VERDICT: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        "ok" => zod::boolean(),
        "reason" => zod::string().optional(),
        "impossible" => zod::boolean().optional(),
    })
});

/// A reply wrapped in one Markdown code fence ("```json ... ```") is read as the JSON inside.
/// JavaScript's `/^```[A-Za-z]*\s*\n([\s\S]*?)\n?```$/`, its `\s` spelled as JavaScript's (RS6).
static FENCE: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"^```[A-Za-z]*[{SPACE_CLASS}]*\n((?s:.)*?)\n?```$"
    ))
    .ok()
});

/// The JSON of the reply, or `undefined` for text that is not JSON.
fn parse_json(text: &str) -> JsValue {
    let trimmed = trim(text);
    let inner = FENCE
        .as_ref()
        .and_then(|fence| fence.captures(trimmed))
        .and_then(|captures| captures.get(1))
        .map_or(trimmed, |inner| inner.as_str());
    json_parse(inner).unwrap_or(JsValue::Undefined)
}

/// `ModelRun` of `types.ts`: a model's verdict, `kind: "model"`. Held here until `types.ts` is
/// ported, the one port of it that names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename = "model")]
pub struct ModelRun {
    /// Whether the condition holds.
    pub ok: bool,
    /// Why it does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Prompt hooks only: the model judged the condition can never be satisfied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impossible: Option<bool>,
}

/// What reading a reply gives: the verdict, or why the reply is not one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ParsedVerdict {
    /// The reply is a verdict.
    Verdict {
        /// The verdict read.
        verdict: ModelRun,
    },
    /// The reply is no verdict.
    Failure {
        /// Why.
        failure: String,
    },
}

/// The options both exports take.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerdictOptions {
    /// Whether the hook reads `impossible` (prompt hooks do, agent hooks ignore it).
    pub impossible: bool,
}

fn failure(text: &str) -> ParsedVerdict {
    ParsedVerdict::Failure {
        failure: text.to_owned(),
    }
}

/// Judgment (spec/judgments.md): a reply that is not exactly one JSON object of this schema (prose
/// around it, a missing `ok`, `ok: false` with no reason) is no verdict, so the hook ends as a
/// non-blocking error instead of allowing or blocking on a guess.
#[must_use]
pub fn parse_verdict(text: &str, options: VerdictOptions) -> ParsedVerdict {
    let Ok(parsed) = VERDICT.safe_parse(&parse_json(text)) else {
        return failure(
            r#"The model did not answer with the JSON object {"ok": ..., "reason": ...}."#,
        );
    };
    let ok = parsed.get("ok").and_then(JsValue::as_bool).unwrap_or(false);
    let reason = parsed
        .get("reason")
        .and_then(JsValue::as_str)
        .map(str::to_owned);
    let impossible = parsed.get("impossible").and_then(JsValue::as_bool);
    if !ok
        && reason
            .as_deref()
            .is_none_or(|reason| trim(reason).is_empty())
    {
        return failure(r#"The model answered "ok": false without the required "reason"."#);
    }
    ParsedVerdict::Verdict {
        verdict: ModelRun {
            ok,
            reason,
            impossible: (options.impossible && impossible == Some(true)).then_some(true),
        },
    }
}

/// What the model is told about its answer: the doc gives the schema and the port a model call is
/// only a prompt, so the instruction travels with it (judgment, spec/judgments.md).
#[must_use]
pub fn response_instruction(options: VerdictOptions) -> String {
    let mut lines = vec![
        r#"Respond with only a JSON object and nothing else: {"ok": true} to allow, or {"ok": false, "reason": "<why>"} to block; "reason" is required when "ok" is false."#,
    ];
    if options.impossible {
        lines.push(
            r#"When "ok" is false and you judge the condition can never be satisfied, also set "impossible": true."#,
        );
    }
    lines.join(" ")
}

crate::registry::exports! {
    "hooks/handlers/remote/prompt-response" {
        "parseVerdict" => ok |text: String, options: VerdictOptions| parse_verdict(&text, options);
        "responseInstruction" => ok |options: VerdictOptions| response_instruction(options);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The TypeScript before the switch, `parseVerdict(text, { impossible })` on each case, printed
    // with `JSON.stringify`.
    #[test]
    fn a_fenced_reply_is_read_as_the_typescript_read_it() {
        let cases = [
            (
                "```json\n{\"ok\":true}\n```",
                false,
                r#"{"verdict":{"kind":"model","ok":true}}"#,
            ),
            (
                "  ```\n{\"ok\": false, \"reason\": \"no\", \"impossible\": true}```  ",
                true,
                r#"{"verdict":{"kind":"model","ok":false,"reason":"no","impossible":true}}"#,
            ),
            (
                "```json \t\n{\"ok\":false,\"reason\":\"  \"}\n```",
                false,
                r#"{"failure":"The model answered \"ok\": false without the required \"reason\"."}"#,
            ),
            (
                "{\"ok\":false,\"reason\":\"x\",\"impossible\":true}",
                false,
                r#"{"verdict":{"kind":"model","ok":false,"reason":"x"}}"#,
            ),
            (
                "```\n{\"ok\":true}\n``` trailing",
                false,
                r#"{"failure":"The model did not answer with the JSON object {\"ok\": ..., \"reason\": ...}."}"#,
            ),
        ];
        assert!(FENCE.is_some());
        for (text, impossible, want) in cases {
            let got = serde_json::to_string(&parse_verdict(text, VerdictOptions { impossible }));
            assert_eq!(got.ok().as_deref(), Some(want), "{text}");
        }
    }
}
