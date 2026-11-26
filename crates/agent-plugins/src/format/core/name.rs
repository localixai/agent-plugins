//! Port of `src/format/core/name.ts`, switched.
//!
//! A core package's `name` (core §5.5): 1–64 characters of `a-z`, `0-9`, `-` and `.`, alphanumeric at
//! both ends, no `--` and no `..`. The checks are the schema's (`schemas/1.1.0/plugin.schema.json`,
//! `name`), each with its own reason so a report says which constraint failed (core §5.3).

use crate::js::code_unit_length;

/// One constraint of the name: when it fails, and what it says then.
struct Rule {
    fails: fn(&str) -> bool,
    reason: &'static str,
}

const RULES: [Rule; 5] = [
    Rule {
        fails: |name| !(1..=64).contains(&code_unit_length(name)),
        reason: "must be 1 to 64 characters long",
    },
    Rule {
        // `/[^a-z0-9.-]/.test(name)`
        fails: |name| {
            name.chars()
                .any(|character| !matches!(character, 'a'..='z' | '0'..='9' | '.' | '-'))
        },
        reason: "may hold only lowercase letters, digits, \"-\" and \".\"",
    },
    Rule {
        // `/^[.-]|[.-]$/.test(name)`
        fails: |name| name.starts_with(['.', '-']) || name.ends_with(['.', '-']),
        reason: "must start and end with a letter or a digit",
    },
    Rule {
        fails: |name| name.contains("--"),
        reason: "must not hold \"--\"",
    },
    Rule {
        fails: |name| name.contains(".."),
        reason: "must not hold \"..\"",
    },
];

/// Why a name breaks core §5.5, each failed constraint once; empty when it is valid.
#[must_use]
pub fn core_name_problems(name: &str) -> Vec<String> {
    RULES
        .iter()
        .filter(|rule| (rule.fails)(name))
        .map(|rule| rule.reason.to_owned())
        .collect()
}

crate::registry::exports! {
    "format/core/name" {
        "coreNameProblems" => ok |name: String| core_name_problems(&name);
    }
}
