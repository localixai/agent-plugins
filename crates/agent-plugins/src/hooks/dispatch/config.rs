//! What the host fixes once for its dispatches (`DispatchConfig` of `src/hooks/dispatch.ts`) and
//! what one dispatch is asked (`DispatchRequest` of `src/hooks/outcome.ts`), as the engine holds
//! them: the host's ports by number, its options as the run context spells them.

use std::sync::Arc;

use serde::Deserialize;

use crate::hooks::interrupt::UserInterrupt;
use crate::hooks::output::merge::TaggedOutput;
use crate::hooks::output::stage::OutputStage;
use crate::hooks::verdict::debug_lines::Parser;
use crate::hooks::verdict::stop_block_cap::{CapWhere, StopBlockCap};
use crate::hooks::verdict::types::MergedOutput;
use crate::js::value::to_js_value;
use crate::js::{JsObject, JsValue};
use crate::ports::Signal;
use crate::registry::{Callback, Port};

/// One dispatch's request (`DispatchRequest`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchRequest {
    /// The event.
    pub event: String,
    /// Its facts.
    #[serde(default)]
    pub facts: JsValue,
    /// The session (`HookSession`), as the host gave it.
    #[serde(default)]
    pub session: JsValue,
    /// The session's abort.
    pub signal: Signal,
    /// The key the user pressed while a tool ran.
    #[serde(default)]
    pub interrupt: Option<UserInterrupt>,
    /// What completed a task, for `TaskCompleted`.
    #[serde(default)]
    pub task_completed_by: Option<String>,
}

impl DispatchRequest {
    /// A member of the session, as given.
    #[must_use]
    pub fn session_text(&self, name: &str) -> JsValue {
        self.session.get(name).cloned().unwrap_or_default()
    }

    /// A text member of the session, empty when absent.
    #[must_use]
    pub fn session_string(&self, name: &str) -> String {
        self.session
            .get(name)
            .and_then(JsValue::as_str)
            .unwrap_or_default()
            .to_owned()
    }
}

/// The output stage of an engine: the core's (`createOutputStage`), or one the host gave.
#[derive(Clone, Copy, Debug)]
pub enum Stage {
    /// The real stage.
    Core(OutputStage),
    /// The host's stage object.
    Host(Port),
}

/// The outputs with their text capped, and why any was cut (`LimitedOutputs`).
#[derive(Debug, Default, Deserialize)]
pub struct Limited {
    /// The outputs, in order.
    pub outputs: Vec<JsObject>,
    /// What capping found.
    #[serde(default)]
    pub issues: Vec<JsValue>,
}

impl Stage {
    /// How a verdict reads a hook's JSON with this stage.
    #[must_use]
    pub const fn parser(&self) -> Parser {
        match self {
            Self::Core(_) => Parser::Core,
            Self::Host(stage) => Parser::Stage(*stage),
        }
    }

    /// `limit(event, outputs, signal)`.
    pub async fn limit(&self, event: &str, outputs: Vec<JsObject>, signal: &Signal) -> Limited {
        match self {
            Self::Core(stage) => {
                let limited = stage.limit(event, outputs, signal.clone()).await;
                to_js_value(&limited)
                    .ok()
                    .and_then(|value| Limited::deserialize(value).ok())
                    .unwrap_or_default()
            }
            Self::Host(stage) => {
                let outputs = JsValue::Array(outputs.into_iter().map(JsValue::Object).collect());
                let signal = to_js_value(signal).unwrap_or_default();
                stage
                    .method("limit", vec![JsValue::from(event), outputs, signal])
                    .await
            }
        }
    }

    /// `merge(event, outputs, facts)`.
    pub async fn merge(
        &self,
        event: &str,
        outputs: Vec<JsObject>,
        facts: &JsValue,
    ) -> MergedOutput {
        match self {
            Self::Core(_) => {
                let tagged = outputs
                    .into_iter()
                    .filter_map(|output| TaggedOutput::deserialize(JsValue::Object(output)).ok())
                    .collect();
                let facts = if facts.is_undefined() {
                    JsValue::Object(JsObject::new())
                } else {
                    facts.clone()
                };
                OutputStage::merge(event, tagged, &facts)
            }
            Self::Host(stage) => {
                let outputs = JsValue::Array(outputs.into_iter().map(JsValue::Object).collect());
                let args = vec![JsValue::from(event), outputs, facts.clone()];
                stage.method("merge", args).await
            }
        }
    }
}

