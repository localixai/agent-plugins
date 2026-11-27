//! [`Port`] as the host's models (`src/ports/model-runner.ts`): `ModelAnswer | PortFailure`, read
//! as a `Result`. An `AgentRequest` crosses as the TypeScript's, the question's members with
//! `tools`, `readOnly: true` and `maxTurns`.

use serde::Serialize;

use super::{Port, arg};
use crate::ports::model_runner::{AgentRequest, ModelAnswer, ModelRequest, ModelRunner};
use crate::ports::{BoxFuture, PortFailure};

/// An `AgentRequest` as the TypeScript types it: a `ModelRequest` that is read-only.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Agent<'a> {
    #[serde(flatten)]
    request: &'a ModelRequest,
    tools: &'a [String],
    read_only: bool,
    max_turns: u32,
}

impl ModelRunner for Port {
    fn evaluate(&self, request: ModelRequest) -> BoxFuture<'_, Result<ModelAnswer, PortFailure>> {
        self.ask_failable("evaluate", vec![arg(Some(&request))])
    }

    fn run_agent(&self, request: AgentRequest) -> BoxFuture<'_, Result<ModelAnswer, PortFailure>> {
        let agent = Agent {
            request: &request.request,
            tools: &request.tools,
            read_only: true,
            max_turns: request.max_turns,
        };
        self.ask_failable("runAgent", vec![arg(Some(&agent))])
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{exchange, json, need, port};
    use crate::ports::Signal;
    use crate::ports::model_runner::{AgentRequest, ModelRequest, ModelRunner as _};

    fn question() -> ModelRequest {
        ModelRequest {
            prompt: "ok?".to_owned(),
            model: None,
            model_preference: None,
            timeout_ms: Some(30),
            signal: Signal::new(),
        }
    }

    #[test]
    fn a_question_and_an_agent_run_cross_as_the_typescript_names_them() {
        let (asked, read) = exchange(
            async { port().evaluate(question()).await },
            json(r#"{"text": "yes"}"#),
        );
        let signal = r#""signal": {"$signal": null, "aborted": false}"#;
        let sent = json(&format!(
            r#"{{"prompt": "ok?", "timeoutMs": 30, {signal}}}"#
        ));
        assert_eq!(asked, need("evaluate", vec![sent]));
        assert_eq!(read, r#"Ok(ModelAnswer { text: "yes" })"#);
        let agent = AgentRequest {
            request: question(),
            tools: vec!["Read".to_owned()],
            max_turns: 3,
        };
        let (asked, read) = exchange(
            async move { port().run_agent(agent).await },
            json(r#"{"failure": "timeout", "message": "slow"}"#),
        );
        let sent = json(&format!(
            r#"{{"prompt": "ok?", "timeoutMs": 30, {signal}, "tools": ["Read"], "readOnly": true, "maxTurns": 3}}"#
        ));
        assert_eq!(asked, need("runAgent", vec![sent]));
        assert_eq!(
            read,
            r#"Err(PortFailure { failure: "timeout", message: "slow" })"#
        );
    }
}
