//! The core's counterpart of `src/host/session-skill-expand.ts`, which stays TypeScript until the
//! session's state and the host's closures it reads (`commandEnvironment` over the session's
//! subprocess, `shellPermission`, the turn's ports) are the core's: here, what the core's own
//! session parts need of it, a skill's content as its file holds it now (`contentOf`, for
//! `session-skill-preload`).

use serde::Deserialize;

use crate::format::issues::error;
use crate::format::skill_types::BodyReader;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::ports::Signal;
use crate::registry::catch_host;

/// An entry as the content reads it: its head, its place and its reader.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentEntry {
    /// The skill or command without its body.
    pub invocable: JsObject,
    /// Reads its body.
    pub read_body: BodyReader,
    /// The `SKILL.md` or the command file, absolute.
    #[serde(default)]
    pub file: Option<String>,
    /// The skill's folder, absolute.
    #[serde(default)]
    pub directory: Option<String>,
}

/// An expansion that failed: also what a read of a skill's content returns when it fails.
#[must_use]
pub fn failed(name: &str, message: &str) -> JsValue {
    let issue = to_js_value(&error(&format!("/{name}"), message)).unwrap_or_default();
    let mut failure = JsObject::new();
    failure.insert("ok".to_owned(), JsValue::Bool(false));
    failure.insert("issues".to_owned(), JsValue::Array(vec![issue]));
    JsValue::Object(failure)
}

fn command_of(entry: &ContentEntry) -> &str {
    entry.invocable.get("command").and_then(JsValue::as_str).unwrap_or_default()
}

/// The body as the skill's file holds it now; a file the port refuses is the skill's failure, not a throw.
async fn body_of(entry: &ContentEntry, signal: Option<&Signal>) -> Result<String, JsValue> {
    let command = command_of(entry);
    match catch_host(entry.read_body.read(signal)).await {
        Ok(Some(body)) => Ok(body),
        Ok(None) => Err(failed(command, "The skill file is gone or no longer loads as a skill")),
        Err(cause) => {
            if signal.is_some_and(Signal::is_aborted) {
                return cause.rethrow().await;
            }
            let message = format!("The skill file cannot be read: {}", cause.message);
            Err(failed(command, &message))
        }
    }
}

/// The head with its place as the entry holds it, absolute: `${CLAUDE_SKILL_DIR}` is the row's
/// `directory`, computed once when the entries were built.
fn located_of(entry: &ContentEntry) -> JsObject {
    let mut located = entry.invocable.clone();
    if let Some(file) = &entry.file {
        located.insert("path".to_owned(), file.as_str().into());
    }
    if let Some(directory) = &entry.directory
        && located.contains_key("dir")
    {
        located.insert("dir".to_owned(), directory.as_str().into());
    }
    located
}

/// The skill with its body as the file holds it now and its absolute place, or the failure.
///
/// # Errors
///
/// The failure (`{ ok: false, issues }`) when the body cannot be read.
pub async fn content_of(entry: &ContentEntry, signal: Option<&Signal>) -> Result<JsObject, JsValue> {
    let body = body_of(entry, signal).await?;
    let mut content = located_of(entry);
    content.insert("body".to_owned(), body.into());
    Ok(content)
}
