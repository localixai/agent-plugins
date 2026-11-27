//! Ported from `src/cli/install-refusals.ts`; only the core uses it now, so it has no wrapper.
//!
//! The refusals of `claude plugin install` before anything is written: bad `--config` values, a dependency closure that cannot be met.

use super::install_config::ConfigOutcome;
use super::result::{CommandOutput, Extras, command_failed};
use crate::js::{JsObject, JsValue};

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
