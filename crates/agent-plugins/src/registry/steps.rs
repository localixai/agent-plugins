//! The host's loop over an export that waits on it, by name and as JSON text: [`start`], then
//! [`resume`] with the answers to the needs of each [`Step`] until it is done. The text forms,
//! [`start_json`] and [`resume_json`], are what `crates/agent-plugins-wasm` exports
//! (docs/RUST.md, "The boundary").

use serde::Deserialize;

use super::task::{self, Need, Reply, Step};
use super::{Answer, MODULES, decoded, handle, remote};
use crate::js::JsValue;
use crate::js::value::write_json_string;

/// Starts `name` (`<module>#<export>`, or `<handle>#<method>` for a method of a
/// [`Handle`](super::Handle)) with `args` and runs it until it is done or waits on the host.
#[must_use]
pub fn start(name: &str, args: &[JsValue]) -> Step {
    let Some((module, function)) = name.split_once('#') else {
        return Step::Done(Answer::Fault(format!(
            "`{name}` names no function: <module>#<function>"
        )));
    };
    let started = match module.parse::<u32>() {
        Ok(number) => handle::start(number, function, args),
        Err(_) => MODULES
            .iter()
            .find(|candidate| candidate.name == module)
            .and_then(|found| (found.start)(function, args)),
    };
    started.map_or_else(
        || Step::Done(Answer::Fault(format!("`{name}` is not exported"))),
        task::start,
    )
}

/// Gives task `number` the host's answers to its needs, by need, and the numbers of the host's
/// signals that aborted since, and runs it until it is done or waits again.
#[must_use]
pub fn resume(number: u32, answers: Vec<(u32, Reply)>, aborted: &[u32]) -> Step {
    for signal in aborted {
        remote::aborted(*signal);
    }
    task::resume(number, answers)
}

/// [`start`] with the arguments' JSON array in the wire's spelling; the step's JSON text.
#[must_use]
pub fn start_json(name: &str, args: &str) -> String {
    decoded(args)
        .map_or_else(
            |why| Step::Done(Answer::Fault(why)),
            |list| start(name, &list),
        )
        .to_json_text()
}

/// One answer as the host writes it: `{"id": n, "result": value}` or `{"id": n, "throws":
/// message}`.
#[derive(Deserialize)]
struct Given {
    id: u32,
    #[serde(default)]
    result: JsValue,
    throws: Option<String>,
}

/// What the host gives `resume`: `{"answers": [...], "aborted": [n...]}`, `aborted` optional.
#[derive(Deserialize)]
struct Resumed {
    answers: Vec<Given>,
    #[serde(default)]
    aborted: Vec<u32>,
}

/// [`resume`] with the answers' JSON text (`Resumed`); the step's JSON text.
#[must_use]
pub fn resume_json(number: u32, answers: &str) -> String {
    let resumed = super::wire_json::read(answers)
        .map_err(|problem| format!("the answers are not JSON: {problem}"))
        .and_then(|raw| {
            Resumed::deserialize(raw).map_err(|error| format!("the answers do not decode: {error}"))
        });
    match resumed {
        Ok(Resumed { answers, aborted }) => {
            let replies = answers
                .into_iter()
                .map(|given| {
                    let reply = given
                        .throws
                        .map_or(Reply::Result(given.result), Reply::Throws);
                    (given.id, reply)
                })
                .collect();
            resume(number, replies, &aborted).to_json_text()
        }
        Err(why) => {
            task::cancel(number);
            Step::Done(Answer::Fault(why)).to_json_text()
        }
    }
}

/// A need as the host reads it: `{"id", "port", "method", "args"}`.
fn need_text(need: &Need) -> String {
    let mut out = format!(r#"{{"id":{},"port":{},"method":"#, need.id, need.port);
    write_json_string(&need.method, &mut out);
    out.push_str(r#","args":"#);
    out.push_str(&JsValue::Array(need.args.clone()).to_json_text());
    out.push('}');
    out
}

impl Step {
    /// The step as the JSON text the host reads: `{"done": answer}` (an [`Answer`]'s text),
    /// `{"task": n, "needs": [{"id", "port", "method", "args"}...]}` or `{"rethrow": need}`.
    #[must_use]
    pub fn to_json_text(&self) -> String {
        match self {
            Self::Done(answer) => format!(r#"{{"done":{}}}"#, answer.to_json_text()),
            Self::Rethrow { need } => format!(r#"{{"rethrow":{need}}}"#),
            Self::Needs { task, needs } => {
                let needs: Vec<String> = needs.iter().map(need_text).collect();
                format!(r#"{{"task":{task},"needs":[{}]}}"#, needs.join(","))
            }
        }
    }
}

#[cfg(test)]
mod tests;
