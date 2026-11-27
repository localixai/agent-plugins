//! Port of `src/hooks/verdict/route.ts`, switched.
//!
//! Where the text of a block goes (hooks.md, "Exit code 2 behavior per event" and the event
//! sections): the route of an event's exit-2 row, applied to every block on it, since a failure
//! "does what exit code 2 does on that event" and a blocking decision "routes the same way".
//!
//! An event the table does not name routes as `discarded` (the TypeScript's table covers the
//! official union, so no caller names another).

use serde::{Deserialize, Serialize};

use super::exit2::{StderrChannel, exit2_rule};
use super::types::{BlockInfo, BlockRoute, BlockVia, Feedback, HookNotice};

#[derive(Clone, Copy)]
struct Route {
    to: BlockRoute,
    /// The docs send the hook's command and stderr to the debug log.
    logged: bool,
}

const fn route_of_channel(channel: StderrChannel) -> Route {
    let (to, logged) = match channel {
        StderrChannel::Claude => (BlockRoute::Claude, false),
        StderrChannel::User => (BlockRoute::User, false),
        StderrChannel::UserAndClaude => (BlockRoute::UserAndClaude, false),
        StderrChannel::Debug => (BlockRoute::None, true),
        StderrChannel::Discarded => (BlockRoute::None, false),
    };
    Route { to, logged }
}

/// Judgment (spec/judgments.md): a `PermissionRequest` failure that denies has the table's
/// "discarded" row for exit 2, which it does not do; the denial's text goes to Claude ("`message`:
/// tells Claude why the permission was denied").
fn route_of(event: &str) -> Route {
    match exit2_rule(event) {
        Some(rule) if rule.failure_denies => route_of_channel(StderrChannel::Claude),
        Some(rule) => route_of_channel(rule.stderr),
        None => route_of_channel(StderrChannel::Discarded),
    }
}

/// The text a block carries, before the route decides whether anyone is shown it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
pub struct BlockText {
    /// The hook's own words.
    pub reason: String,
    /// The line the docs show.
    pub message: String,
}

/// A block on `event`: the text stays with the hook when the docs show it to no one.
#[must_use]
pub fn routed_block(
    event: &str,
    via: BlockVia,
    text: BlockText,
    condition: Option<String>,
) -> BlockInfo {
    let Route { to, logged } = route_of(event);
    if to != BlockRoute::None {
        return BlockInfo {
            via,
            to,
            reason: text.reason,
            message: text.message,
            debug: None,
            condition,
        };
    }
    BlockInfo {
        via,
        to,
        reason: String::new(),
        message: String::new(),
        debug: logged.then_some(text.message),
        condition,
    }
}

/// `Pick<HandlerVerdict, 'notice' | 'feedback'>`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Shown {
    /// The notice the user sees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<HookNotice>,
    /// The feedback Claude gets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<Feedback>,
}

/// "You see it as a warning in the transcript, and it stays in the conversation, so Claude sees it
/// when the conversation continues": a block routed to both also leaves a notice and feedback.
#[must_use]
pub fn shown_with(handler_id: &str, block: &BlockInfo) -> Shown {
    if block.to != BlockRoute::UserAndClaude {
        return Shown::default();
    }
    Shown {
        notice: Some(HookNotice {
            handler_id: handler_id.to_owned(),
            message: block.message.clone(),
        }),
        feedback: Some(Feedback {
            handler_id: handler_id.to_owned(),
            to: "claude".to_owned(),
            text: block.message.clone(),
        }),
    }
}

crate::registry::exports! {
    "hooks/verdict/route" {
        "routedBlock" => ok |event: String, via: BlockVia, text: BlockText, condition: Option<String>|
            routed_block(&event, via, text, condition);
        "shownWith" => ok |handler_id: String, block: BlockInfo| shown_with(&handler_id, &block);
    }
}
