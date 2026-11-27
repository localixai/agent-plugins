//! Port of `src/hooks/output/messages.ts`, switched.
//!
//! Where a hook's `systemMessage` goes (hooks.md, "JSON output": "Warning message shown to the
//! user"). `DirectoryAdded` is the one event that routes it by source: "`slash_command`: Claude Code
//! delivers the hook's `systemMessage` to Claude as context on the next conversation turn, rather
//! than showing it to you. `register_repo_root`: Claude Code writes `systemMessage` output ... to
//! the debug log only".

use serde::{Deserialize, Serialize};

use super::caps::output_caps;
use crate::hooks::input::fact_read::fact_of;
use crate::js::JsValue;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Route {
    User,
    Claude,
    Debug,
}

fn directory_source(source: &JsValue) -> Option<Route> {
    match source.as_str() {
        Some("slash_command") => Some(Route::Claude),
        Some("register_repo_root") => Some(Route::Debug),
        _ => None,
    }
}

fn route_of(event: &str, facts: &JsValue) -> Option<Route> {
    match event {
        "DirectoryAdded" => {
            Some(directory_source(&fact_of(facts, "source")).unwrap_or(Route::User))
        }
        _ => None,
    }
}

/// The part of a hook's read (`Read`) the routing names.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct MessageRead {
    /// The validated JSON output.
    #[serde(default)]
    pub output: Option<JsValue>,
}

/// Where the messages go.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutedMessages {
    /// Shown to the user, in registry order.
    pub to_user: Vec<String>,
    /// Added to Claude's context on the next turn.
    pub to_claude: Vec<String>,
}

/// The `systemMessage` of each read, routed as the event routes it.
#[must_use]
pub fn route_messages(event: &str, reads: &[MessageRead], facts: &JsValue) -> RoutedMessages {
    let accepts = output_caps().get(event).is_some_and(|cap| {
        cap.accepts
            .members
            .iter()
            .any(|field| field == "systemMessage")
    });
    if !accepts {
        return RoutedMessages::default();
    }
    let messages: Vec<String> = reads
        .iter()
        .filter_map(|read| read.output.as_ref()?.get("systemMessage")?.as_str())
        .filter(|message| !message.is_empty())
        .map(str::to_owned)
        .collect();
    let route = route_of(event, facts).unwrap_or(Route::User);
    RoutedMessages {
        to_user: if route == Route::User {
            messages.clone()
        } else {
            vec![]
        },
        to_claude: if route == Route::Claude {
            messages
        } else {
            vec![]
        },
    }
}

crate::registry::exports! {
    "hooks/output/messages" {
        "routeMessages" => ok |event: String, reads: Vec<MessageRead>, facts: JsValue|
            route_messages(&event, &reads, &facts);
    }
}
