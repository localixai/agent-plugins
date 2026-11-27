//! Ported from `src/format/agent-hooks-field.ts`; only the core uses it now, so it has no wrapper.
//!
//! `hooks` (sub-agents.md, "Frontmatter reference": "Lifecycle hooks scoped to this subagent"): the
//! settings' hook format, an event map, and "All hook events are supported". What is wrong is
//! reported as a warning and no hook is kept, so a half-read file never runs half its hooks; the
//! agent still loads. A plugin agent never gets here: the registry marks the field ignored.

use super::agent_scalars::AgentFieldRead;
use super::hooks_settings::{HooksReadOptions, HooksSettings, read_hooks_settings};
use super::issues::warning;
use crate::js::JsValue;

/// The `hooks` field of an agent.
#[must_use]
pub fn hooks(path: &str, key: &str, raw: &JsValue) -> AgentFieldRead<HooksSettings> {
    if matches!(raw, JsValue::Undefined | JsValue::Null) {
        return AgentFieldRead {
            value: None,
            issues: Vec::new(),
        };
    }
    let read = read_hooks_settings(raw, HooksReadOptions::default());
    let issues: Vec<_> = read
        .issues
        .iter()
        .map(|issue| {
            warning(
                path,
                &format!(
                    "Invalid \"{key}\": {}; the hooks are ignored",
                    issue.message
                ),
            )
        })
        .collect();
    AgentFieldRead {
        value: issues.is_empty().then_some(read.hooks),
        issues,
    }
}
