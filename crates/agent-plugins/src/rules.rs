//! Layer 3: permission rules, read once and matched against tool calls, and the decisions they
//! lead to (`src/rules/` in the npm package).

pub mod bash;
pub mod decide;
pub mod hook_matcher;
pub mod issues;
pub mod matchers;
pub mod memo;
pub mod path;
pub mod restricts;
pub mod rule_warnings;
pub mod sources;
pub mod specifier_formats;
pub mod specifier_guards;
pub mod target;
pub mod tool_pattern;
pub mod tools;
pub mod types;
