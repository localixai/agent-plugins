//! Port of `src/format/user-config-options.ts`.
//!
//! The `options` rules of a `userConfig` option (plugins/manifest-reference.md, "Limit a field to
//! fixed options": "`options` applies to a `string` field that isn't `multiple` or `sensitive`",
//! each value "a plain label of 1 to 64 characters", "Set `default` to one of the listed values, or
//! set `required: true`"; a plugin that breaks them "fails to load"). The messages, paths, order and
//! the rules the docs leave to "anything else it rejects" are the oracle's (2.1.325, spec §3.4.2 and
//! J3.10): length in code points; no edge space; no control, invisible or bidirectional-formatting
//! character; no repeat in any letter case; at least one value.

use std::collections::HashSet;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::code_points::code_point_length;
use crate::js::{JsValue, Pattern, regex_test, trim};
use crate::zod::{self, Schema, issue};

const MAX_CODE_POINTS: usize = 64;
/// Controls (`Cc`) and format characters (`Cf`: U+200B and the other invisible and bidi ones).
static HIDDEN: Pattern = LazyLock::new(|| Regex::new(r"[\p{Cc}\p{Cf}]").ok());

/// The first rule one value breaks, in the oracle's order; `None` when it keeps them all.
fn value_problem(value: &str) -> Option<&'static str> {
    if value.is_empty() {
        return Some("An option cannot be empty");
    }
    if code_point_length(value) > MAX_CODE_POINTS {
        return Some("An option can be at most 64 characters");
    }
    if trim(value) != value {
        return Some("An option cannot start or end with a space");
    }
    if regex_test(&HIDDEN, value) {
        return Some(
            "An option cannot contain control, invisible or bidirectional-formatting characters",
        );
    }
    None
}

/// One value of `options`: its problem is reported at `options.<i>` (`optionValue`).
pub static OPTION_VALUE: LazyLock<Schema> = LazyLock::new(|| {
    zod::string().check(|context| {
        let Some(problem) = value_problem(context.value().as_str().unwrap_or_default()) else {
            return;
        };
        let input = context.value().clone();
        context.push_issue(
            issue("custom")
                .input(input)
                .message(problem)
                .continues(true),
        );
    })
});

/// What the list-level rules read of an option.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct OptionsHolder {
    /// The option's `type`.
    #[serde(rename = "type")]
    pub kind: String,
    /// Its `options`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    /// Its `default`, any value.
    #[serde(default)]
    pub default: JsValue,
    /// Its `required`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    /// Its `multiple`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multiple: Option<bool>,
    /// Its `sensitive`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sensitive: Option<bool>,
}

/// Where a problem is reported.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProblemPath {
    /// `options`.
    Options,
    /// `default`.
    Default,
}

impl ProblemPath {
    /// The key as the issue's path writes it.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Options => "options",
            Self::Default => "default",
        }
    }
}

/// One list-level problem.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Problem {
    /// The key it is at.
    pub path: ProblemPath,
    /// What is wrong.
    pub message: String,
}

/// The types `options` applies to: "For `string`, the values the field accepts".
const TYPES_WITH_OPTIONS: [&str; 1] = ["string"];

const ONLY_FOR_STRING: &str =
    "options is only for a field of type \"string\" that is neither multiple nor sensitive";

fn repeats(options: &[String]) -> bool {
    options
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<HashSet<String>>()
        .len()
        != options.len()
}

fn problem(path: ProblemPath, message: impl Into<String>) -> Problem {
    Problem {
        path,
        message: message.into(),
    }
}

/// The default rule: one of the values, compared exactly, or `required: true` (J3.11).
fn default_problem(option: &OptionsHolder, options: &[String]) -> Option<Problem> {
    let fallback = &option.default;
    if fallback.is_undefined() {
        return if option.required == Some(true) {
            None
        } else {
            Some(problem(
                ProblemPath::Default,
                "a field with options needs a default among them, or required: true",
            ))
        };
    }
    if fallback
        .as_str()
        .is_some_and(|text| options.iter().any(|value| value == text))
    {
        return None;
    }
    Some(problem(
        ProblemPath::Default,
        format!("default must be one of the options: {}", options.join(", ")),
    ))
}

/// The list-level problems, in the oracle's order. A field that may not have `options` at all
/// reports that alone (oracle 2.1.325: a `multiple` field with a wrong `default` draws one error).
#[must_use]
pub fn options_problems(option: &OptionsHolder) -> Vec<Problem> {
    let Some(options) = &option.options else {
        return Vec::new();
    };
    let allowed = TYPES_WITH_OPTIONS.contains(&option.kind.as_str())
        && option.multiple != Some(true)
        && option.sensitive != Some(true);
    if !allowed {
        return vec![problem(ProblemPath::Options, ONLY_FOR_STRING)];
    }
    let mut problems: Vec<Problem> = Vec::new();
    if options.is_empty() {
        problems.push(problem(
            ProblemPath::Options,
            "options needs at least one value",
        ));
    }
    if repeats(options) {
        problems.push(problem(
            ProblemPath::Options,
            "options cannot repeat a value (in any letter case)",
        ));
    }
    if let Some(fallback) = default_problem(option, options) {
        problems.push(fallback);
    }
    problems
}

crate::registry::exports! {
    "format/user-config-options" {
        "optionsProblems" => ok |option: OptionsHolder| options_problems(&option);
    }
}
