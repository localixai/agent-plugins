//! Port of `src/format/mcp-elicitation.ts`, switched.
//!
//! MCP elicitation (mcp.md, "Respond to MCP elicitation requests", "Elicitation modes Claude Code
//! declares"): a server asks for input mid-task, in form mode (fields the server defines) or URL
//! mode (a link to open). Pure data: the dialog, the browser and the connection are the host's.
//! The request, the answer and the hook fields are the official types, read here as `JsValue`s.

use std::cmp::Ordering;

use serde::Deserialize;

use super::mcp_elicitation_form::{content_of, form_answer};
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue, code_unit_length, compare_code_units};

/// One of `ElicitationMode`.
const FORM: &str = "form";
const URL: &str = "url";

/// A connection an elicitation capability is declared on.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpConnection {
    /// The MCP protocol revision the connection runs on, such as `2026-07-28`.
    pub protocol_revision: String,
    /// A stdio, sse, http or ws server of the user's Claude Code MCP configuration (not a
    /// claude.ai connector).
    pub user_configured: bool,
    /// The server entry sets `"bareElicitationCapability": true` (errors.md, "MCP server rejected
    /// the elicitation capability"): it is offered the bare `{}`, form mode only, on any revision.
    #[serde(default)]
    pub bare_elicitation_capability: Option<bool>,
}

/// "Connections that use protocol revision 2026-07-28".
pub const NEW_REVISION: &str = "2026-07-28";

/// `{}`, or `{form: {}, url: {}}`.
fn capability(both: bool) -> JsObject {
    if both {
        [FORM, URL]
            .iter()
            .map(|mode| ((*mode).to_owned(), JsValue::Object(JsObject::new())))
            .collect()
    } else {
        JsObject::new()
    }
}

/// What `elicitation` is declared on a connection: `{form: {}, url: {}}` on revision 2026-07-28
/// and later, and on earlier revisions to the servers of a user's configuration; the bare `{}` on
/// other connections of the earlier revisions, and to a server entry with
/// `bareElicitationCapability: true` (spec §17, "Elicitation capability rejected"). Revisions are
/// dates, so they compare as text.
#[must_use]
pub fn declared_elicitation(connection: &McpConnection) -> JsObject {
    if connection.bare_elicitation_capability == Some(true) {
        return capability(false);
    }
    let both = compare_code_units(&connection.protocol_revision, NEW_REVISION) != Ordering::Less
        || connection.user_configured;
    capability(both)
}

/// The modes a server may request: "the bare `elicitation: {}` ... permits elicitation requests in
/// form mode only".
#[must_use]
pub fn elicitation_modes(capability: &JsValue) -> Vec<&'static str> {
    let declared: Vec<&'static str> = [FORM, URL]
        .into_iter()
        .filter(|mode| {
            capability
                .get(mode)
                .is_some_and(|value| !value.is_undefined())
        })
        .collect();
    if declared.is_empty() {
        vec![FORM]
    } else {
        declared
    }
}

/// "Caps how long that argument can be": 8,000, "A URL with none of them reaches the cap at about
/// 8,000 characters", and "every third character a `%` ... roughly 4,000".
pub const ELICITATION_URL_ARGUMENT_CAP: usize = 8_000;

/// Judgment (spec/judgments.md): the docs name `%` and `&` as examples of "every character that
/// needs escaping"; the kit takes the characters a Windows command line treats specially:
/// `/[%&^|<>()"!]/g`.
const fn needs_escaping(character: char) -> bool {
    matches!(
        character,
        '%' | '&' | '^' | '|' | '<' | '>' | '(' | ')' | '"' | '!'
    )
}

/// "Its own character plus three escape characters": each such character counts four times.
#[must_use]
pub fn url_argument_length(url: &str) -> usize {
    code_unit_length(url)
        + 3 * url
            .chars()
            .filter(|character| needs_escaping(*character))
            .count()
}

/// What the person can do with a URL request: over the cap "you can only decline the request".
#[must_use]
pub fn elicitation_url_choices(url: &str) -> Vec<&'static str> {
    if url_argument_length(url) > ELICITATION_URL_ARGUMENT_CAP {
        vec!["decline"]
    } else {
        vec!["open", "decline"]
    }
}

