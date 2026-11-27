//! Ported from `src/hooks/verdict/suspect-capture.ts`; only the core uses it now, so it has no wrapper.
//!
//! A hook that starts a background process without redirecting its output (hooks.md, "Hooks that
//! start a background process"): "If your script prints an allow, then starts a background process
//! without redirecting that process's stdout and stderr, Claude Code blocks the tool call. The hook
//! error includes `refusing to honor the suspect capture`." A decision printed after "Claude Code
//! has stopped reading" never counts: the run is what the process printed up to its exit.
//!
//! Judgments (spec/judgments.md): the docs show an allow; any exit but 2 whose capture is held
//! open is refused, since the same late write could be a deny. The refusal blocks where exit 2
//! does (and where a failure denies, `PermissionRequest`), not on the events that keep Claude
//! working (`Stop`, `SubagentStop`, `TaskCompleted`, `TeammateIdle`), where blocking would loop.

use super::debug_lines::Subject;
use super::exit2::exit2_rule;
use super::notices::{handler_label, hook_error, hook_name};
use super::on_failure::{value_of, with_block};
use super::route::{BlockText, routed_block};
use super::types::{BlockVia, HookNotice};
use crate::js::{JsObject, JsValue};

const SUSPECT_CAPTURE: &str = "refusing to honor the suspect capture";

fn blocks_here(subject: &Subject) -> bool {
    exit2_rule(&subject.event).is_some_and(|rule| {
        (rule.can_block || rule.failure_denies)
            && !rule.ignores_on_failure
            && !rule
                .cannot_block_when
                .is_some_and(|cannot| cannot(&subject.facts))
    })
}

/// The verdict of a run whose capture is held open.
#[must_use]
pub fn suspect_capture(subject: &Subject) -> JsObject {
    let message = hook_error(
        &hook_name(&subject.event, subject.tool_name.as_deref()),
        &handler_label(&subject.handler()),
        SUSPECT_CAPTURE,
    );
    let mut verdict = JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("succeeded".to_owned(), JsValue::Bool(false)),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ]);
    if !blocks_here(subject) {
        verdict.insert("status".to_owned(), JsValue::from("non-blocking-error"));
        let notice = HookNotice {
            handler_id: subject.entry_id(),
            message,
        };
        verdict.insert("notice".to_owned(), value_of(&notice));
        return verdict;
    }
    let block = routed_block(
        &subject.event,
        BlockVia::Failure,
        BlockText {
            reason: SUSPECT_CAPTURE.to_owned(),
            message,
        },
        None,
    );
    verdict.insert("status".to_owned(), JsValue::from("blocked"));
    with_block(verdict, &subject.entry_id(), &block)
}
