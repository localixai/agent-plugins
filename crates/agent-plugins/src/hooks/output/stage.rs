//! Port of `src/hooks/output/stage.ts`, switched.
//!
//! The real output stage the engine takes: parse, cap, merge (hooks.md, "JSON output"). It is a
//! handle (docs/RUST.md, "Handles"): the scratch port it was made with is the host's.

use serde::Deserialize;

use super::limit::{LimitRequest, LimitedOutputs, limit_outputs};
use super::merge::{TaggedOutput, merge_outputs};
use super::parse::{ParseResult, parse_hook_output};
use crate::hooks::verdict::types::MergedOutput;
use crate::js::{JsObject, JsValue};
use crate::ports::Signal;
use crate::registry::{Handle, Port};

/// The options of `createOutputStage`.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct OutputStageOptions {
    /// Where text over the 10,000-character cap is saved; without it such text is cut at the cap.
    #[serde(default)]
    pub scratch: Option<Port>,
}

/// `OutputStage`: what the verdict and the dispatch lean on.
#[derive(Clone, Copy, Debug)]
pub struct OutputStage {
    scratch: Option<Port>,
}

impl OutputStage {
    /// `parse`: `parseHookOutput`.
    ///
    /// # Errors
    ///
    /// What `parseHookOutput` throws.
    pub fn parse(event: &str, json: &JsValue) -> Result<ParseResult, String> {
        parse_hook_output(event, json)
    }

    /// `limit`: `limitOutputs` with the stage's scratch port.
    pub async fn limit(
        &self,
        event: &str,
        outputs: Vec<JsObject>,
        signal: Signal,
    ) -> LimitedOutputs {
        let request = LimitRequest {
            signal,
            scratch: self.scratch,
        };
        limit_outputs(event, outputs, &request).await
    }

    /// `merge`: `mergeOutputs`.
    #[must_use]
    pub fn merge(event: &str, outputs: Vec<TaggedOutput>, facts: &JsValue) -> MergedOutput {
        merge_outputs(event, outputs, facts)
    }
}

/// The real output stage the engine takes: parse, cap, merge (hooks.md, "JSON output").
#[must_use]
pub const fn create_output_stage(options: OutputStageOptions) -> OutputStage {
    OutputStage {
        scratch: options.scratch,
    }
}

crate::registry::methods! {
    OutputStage {
        "parse" => fallible |_this, event: String, json: JsValue| OutputStage::parse(&event, &json);
        "limit" => async ok |this, event: String, outputs: Vec<JsObject>, signal: Signal|
            this.limit(&event, outputs, signal).await;
        "merge" => ok |_this, event: String, outputs: Vec<TaggedOutput>, facts: JsValue| {
            let facts = if facts.is_undefined() { JsValue::Object(JsObject::new()) } else { facts };
            OutputStage::merge(&event, outputs, &facts)
        };
    }
}

crate::registry::exports! {
    "hooks/output/stage" {
        "createOutputStage" => ok |options: Option<OutputStageOptions>|
            Handle::new(create_output_stage(options.unwrap_or_default()));
    }
}
