//! Port of `src/convert/skill-body.ts`, switched.
//!
//! What Claude Code expands in a skill's body and a core client shows as written (Agent Plugins
//! Extended §18.3.2 item 3, §18.4.2 item 2, §18.4.3 item 2): argument placeholders (`$ARGUMENTS`,
//! `$ARGUMENTS[N]`, `$N`, skills.md "Available string substitutions"), the `${CLAUDE_*}` and
//! `${user_config.*}` variables, and injected commands (`` !`cmd` `` and ```` ```! ````, "Inject
//! dynamic context"). A placeholder after a single backslash is escaped and stays text, as
//! `skills/substitute.ts` reads it; `$name` expands only for names a Claude Code `arguments` field
//! declares, which no core skill has.

use crate::format::frontmatter::parse_frontmatter;
use crate::js::regexp::{RegExp, RegExpMatch};
use crate::js::trim;
use crate::skills::shell_scan::find_injections;

const ARGUMENT: &str = r"\\?\$(?:ARGUMENTS\[\d+\]|ARGUMENTS(?![A-Za-z0-9_])|\d+)";
const VARIABLE: &str = r"\$\{(?:CLAUDE_(?:SKILL_DIR|SESSION_ID|EFFORT|PLUGIN_ROOT|PLUGIN_DATA|PROJECT_DIR)|user_config\.[^}]+)\}";

/// `[...body.matchAll(/pattern/g)]`.
fn match_all(pattern: &str, body: &str) -> Vec<RegExpMatch> {
    RegExp::new(pattern, "g")
        .ok()
        .and_then(|regexp| regexp.match_all(body).ok())
        .unwrap_or_default()
}

/// `found[0]`.
fn text_of(found: &RegExpMatch) -> String {
    found
        .captures
        .first()
        .cloned()
        .flatten()
        .unwrap_or_default()
}

/// `body.charAt(index - 1) === '\\'`: the code unit before the match.
fn escaped_by_backslash(body: &str, index: usize) -> bool {
    index
        .checked_sub(1)
        .and_then(|before| body.encode_utf16().nth(before))
        .is_some_and(|unit| unit == u16::from(b'\\'))
}

fn argument_placeholders(body: &str) -> Vec<String> {
    match_all(ARGUMENT, body)
        .iter()
        .filter(|found| {
            !text_of(found).starts_with('\\') || escaped_by_backslash(body, found.index)
        })
        .map(|found| {
            let text = text_of(found);
            text.strip_prefix('\\').unwrap_or(&text).to_owned()
        })
        .collect()
}

fn unique(values: Vec<String>) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for value in values {
        if !kept.contains(&value) {
            kept.push(value);
        }
    }
    kept
}

/// Each kind of expanded text the body holds, as a phrase that names it, in a fixed order.
#[must_use]
pub fn expanded_features(text: &str) -> Vec<String> {
    let body = parse_frontmatter(text).body;
    let placeholders = unique(argument_placeholders(&body));
    let variables = unique(match_all(VARIABLE, &body).iter().map(text_of).collect());
    let injections: Vec<String> = find_injections(&body)
        .iter()
        .map(|found| trim(&found.command).to_owned())
        .collect();
    let mut features = Vec::new();
    if !placeholders.is_empty() {
        features.push(format!("argument placeholders {}", placeholders.join(", ")));
    }
    if !variables.is_empty() {
        features.push(format!("variables {}", variables.join(", ")));
    }
    if !injections.is_empty() {
        features.push(format!("injected commands ({})", injections.join("; ")));
    }
    features
}

crate::registry::exports! {
    "convert/skill-body" {
        "expandedFeatures" => ok |text: String| expanded_features(&text);
    }
}
