//! Port of `src/format/validate/plugin-name.ts`, switched.
//!
//! The checks of `claude plugin validate` on a manifest's `name` (oracle 2.1.319): an empty name, a
//! space, a control or bidirectional-formatting character are errors and the only report; any
//! other name gets the reserved-name and kebab-case advice, which comes after the schema passes.

use serde::{Deserialize, Serialize};

use super::finding::{Finding, error_at};
use crate::format::names::{NameContext, contains_control_or_bidi, name_validation_issues};

const KEBAB_EXAMPLE: &str = "\"my-plugin\"";

/// The errors that stop the advice, as the oracle words them.
fn blocking_message(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("Plugin name cannot be empty".to_owned());
    }
    if name.contains(' ') {
        return Some(format!(
            "Plugin name cannot contain spaces. Use kebab-case (e.g., {KEBAB_EXAMPLE})"
        ));
    }
    contains_control_or_bidi(name).then(|| {
        "Plugin name cannot contain control or bidirectional-formatting characters".to_owned()
    })
}

/// What changes the checks of a name.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct NameOptions {
    /// An official marketplace lists this plugin: its name is not checked against the reserved ones.
    #[serde(default)]
    pub official: Option<bool>,
    /// Where the manifest is read: a marketplace entry's gets the shorter advice. Default the plugin's own file.
    #[serde(default)]
    pub context: Option<NameContext>,
}

/// What a name is reported for.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct NameFindings {
    /// The errors that stop the advice.
    pub blocking: Vec<Finding>,
    /// The advice that follows when none does.
    pub advice: Vec<Finding>,
}

/// `reported`: the marketplace entry carries this name and has already said what it thinks of it.
#[must_use]
pub fn name_findings(name: &str, options: NameOptions, reported: bool) -> NameFindings {
    if let Some(message) = blocking_message(name) {
        return NameFindings {
            blocking: vec![error_at("name", &message)],
            advice: Vec::new(),
        };
    }
    let skip_reserved = options.official == Some(true) || reported;
    let advice = name_validation_issues(name, skip_reserved, options.context)
        .into_iter()
        .map(|issue| Finding {
            severity: issue.severity,
            path: "name".to_owned(),
            message: issue.message,
        })
        .collect();
    NameFindings {
        blocking: Vec::new(),
        advice,
    }
}

crate::registry::exports! {
    "format/validate/plugin-name" {
        "nameFindings" => ok |name: String, options: NameOptions, reported: bool| name_findings(&name, options, reported);
    }
}
