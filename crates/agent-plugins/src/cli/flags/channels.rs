//! Ported from `src/cli/flags/channels.ts`; only the core uses it now, so it has no wrapper.
//!
//! `--channels` and `--dangerously-load-development-channels` (cli-reference.md, "CLI flags";
//! channels.md, "Enterprise controls"; channels-reference.md, "Test during the research preview").
//! `--channels` takes a "space-separated list of `plugin:<name>@<marketplace>` entries"; the
//! development flag "accepts `plugin:<name>@<marketplace>` and `server:<name>` entries", prompts for
//! confirmation, "so it takes effect in interactive sessions", and with `-p` Claude Code "ignores the
//! flag". The kit builds the entries `channelVerdict` reads; the verdict is that module's. The wording
//! of an entry that is neither form is in spec/judgments.md.

use super::types::{Contribution, FlagContext, SessionChannel, refuse};
use super::values::words_of;
use crate::js::{JsValue, is_whitespace};

/// `/^plugin:([^@\s]+)@(\S+)$/`: the plugin and the marketplace.
fn plugin_entry(raw: &str) -> Option<(&str, &str)> {
    let rest = raw.strip_prefix("plugin:")?;
    let (plugin, marketplace) = rest.split_once('@')?;
    let plain = |text: &str| !text.is_empty() && !text.chars().any(is_whitespace);
    (plain(plugin) && plain(marketplace)).then_some((plugin, marketplace))
}

/// `/^server:(\S+)$/`.
fn server_entry(raw: &str) -> Option<&str> {
    let name = raw.strip_prefix("server:")?;
    (!name.is_empty() && !name.chars().any(is_whitespace)).then_some(name)
}

fn entries_of(values: &[String]) -> Vec<String> {
    values.iter().flat_map(|value| words_of(value)).collect()
}

/// A `ChannelPluginRef`: `{ plugin, marketplace }`.
fn plugin_ref(raw: &str) -> Option<JsValue> {
    plugin_entry(raw).map(|(plugin, marketplace)| {
        JsValue::from([
            ("plugin", plugin.into()),
            ("marketplace", marketplace.into()),
        ])
    })
}

fn server_name(raw: &str) -> Option<String> {
    server_entry(raw).map(str::to_owned)
}

type ChannelOf = fn(&str, bool) -> Option<SessionChannel>;

/// `--channels` entries name plugins only: they face the allowlist.
fn channels_entry(raw: &str, _interactive: bool) -> Option<SessionChannel> {
    plugin_ref(raw).map(|plugin| SessionChannel {
        raw: raw.to_owned(),
        entry: JsValue::from([("origin", "channels".into()), ("plugin", plugin)]),
        server: None,
    })
}

/// A development entry is a plugin or a server; either skips the allowlist for itself only.
fn development_entry(raw: &str, interactive: bool) -> Option<SessionChannel> {
    let plugin = plugin_ref(raw);
    let server = server_name(raw);
    if plugin.is_none() && server.is_none() {
        return None;
    }
    let mut entry = JsValue::from([
        ("origin", "development".into()),
        ("interactive", interactive.into()),
    ]);
    if let (JsValue::Object(members), Some(plugin)) = (&mut entry, plugin) {
        members.insert("plugin".to_owned(), plugin);
    }
    Some(SessionChannel {
        raw: raw.to_owned(),
        entry,
        server,
    })
}

fn channels_from(
    values: &[String],
    build: ChannelOf,
    expected: &str,
    interactive: bool,
) -> Contribution {
    let raws = entries_of(values);
    let built: Vec<(String, Option<SessionChannel>)> = raws
        .into_iter()
        .map(|raw| {
            let channel = build(&raw, interactive);
            (raw, channel)
        })
        .collect();
    let bad: Vec<&String> = built
        .iter()
        .filter(|one| one.1.is_none())
        .map(|one| &one.0)
        .collect();
    if !bad.is_empty() {
        return refuse(
            bad.into_iter()
                .map(|raw| format!("Invalid channel entry \"{raw}\": expected {expected}"))
                .collect(),
        );
    }
    Contribution {
        channels: Some(built.into_iter().filter_map(|one| one.1).collect()),
        ..Contribution::default()
    }
}

/// `--channels`.
#[must_use]
pub fn channels(values: &[String]) -> Contribution {
    channels_from(values, channels_entry, "plugin:<name>@<marketplace>", true)
}

/// `--dangerously-load-development-channels`.
#[must_use]
pub fn development_channels(values: &[String], context: &FlagContext) -> Contribution {
    channels_from(
        values,
        development_entry,
        "plugin:<name>@<marketplace> or server:<name>",
        !context.facts.print,
    )
}
