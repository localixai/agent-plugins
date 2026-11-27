//! Ported from `src/hooks/select/matcher.ts`; only the core uses it now, so it has no wrapper.

use crate::hooks::contracts::MonitorTreatment;
use crate::js::JsValue;
use crate::registry::Port;
use crate::rules::hook_matcher::matches_matcher;

use super::if_guard::if_of;
use super::matcher_targets::{MatcherValue, call_of, mode_of, value_of};
use super::mcp_tools::{MonitorHook, monitor_treatment};

/// Whether a handler's matcher selects an occurrence of the event, and how a Monitor call got
/// there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatcherResult {
    /// Whether the matcher selects it.
    pub selects: bool,
    /// How a Monitor call got there.
    pub monitor: Option<MonitorTreatment>,
}

/// hooks.md, "Matcher patterns": the matcher is tested against the field the event filters on, in
/// the grammar the event accepts. An event without matcher support ignores it, and a model switch
/// with no canonical name runs every hook ("regardless of matcher").
pub async fn matcher_selects(
    event: &str,
    facts: &JsValue,
    entry: &JsValue,
    model_catalog: Option<&Port>,
) -> MatcherResult {
    let MatcherValue::Filters(value) = value_of(event, facts, model_catalog).await else {
        return MatcherResult {
            selects: true,
            monitor: None,
        };
    };
    let matcher = entry.get("matcher").and_then(JsValue::as_str);
    let direct = matches_matcher(matcher, value.as_deref(), mode_of(event));
    let call = call_of(event, facts);
    let hook = MonitorHook {
        matcher: matcher.map(str::to_owned),
        if_rule: if_of(entry).map(str::to_owned),
    };
    let monitor = monitor_treatment(event, call.as_ref(), &hook, direct);
    match monitor {
        None => MatcherResult {
            selects: direct,
            monitor: None,
        },
        Some(monitor) => MatcherResult {
            selects: true,
            monitor: Some(monitor),
        },
    }
}
