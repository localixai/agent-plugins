//! Port of `src/hooks/select/if-guard.ts`, switched.
//!
//! A handler's `if` against the tool call. The evaluator is a function: the kit's own
//! (`ifEvaluatorFor`, a function of the core's over a match context) or one the host gives, which
//! the core calls back.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::hooks::contracts::{MonitorTreatment, ToolCallView};
use crate::js::{JsValue, Pattern, SPACE_CLASS, regex_test};
use crate::registry::{Callback, Function, Port};
use crate::rules::bash::if_eval::{IfResult, evaluate_if};
use crate::rules::types::MatchContext;

/// The evaluator of permission rule syntax over a match context the host (or the engine) built.
struct IfEvaluator(MatchContext);

crate::registry::methods! {
    IfEvaluator {
        "call" => fallible |this, rule: String, tool_name: String, tool_input: JsValue|
            evaluate_if(&rule, &tool_name, &tool_input, &this.0);
    }
}

/// `ifEvaluatorFor`: evaluates one `if` rule against a tool call, with the context rules need.
#[must_use]
pub fn if_evaluator_for(context: MatchContext) -> Function {
    Function::new(IfEvaluator(context))
}

/// Whether the handler runs, with what the kit found wrong in its `if`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct GuardResult {
    /// Whether the handler runs.
    pub runs: bool,
    /// What the kit found wrong.
    pub issues: Vec<PluginIssue>,
}

/// What the `if` is read against: the tool call (absent on a non-tool event), how a Monitor call
/// got here, the rule reader.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct GuardSubject {
    /// The tool call.
    #[serde(default)]
    pub call: Option<ToolCallView>,
    /// How a Monitor call got here.
    #[serde(default)]
    pub monitor: Option<MonitorTreatment>,
    /// The rule reader.
    #[serde(default)]
    pub evaluate: Option<Callback>,
}

/// A subject as the host passes it: data, or the host's object when it holds the evaluator.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum SubjectArg {
    /// The host's object, its members read by name.
    Host(Port),
    /// Plain data.
    Data(GuardSubject),
}

impl SubjectArg {
    async fn read(self) -> GuardSubject {
        match self {
            Self::Data(subject) => subject,
            Self::Host(port) => GuardSubject {
                call: port.member("call").await,
                monitor: port.member("monitor").await,
                evaluate: port.member("evaluate").await,
            },
        }
    }
}

const RUNS: GuardResult = GuardResult {
    runs: true,
    issues: Vec::new(),
};

/// A `)` followed by `&&` or `||` and another rule: the shape of two rules joined by an operator.
static JOINED_RULES: Pattern = LazyLock::new(|| {
    Regex::new(&format!(
        r"\)[{SPACE_CLASS}]*(?:&&|\|\|)[{SPACE_CLASS}]*[A-Za-z_*]"
    ))
    .ok()
});

/// hooks.md, "Common fields": "The `if` field holds exactly one permission rule. There is no `&&`,
/// `||`, or list syntax for combining rules". Parentheses inside a rule's content are literal, so
/// `Bash(rm *) && Bash(git *)` reads as one `Bash` rule that matches almost nothing. Judgment
/// (spec/judgments.md): the rule is still read as one, and a warning says why it never fires.
fn joined_rules_issues(rule: &str, path: &str) -> Vec<PluginIssue> {
    if regex_test(&JOINED_RULES, rule) {
        vec![warning(
            &format!("{path}.if"),
            "The `if` field holds exactly one permission rule; `&&` and `||` do not combine rules, so this text is read as a single rule. Use a separate hook handler for each rule",
        )]
    } else {
        Vec::new()
    }
}

/// hooks.md, "Common fields": `if` is "only evaluated on tool events ... On other events, a hook
/// with `if` set never runs". On a tool event the rule is read as permission rule syntax; a call
/// the rule cannot be decided for runs the hook ("it runs your hook regardless of the pattern"). A
/// Monitor call that reached the hook through a `Bash` matcher is a Bash call to `if`: "In `if`,
/// name Bash ... The hook doesn't run if its `if` names Monitor". `rule` is the handler's `if`.
pub async fn guard_with_if(rule: Option<&str>, subject: &GuardSubject, path: &str) -> GuardResult {
    let Some(rule) = rule else {
        return RUNS;
    };
    let Some(call) = &subject.call else {
        return GuardResult {
            runs: false,
            issues: Vec::new(),
        };
    };
    let Some(evaluate) = subject.evaluate else {
        return GuardResult {
            runs: true,
            issues: vec![warning(
                &format!("{path}.if"),
                "No rule evaluator was given, so the `if` filter does not apply and the hook runs for every call",
            )],
        };
    };
    let tool_name = if subject.monitor.is_some_and(|monitor| monitor.as_bash) {
        JsValue::from("Bash")
    } else {
        call.tool_name.clone()
    };
    let args = vec![rule.into(), tool_name, call.tool_input.clone()];
    let result: IfResult = evaluate.call(args).await;
    let mut issues = joined_rules_issues(rule, path);
    issues.extend(result.issues.into_iter().map(|issue| PluginIssue {
        path: format!("{path}.if"),
        ..issue
    }));
    GuardResult {
        runs: result.runs,
        issues,
    }
}

/// The handler's `if`, read from a registered entry.
#[must_use]
pub fn if_of(entry: &JsValue) -> Option<&str> {
    entry
        .get("handler")
        .and_then(|handler| handler.get("if"))
        .and_then(JsValue::as_str)
}

crate::registry::exports! {
    "hooks/select/if-guard" {
        "ifEvaluatorFor" => ok |context: MatchContext| if_evaluator_for(context);
        "guardWithIf" => async ok |entry: JsValue, subject: SubjectArg, path: String|
            guard_with_if(if_of(&entry), &subject.read().await, &path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joined_rules_read_javascript_blanks() {
        assert_eq!(
            joined_rules_issues("Bash(rm *)\u{A0}&& Bash(git *)", "p").len(),
            1
        );
        assert!(joined_rules_issues("Bash(echo (a))", "p").is_empty());
    }
}