/// An object of the members given, in their order.
fn object(members: impl IntoIterator<Item = (&'static str, JsValue)>) -> JsValue {
    JsValue::Object(
        members
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

/// URL mode carries no content: the answer is that the link was opened.
fn url_answer() -> JsValue {
    object([("result", object([("action", JsValue::from("accept"))]))])
}

/// `ANSWERS[mode]`.
fn answer_of(mode: &str, request: &JsValue, values: &JsObject) -> JsValue {
    match mode {
        URL => url_answer(),
        _ => to_js_value(&form_answer(request, values)).unwrap_or_default(),
    }
}

/// The answer to a request: a form's values checked against the fields its schema defines
/// (judgment, spec/judgments.md: the docs say the person "fills in the fields and submits", and do
/// not say how the client checks them; a value for a field the schema lacks is not sent), or the
/// acceptance of a link; declining or cancelling sends no content. A request in no mode is a form
/// (the MCP default).
#[must_use]
pub fn answer_elicitation(request: &JsValue, response: &JsValue) -> JsValue {
    if let Some(values) = response.get("values") {
        let mode = request
            .get("mode")
            .and_then(JsValue::as_str)
            .unwrap_or(FORM);
        let empty = JsObject::new();
        return answer_of(mode, request, values.as_object().unwrap_or(&empty));
    }
    let action = response.get("action").cloned().unwrap_or_default();
    object([("result", object([("action", action)]))])
}

/// What the `Elicitation` hook receives, in the official field names (hooks.md, "Elicitation").
#[must_use]
pub fn elicitation_hook_facts(request: &JsValue) -> JsObject {
    let member = |key: &str| request.get(key).cloned().unwrap_or_default();
    let mut facts = JsObject::new();
    facts.insert("mcp_server_name".to_owned(), member("serverName"));
    facts.insert("message".to_owned(), member("message"));
    for (from, to) in [
        ("mode", "mode"),
        ("url", "url"),
        ("elicitationId", "elicitation_id"),
        ("requestedSchema", "requested_schema"),
    ] {
        let value = member(from);
        if !value.is_undefined() {
            facts.insert(to.to_owned(), value);
        }
    }
    facts
}

/// An answer, with the content only the actions in `carrying` send. Judgment (spec/judgments.md):
/// MCP defines `content` for `accept`; the docs add that an `ElicitationResult` hook's `cancel`
/// goes to the server "along with the values the user submitted", and that a decline is "without
/// the user's values".
fn answered(action: &JsValue, content: Option<&JsValue>, carrying: &[&str]) -> JsValue {
    let carries = action.as_str().is_some_and(|name| carrying.contains(&name));
    match content.filter(|content| !content.is_undefined()) {
        Some(content) if carries => {
            let empty = JsObject::new();
            let values = content_of(content.as_object().unwrap_or(&empty));
            object([
                ("action", action.clone()),
                ("content", to_js_value(&values).unwrap_or_default()),
            ])
        }
        _ => object([("action", action.clone())]),
    }
}

/// What the session says about the dialog: with `permissions.denyInsteadOfPrompting` none is
/// shown.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ElicitationSession {
    /// `permissions.denyInsteadOfPrompting`.
    #[serde(default)]
    pub deny_instead_of_prompting: Option<bool>,
}

/// A member that is neither absent nor `undefined`.
fn present<'a>(value: &'a JsValue, key: &str) -> Option<&'a JsValue> {
    value.get(key).filter(|member| !member.is_undefined())
}

/// "To auto-respond to elicitation requests without showing a dialog, use the `Elicitation`
/// hook": an output that names an `action` is the answer; one without is ignored (hooks.md), and
/// the dialog shows (`None`). With `permissions.denyInsteadOfPrompting` there is no dialog: a
/// request no hook answers is cancelled (settings-reference.md, "What else
/// `denyInsteadOfPrompting` stops").
#[must_use]
pub fn auto_respond_elicitation(output: &JsValue, session: &ElicitationSession) -> Option<JsValue> {
    if let Some(action) = present(output, "action") {
        return Some(answered(action, output.get("content"), &["accept"]));
    }
    (session.deny_instead_of_prompting == Some(true))
        .then(|| object([("action", JsValue::from("cancel"))]))
}

/// The `ElicitationResult` hook "can override the action or content before the response is
/// sent".
#[must_use]
pub fn override_elicitation_result(result: &JsValue, output: &JsValue) -> JsValue {
    let nullish = |value: Option<&JsValue>| value.is_none_or(JsValue::is_nullish);
    let action = if nullish(output.get("action")) {
        result.get("action")
    } else {
        output.get("action")
    };
    let content = if nullish(output.get("content")) {
        result.get("content")
    } else {
        output.get("content")
    };
    answered(
        action.unwrap_or(&JsValue::Undefined),
        content,
        &["accept", "cancel"],
    )
}

crate::registry::exports! {
    "format/mcp-elicitation" {
        "declaredElicitation" => ok |connection: McpConnection| declared_elicitation(&connection);
        "elicitationModes" => ok |capability: JsValue| elicitation_modes(&capability);
        "ELICITATION_URL_ARGUMENT_CAP" => ok || ELICITATION_URL_ARGUMENT_CAP;
        "urlArgumentLength" => ok |url: String| url_argument_length(&url);
        "elicitationUrlChoices" => ok |url: String| elicitation_url_choices(&url);
        "answerElicitation" => ok |request: JsValue, response: JsValue| answer_elicitation(&request, &response);
        "elicitationHookFacts" => ok |request: JsValue| elicitation_hook_facts(&request);
        "autoRespondElicitation" => ok |output: JsValue, session: Option<ElicitationSession>| auto_respond_elicitation(&output, &session.unwrap_or_default());
        "overrideElicitationResult" => ok |result: JsValue, output: JsValue| override_elicitation_result(&result, &output);
    }
}

#[cfg(test)]
mod tests {
    use super::{elicitation_url_choices, url_argument_length};

    #[test]
    fn each_escaped_character_counts_four_times() {
        assert_eq!(url_argument_length("a%b&"), 10);
        assert_eq!(
            elicitation_url_choices(&"%".repeat(2_000)),
            vec!["open", "decline"]
        );
        assert_eq!(elicitation_url_choices(&"%".repeat(2_001)), vec!["decline"]);
    }
}