/// The counter of consecutive Stop blocks: the core's (`createStopBlockCap`), or the host's.
#[derive(Clone, Debug)]
pub enum Cap {
    /// The core's counter.
    Core(Arc<StopBlockCap>),
    /// The host's object.
    Host(Port),
}

/// What the host fixes once (`DispatchConfig`).
#[derive(Clone, Debug)]
pub struct DispatchConfig {
    /// The output stage.
    pub output: Stage,
    /// The ports of a run (`RunPorts`), as the run context carries them.
    pub run_ports: JsValue,
    /// The host's environment.
    pub environment: Port,
    /// The host's clock.
    pub clock: Option<Port>,
    /// The host's observer.
    pub observer: Option<Port>,
    /// Reads a handler's `if`.
    pub if_evaluator: Option<Callback>,
    /// Names the canonical model of a model switch.
    pub model_catalog: Option<Port>,
    /// Windows.
    pub windows: bool,
    /// What a command hook with no `shell` runs in.
    pub default_shell: String,
    /// The members every run context adds (`commandOptionsOf`, `userConfigOf`, `httpPolicy`,
    /// `envFiles`, `checkPluginDirectory`), each only when set.
    pub run_options: JsObject,
    /// `CLAUDE_ENV_FILE`: the files, and the clearing a `CwdChanged` event does.
    pub env_files: Option<Port>,
    /// The counter of consecutive Stop blocks; without it no cap applies.
    pub stop_cap: Option<Cap>,
    /// Where a background hook's text over the cap is saved.
    pub scratch: Option<Port>,
}

impl DispatchConfig {
    /// The outcome after the Stop cap, when it counts this dispatch.
    pub async fn capped(
        &self,
        outcome: JsObject,
        request: &DispatchRequest,
        capped: bool,
    ) -> JsObject {
        let Some(cap) = self.stop_cap.as_ref().filter(|_| capped) else {
            return outcome;
        };
        let at = CapWhere {
            session_id: request.session_string("sessionId"),
            facts: request.facts.clone(),
        };
        match cap {
            Cap::Core(cap) => cap.apply(outcome, at, self.environment).await,
            Cap::Host(cap) => {
                let at = JsObject::from_iter([
                    ("sessionId".to_owned(), JsValue::from(at.session_id)),
                    ("facts".to_owned(), at.facts),
                ]);
                let environment = to_js_value(&self.environment).unwrap_or_default();
                let args = vec![JsValue::Object(outcome), JsValue::Object(at), environment];
                cap.method("apply", args).await
            }
        }
    }

    /// Starts the Stop count again for the session (and its subagent): Claude called a tool.
    pub async fn reset_cap(&self, request: &DispatchRequest) {
        let session_id = request.session_string("sessionId");
        let agent = request
            .session
            .get("agent")
            .and_then(|agent| agent.get("id"));
        let agent = agent.and_then(JsValue::as_str).map(str::to_owned);
        match &self.stop_cap {
            Some(Cap::Core(cap)) => cap.reset(&session_id, agent.as_deref()),
            Some(Cap::Host(cap)) => {
                let agent = agent.map_or(JsValue::Undefined, JsValue::from);
                let _: JsValue = cap
                    .method("reset", vec![JsValue::from(session_id), agent])
                    .await;
            }
            None => {}
        }
    }

    /// Clears the environment files when the event does (`EnvFiles.beforeEvent`).
    pub async fn before_event(&self, event: &str, signal: &Signal) {
        if let Some(files) = self.env_files {
            let signal = to_js_value(signal).unwrap_or_default();
            let _: JsValue = files
                .method("beforeEvent", vec![JsValue::from(event), signal])
                .await;
        }
    }
}
