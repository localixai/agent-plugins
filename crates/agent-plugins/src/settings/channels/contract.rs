//! Port of `src/settings/channels/contract.ts`, switched.
//!
//! The channel contract a host reads from a server and applies to its events
//! (channels-reference.md, "Server options", "Notification format", "Gate inbound messages";
//! channels.md, "Security"). Pure over data the host read from the MCP connection; the kit starts
//! no server.

use std::collections::HashSet;
use std::fmt::Write as _;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::registry::Set;

/// What a server's capabilities declare.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    /// `experimental['claude/channel']`: "Required. Always `{}`. Presence registers the notification
    /// listener."
    pub channel: bool,
    /// `experimental['claude/channel/permission']`: `{}` declares relay; "omit the key or set it to
    /// `false`" opts out.
    pub permission: bool,
    /// `tools`: "Two-way only", the standard MCP tool capability.
    pub tools: bool,
}

/// Reads the `capabilities` object of the server's initialize result.
#[must_use]
pub fn channel_capabilities(capabilities: &JsValue) -> ChannelCapabilities {
    let record = capabilities.as_object();
    let experimental = record
        .and_then(|record| record.get("experimental"))
        .and_then(JsValue::as_object);
    ChannelCapabilities {
        channel: experimental
            .is_some_and(|experimental| experimental.contains_key("claude/channel")),
        permission: experimental
            .and_then(|experimental| experimental.get("claude/channel/permission"))
            .is_some_and(JsValue::is_object),
        tools: record
            .and_then(|record| record.get("tools"))
            .is_some_and(JsValue::is_object),
    }
}

/// "To create a one-way channel, omit `capabilities.tools`": a channel with tools can take
/// replies.
#[must_use]
pub const fn is_two_way(capabilities: &ChannelCapabilities) -> bool {
    capabilities.tools
}

/// `instructions` "Claude Code delivers ... to Claude as context when the server connects": the
/// text to add to the context, or `undefined` when the server gave none.
#[must_use]
pub fn channel_instructions(instructions: &JsValue) -> Option<String> {
    match instructions.as_str() {
        Some(text) if !text.is_empty() => Some(text.to_owned()),
        Some(_) | None => None,
    }
}

/// The `notifications/claude/channel` params.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChannelNotification {
    /// The event body.
    pub content: String,
    /// The attributes of the tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub meta: Option<IndexMap<String, String>>,
}

/// The `notifications/claude/channel` params a server sent, read as a `ChannelNotification`:
/// `content` is a required string ("The event body"), so a notification without one is
/// `undefined`; `meta` keeps the entries whose key is an identifier and whose value is a string.
/// Judgment (spec/judgments.md): the docs type `meta` as `Record<string, string>` and say nothing
/// about a value of another type; it is dropped, as a key that is not an identifier is.
#[must_use]
pub fn read_channel_notification(params: &JsValue) -> Option<ChannelNotification> {
    let content = params.get("content")?.as_str()?;
    let mut kept: IndexMap<String, String> = IndexMap::new();
    for (key, value) in entries_of(params.get("meta")) {
        // `kept[key] = value` of `__proto__` sets the prototype, which a string never does.
        if is_identifier(&key)
            && key != "__proto__"
            && let Some(text) = value.as_str()
        {
            kept.insert(key, text.to_owned());
        }
    }
    Some(ChannelNotification {
        content: content.to_owned(),
        meta: Some(kept),
    })
}

/// `Object.entries` of an object or an array (its indexes as keys); nothing for any other value.
fn entries_of(value: Option<&JsValue>) -> Vec<(String, &JsValue)> {
    match value {
        Some(JsValue::Object(members)) => crate::js::property_order(members)
            .into_iter()
            .map(|(key, member)| (key.clone(), member))
            .collect(),
        Some(JsValue::Array(items)) => items
            .iter()
            .enumerate()
            .map(|(index, item)| (index.to_string(), item))
            .collect(),
        Some(
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_),
        )
        | None => Vec::new(),
    }
}

/// "Keys must be identifiers: letters, digits, and underscores only ... other characters are
/// silently dropped." (`/^[A-Za-z0-9_]+$/`)
fn is_identifier(key: &str) -> bool {
    !key.is_empty()
        && key
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Judgment (spec/judgments.md): the docs show attributes as `key="value"` and do not say how a
/// quote in a value is written; the kit escapes `&`, `"`, `<` and `>` as XML does.
fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// The event as Claude sees it: `<channel source="..." key="value">content</channel>`. `source`
/// "is set automatically from your server's configured name"; each valid `meta` entry is an
/// attribute.
#[must_use]
pub fn channel_tag(source: &str, notification: &ChannelNotification) -> String {
    let mut attributes = String::new();
    for (key, value) in notification.meta.iter().flatten() {
        if is_identifier(key) && key.as_str() != "source" {
            let _ = write!(attributes, " {key}=\"{}\"", escape_attribute(value));
        }
    }
    let head = format!(
        "<channel source=\"{}\"{attributes}>",
        escape_attribute(source)
    );
    format!("{head}\n{}\n</channel>", notification.content)
}

/// Whether an event reaches the session. "If the session hasn't loaded your server as a channel,
/// or the organization policy blocks it, Claude Code drops the events silently and returns no
/// error": the server must be named by a flag (`verdict` is set) and the verdict must admit it.
/// Being in `.mcp.json` alone is not enough (channels.md, "Security").
#[must_use]
pub fn delivers_channel_events(verdict: &JsValue) -> bool {
    verdict.get("allowed").and_then(JsValue::as_bool) == Some(true)
}

/// "Events queue into the session and are processed in order. If several notifications arrive
/// while Claude is busy, they're delivered together on the next turn and Claude handles them as a
/// group."
#[must_use]
pub fn group_channel_events(tags: &[String]) -> Option<String> {
    if tags.is_empty() {
        None
    } else {
        Some(tags.join("\n"))
    }
}

/// A chat platform's account: its id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundParty {
    /// The id.
    pub id: String,
}

/// A message from a chat platform: who sent it and in which room.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundMessage {
    /// The sender.
    pub from: InboundParty,
    /// The room.
    pub chat: InboundParty,
}

/// "Gate on the sender's identity, not the chat or room identity": a message passes when its
/// sender is on the list, whatever room it came from.
#[must_use]
pub fn sender_allowed(allowed: &HashSet<String>, message: &InboundMessage) -> bool {
    allowed.contains(&message.from.id)
}

crate::registry::exports! {
    "settings/channels/contract" {
        "channelCapabilities" => ok |capabilities: JsValue| channel_capabilities(&capabilities);
        "isTwoWay" => ok |capabilities: ChannelCapabilities| is_two_way(&capabilities);
        "channelInstructions" => ok |instructions: JsValue| channel_instructions(&instructions);
        "readChannelNotification" => ok |params: JsValue| read_channel_notification(&params);
        "channelTag" => ok |source: String, notification: ChannelNotification| channel_tag(&source, &notification);
        "deliversChannelEvents" => ok |verdict: JsValue| delivers_channel_events(&verdict);
        "groupChannelEvents" => ok |tags: Vec<String>| group_channel_events(&tags);
        "senderAllowed" => ok |allowed: Set<String>, message: InboundMessage| sender_allowed(&allowed.members.into_iter().collect(), &message);
    }
}
