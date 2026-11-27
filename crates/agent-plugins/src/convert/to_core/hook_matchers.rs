//! Ported from `src/convert/to-core/hook-matchers.ts`; only the core uses it now, so it has no wrapper.
//!
//! Which Claude Code matchers convert, per core event (Agent Plugins Extended §18.4.3 item 4): on the
//! tool events a matcher that selects one profile's tools, or everything; on `SessionStart` the new
//! sessions the core fires on (`startup|clear|fork`, J18.6) or everything, which then runs on new
//! sessions only; `UserPromptSubmit` and `Stop` ignore a matcher in Claude Code (hooks.md, "Matcher
//! patterns"), so any converts.

use serde::Serialize;

use crate::convert::profiles::{ProfileName, matches_everything, profile_of_matcher};
use crate::format::core::hooks::CoreHookEvent;

/// What a matcher reads as: its profile or narrowing, or why it does not convert.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct MatcherReading {
    /// The profile whose tools the matcher selects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<ProfileName>,
    /// What the conversion narrows.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub narrowed: Option<String>,
    /// Why the matcher does not convert.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

const NEW_SESSIONS: [&str; 3] = ["startup", "clear", "fork"];

fn tool_matcher(matcher: Option<&str>) -> MatcherReading {
    if matches_everything(matcher) {
        return MatcherReading::default();
    }
    match profile_of_matcher(matcher.unwrap_or_default()) {
        None => MatcherReading {
            problem: Some(format!(
                "the matcher \"{}\" selects no single tool profile",
                matcher.unwrap_or("undefined")
            )),
            ..MatcherReading::default()
        },
        Some(profile) => MatcherReading {
            profile: Some(profile),
            ..MatcherReading::default()
        },
    }
}

fn session_matcher(matcher: Option<&str>) -> MatcherReading {
    if matches_everything(matcher) {
        return MatcherReading {
            narrowed: Some(
                "it runs on new sessions only (startup, clear, fork), not on resume or compact"
                    .to_owned(),
            ),
            ..MatcherReading::default()
        };
    }
    let sources: Vec<&str> = matcher.unwrap_or_default().split('|').collect();
    let same = sources.len() == NEW_SESSIONS.len()
        && NEW_SESSIONS.iter().all(|source| sources.contains(source));
    if same {
        MatcherReading::default()
    } else {
        MatcherReading {
            problem: Some(format!(
                "the matcher \"{}\" selects other sessions than the new ones the core fires on",
                matcher.unwrap_or("undefined")
            )),
            ..MatcherReading::default()
        }
    }
}

fn any(_matcher: Option<&str>) -> MatcherReading {
    MatcherReading::default()
}

/// How one core event reads a matcher.
fn matcher_of(event: CoreHookEvent) -> fn(Option<&str>) -> MatcherReading {
    match event {
        CoreHookEvent::SessionStart => session_matcher,
        CoreHookEvent::UserPromptSubmit | CoreHookEvent::Stop => any,
        CoreHookEvent::PreToolUse | CoreHookEvent::PostToolUse => tool_matcher,
    }
}

/// What a Claude Code matcher reads as for a core event.
#[must_use]
pub fn read_matcher(event: CoreHookEvent, matcher: Option<&str>) -> MatcherReading {
    matcher_of(event)(matcher)
}
