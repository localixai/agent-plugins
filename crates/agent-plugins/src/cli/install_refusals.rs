//! Port of `src/cli/install-refusals.ts`, switched.
//!
//! The refusals of `claude plugin install` before anything is written: bad `--config` values, a dependency closure that cannot be met.

use serde::Deserialize;

use super::result::{CommandOutput, Extras, command_failed};
use crate::js::{JsObject, JsValue};

/// A `--config` value that is refused: its key and why (`install-config.ts`).
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ConfigProblem {
    /// The key.
    pub key: String,
    /// Why.
    pub message: String,
}

/// What reading the `--config` values found: only what the refusal reads.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ConfigOutcome {
    /// Failing values: the install is refused.
    pub refused: Vec<ConfigProblem>,
}

/// The refusal of bad `--config` values.
#[must_use]
pub fn config_refusal(config: &ConfigOutcome) -> CommandOutput {
    let mut lines = vec!["Failed to install plugin:".to_owned()];
    lines.extend(
        config
            .refused
            .iter()
            .map(|problem| format!("--config {}: {}", problem.key, problem.message)),
    );
    let mut fields = JsObject::new();
    fields.insert(
        "invalidKeys".to_owned(),
        JsValue::Array(
            config
                .refused
                .iter()
                .map(|problem| JsValue::String(problem.key.clone()))
                .collect(),
        ),
    );
    command_failed(
        "install",
        &lines.join("\n"),
        Extras {
            failure_code: Some("invalid_config".to_owned()),
            fields: Some(fields),
        },
    )
}

/// The refusal of a dependency closure that cannot be met.
#[must_use]
pub fn closure_refusal(id: &str, messages: &[String]) -> CommandOutput {
    let mut lines = vec![format!("Failed to install plugin \"{id}\":")];
    lines.extend(messages.iter().cloned());
    command_failed(
        "install",
        &lines.join("\n"),
        Extras {
            failure_code: Some("dependency_error".to_owned()),
            fields: None,
        },
    )
}

crate::registry::exports! {
    "cli/install-refusals" {
        "configRefusal" => ok |config: ConfigOutcome| config_refusal(&config);
        "closureRefusal" => ok |id: String, messages: Vec<String>| closure_refusal(&id, &messages);
    }
}
