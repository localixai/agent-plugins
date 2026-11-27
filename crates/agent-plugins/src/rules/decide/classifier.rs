//! Port of `src/rules/decide/classifier.ts`, switched.
//!
//! The host's reviewer for auto mode (permission-modes.md, "Eliminate permission prompts with auto
//! mode"): "a second model, the classifier, reviews actions instead of you". The kit holds no model
//! and no classifier. `decide` returns `classify` for an action that goes to the reviewer; the host
//! (or `settleClassification`) hands it here.
//!
//! It names `PreparedCall`, a layer-3 type, so it lives in the rules module rather than in
//! `src/ports/` (docs/ARCHITECTURE.md, "Ports").

use serde::Deserialize;

use super::context::PermissionMode;
use super::unanswered::{PromptFacts, settle_prompt};
use crate::js::JsValue;
use crate::js::value::to_js_value;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;
use crate::rules::types::{Decision, DecisionReason, PermissionBehavior};

/// The reviewer: `review(call, { mode }, signal)`.
pub trait PermissionClassifier: Send + Sync {
    /// What the reviewer makes of `call` in `mode`.
    fn review<'a>(
        &'a self,
        call: &'a JsValue,
        mode: PermissionMode,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, PermissionBehavior>;
}

/// The host's reviewer, its method `review` asked of it.
impl PermissionClassifier for Port {
    fn review<'a>(
        &'a self,
        call: &'a JsValue,
        mode: PermissionMode,
        signal: Option<&'a Signal>,
    ) -> BoxFuture<'a, PermissionBehavior> {
        let context = JsValue::from([("mode", to_js_value(&mode).unwrap_or_default())]);
        let signal = signal
            .and_then(|signal| to_js_value(signal).ok())
            .unwrap_or_default();
        Box::pin(self.method::<PermissionBehavior>("review", vec![call.clone(), context, signal]))
    }
}

/// How the reviewer's answer reads as a decision: allow runs, deny blocks, ask prompts.
const fn answer(behavior: PermissionBehavior, reason: DecisionReason) -> Decision {
    match behavior {
        PermissionBehavior::Allow => Decision::Allow { reason },
        PermissionBehavior::Deny => Decision::Deny { reason },
        PermissionBehavior::Ask => Decision::Ask { reason },
    }
}

/// What a host hands `settleClassification`: the call, the mode, its reviewer and an abort signal.
#[derive(Deserialize)]
pub struct ClassificationRequest {
    /// The call (a `PreparedCall`), handed to the reviewer as it came.
    pub call: JsValue,
    /// The session's mode.
    pub mode: PermissionMode,
    /// The host's reviewer.
    #[serde(default)]
    pub classifier: Option<Port>,
    /// Its cancellation.
    #[serde(default)]
    pub signal: Option<Signal>,
    /// The session's prompt facts (`DecideContext.cannotPrompt`, `denyInsteadOfPrompting`): a review
    /// that ends in a prompt is denied where nobody may answer it.
    #[serde(default)]
    pub prompts: Option<PromptFacts>,
}

/// The decision once the host's reviewer has answered a `classify`. Any other decision is returned
/// unchanged and the reviewer is not asked. With no reviewer there is no verdict, and no verdict
/// denies the action (permission-modes.md, "When auto mode falls back").
pub async fn settle_classification(
    decision: Decision,
    request: &ClassificationRequest,
) -> Decision {
    let Decision::Classify {
        reason,
        then_prompt,
    } = decision
    else {
        return decision;
    };
    // "Claude Code denies the action rather than run it unreviewed" when there is no verdict.
    let Some(classifier) = &request.classifier else {
        return answer(PermissionBehavior::Deny, reason);
    };
    let verdict = classifier
        .review(&request.call, request.mode, request.signal.as_ref())
        .await;
    let answered = answer(verdict, reason);
    // An approval that "still goes to the prompt" asks; a denial or an `ask` stands.
    let settled = if then_prompt.is_some() && matches!(answered, Decision::Allow { .. }) {
        answer(PermissionBehavior::Ask, reason)
    } else {
        answered
    };
    settle_prompt(settled, request.prompts.unwrap_or_default())
}

crate::registry::exports! {
    "rules/decide/classifier" {
        "settleClassification" => async ok |decision: Decision, request: ClassificationRequest| settle_classification(decision, &request).await;
    }
}
