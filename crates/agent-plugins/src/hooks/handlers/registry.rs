//! Port of `src/hooks/handlers/registry.ts`, switched.
//!
//! One runner per handler type (hooks.md, "Hook handler fields"): the TypeScript's table over the
//! official union is a `match` on the type here, so a type the SDK adds is a missed arm the
//! compiler does not see, and a hook of an unknown type is reported as not run.

use serde::ser::{SerializeMap, Serializer};
use serde::{Deserialize, Serialize};

use super::command::run_command::{CommandContext, CommandEntry, CommandRawRun, run_command};
use super::context::{BackgroundResult, RunContext};
use super::remote::agent::run_agent;
use super::remote::http::run_http;
use super::remote::mcp_tool::run_mcp_tool;
use super::remote::prompt::run_prompt;
use super::status::{StartObserver, StartedEntry, announce_start};
use super::timeouts::TimedEntry;
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::registry::{CorePromise, Port};

/// What a handler's run is: one running in the background, whose process the core awaits, or any
/// other run as the TypeScript's `HandlerRun` spells it.
#[derive(Debug)]
pub enum RunOf {
    /// It runs in the background.
    Background(CorePromise<BackgroundResult>),
    /// Any other run.
    Data(JsValue),
}

/// The raw run and the issues the run itself found (`HandlerResult`).
#[derive(Debug)]
pub struct HandlerRun {
    /// The run.
    pub run: RunOf,
    /// The issues.
    pub issues: Vec<JsValue>,
}

impl Serialize for HandlerRun {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        match &self.run {
            RunOf::Background(done) => {
                #[derive(Serialize)]
                struct Background<'a> {
                    kind: &'static str,
                    done: &'a CorePromise<BackgroundResult>,
                }
                let run = Background {
                    kind: "background",
                    done,
                };
                map.serialize_entry("run", &run)?;
            }
            RunOf::Data(run) => map.serialize_entry("run", run)?,
        }
        map.serialize_entry("issues", &self.issues)?;
        map.end()
    }
}

/// A run that did not happen, with why.
fn skipped(reason: String) -> HandlerRun {
    let run = JsValue::Object(
        [
            ("kind".to_owned(), JsValue::from("skipped")),
            ("reason".to_owned(), JsValue::from(reason)),
        ]
        .into_iter()
        .collect(),
    );
    HandlerRun {
        run: RunOf::Data(run),
        issues: Vec::new(),
    }
}

/// A run read back as data, with its issues.
fn handled<T: Serialize>(result: &T) -> HandlerRun {
    let value = to_js_value(result).unwrap_or_default();
    HandlerRun {
        run: RunOf::Data(value.get("run").cloned().unwrap_or_default()),
        issues: value
            .get("issues")
            .and_then(JsValue::as_array)
            .cloned()
            .unwrap_or_default(),
    }
}

/// The handler's type.
fn type_of(entry: &JsValue) -> String {
    entry
        .get("handler")
        .and_then(|handler| handler.get("type"))
        .map_or_else(|| "undefined".to_owned(), crate::js::string_of)
}

fn run_typed<E, C>(entry: &JsValue, context: &JsValue) -> Result<(E, C), String>
where
    E: for<'de> Deserialize<'de>,
    C: for<'de> Deserialize<'de>,
{
    let typed = E::deserialize(entry.clone()).map_err(|error| error.to_string())?;
    let read = C::deserialize(context.clone()).map_err(|error| error.to_string())?;
    Ok((typed, read))
}

async fn run_of_type(entry: &JsValue, context: &JsValue) -> Result<HandlerRun, String> {
    let kind = type_of(entry);
    Ok(match kind.as_str() {
        "command" => {
            let (entry, context) = run_typed::<CommandEntry, CommandContext>(entry, context)?;
            let result = run_command(&entry, context).await;
            let issues = to_js_value(&result.issues).unwrap_or_default();
            let issues = issues.as_array().cloned().unwrap_or_default();
            match result.run {
                CommandRawRun::Background(background) => HandlerRun {
                    run: RunOf::Background(background.done),
                    issues,
                },
                run @ (CommandRawRun::Foreground(_) | CommandRawRun::Skipped(_)) => HandlerRun {
                    run: RunOf::Data(to_js_value(&run).unwrap_or_default()),
                    issues,
                },
            }
        }
        "http" => {
            let (entry, context) = run_typed::<TimedEntry, RunContext>(entry, context)?;
            handled(&run_http(&entry, context).await)
        }
        "mcp_tool" => {
            let (entry, context) = run_typed::<TimedEntry, RunContext>(entry, context)?;
            handled(&run_mcp_tool(&entry, context).await)
        }
        "prompt" => {
            let (entry, context) = run_typed::<TimedEntry, RunContext>(entry, context)?;
            handled(&run_prompt(&entry, context).await)
        }
        "agent" => {
            let (entry, context) = run_typed::<TimedEntry, RunContext>(entry, context)?;
            handled(&run_agent(&entry, context).await)
        }
        _ => skipped(format!("No runner takes a {kind} hook.")),
    })
}

/// The observer of the run's ports: the context's own, read as a member when the context crossed
/// as the host's object.
async fn observer_of(context: &JsValue) -> Option<StartObserver> {
    let ports = match Port::deserialize(context.clone()) {
        Ok(port) => port.member::<JsValue>("ports").await,
        Err(_) => context.get("ports").cloned().unwrap_or_default(),
    };
    StartObserver::deserialize(ports.get("observer")?.clone()).ok()
}

/// Runs one handler: tells the host it started (`statusMessage`), then hands it to the runner of
/// its type. An entry or context its runner cannot read is a hook that could not start.
pub async fn run_handler(entry: &JsValue, context: &JsValue) -> HandlerRun {
    if let Ok(started) = StartedEntry::deserialize(entry.clone()) {
        announce_start(observer_of(context).await.as_ref(), &started).await;
    }
    match run_of_type(entry, context).await {
        Ok(run) => run,
        Err(message) => HandlerRun {
            run: RunOf::Data(JsValue::Object(
                [
                    ("kind".to_owned(), JsValue::from("not-started")),
                    ("message".to_owned(), JsValue::from(message)),
                ]
                .into_iter()
                .collect(),
            )),
            issues: Vec::new(),
        },
    }
}

crate::registry::exports! {
    "hooks/handlers/registry" {
        "runHandler" => async ok |entry: JsValue, context: JsValue| run_handler(&entry, &context).await;
    }
}
