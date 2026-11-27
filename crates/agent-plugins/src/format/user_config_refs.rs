//! Port of `src/format/user-config-refs.ts`, switched.
//!
//! `${user_config.KEY}` references (plugins/manifest-reference.md, "Reference a saved value") and the
//! option keys they name: the one reader every field that substitutes, refuses or checks them uses.
//! A reference is a braced `${...}` whose text starts with `user_config.`; the key is what follows,
//! as written (a bundled server's keys included). Whether the key is a valid option key is
//! `USER_CONFIG_KEY`'s.

use std::sync::LazyLock;

use regex::Regex;

use super::mcp_env::braced_references;
use crate::js::{Pattern, regex_test};
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// The source of `USER_CONFIG_KEY`, for a schema's `.regex`.
pub const USER_CONFIG_KEY_SOURCE: &str = r"^[A-Za-z_][A-Za-z0-9_]*$";

/// `userConfig` keys: letters, digits and underscores, not starting with a digit.
pub static USER_CONFIG_KEY: Pattern = LazyLock::new(|| Regex::new(USER_CONFIG_KEY_SOURCE).ok());

const PREFIX: &str = "user_config.";

/// The key of a `user_config.KEY` reference (the text inside `${...}`), or `None` for any other.
#[must_use]
pub fn user_config_key_in(inner: &str) -> Option<&str> {
    inner.strip_prefix(PREFIX).filter(|key| !key.is_empty())
}

/// The option a reference names when its key is a valid option key (`USER_CONFIG_KEY`), else
/// `None`: `${user_config.2fa}` names no option.
#[must_use]
pub fn valid_option_key_in(inner: &str) -> Option<&str> {
    user_config_key_in(inner).filter(|key| regex_test(&USER_CONFIG_KEY, key))
}

/// The keys `text` references, in order, repeats kept.
#[must_use]
pub fn user_config_references(text: &str) -> Vec<String> {
    braced_references(text)
        .into_iter()
        .filter_map(|(_, inner)| user_config_key_in(inner).map(str::to_owned))
        .collect()
}

/// `replace(key, reference)`: what a reference becomes.
pub trait ReplaceReference: Send + Sync {
    /// The text for the reference `reference` to `key`.
    fn replace<'a>(&'a self, key: &'a str, reference: &'a str) -> BoxFuture<'a, String>;
}

impl<F: Fn(&str, &str) -> String + Send + Sync> ReplaceReference for F {
    fn replace<'a>(&'a self, key: &'a str, reference: &'a str) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(self(key, reference)))
    }
}

impl ReplaceReference for Callback {
    fn replace<'a>(&'a self, key: &'a str, reference: &'a str) -> BoxFuture<'a, String> {
        Box::pin(self.call(vec![key.into(), reference.into()]))
    }
}

/// `text` with each reference replaced by `replace(key, reference)`; other `${...}` stay as written.
pub async fn replace_user_config_references(text: &str, replace: &dyn ReplaceReference) -> String {
    let mut out = String::new();
    let mut last = 0;
    for (range, inner) in braced_references(text) {
        out.push_str(text.get(last..range.start).unwrap_or_default());
        let reference = text.get(range.clone()).unwrap_or_default();
        last = range.end;
        match user_config_key_in(inner) {
            None => out.push_str(reference),
            Some(key) => out.push_str(&replace.replace(key, reference).await),
        }
    }
    out.push_str(text.get(last..).unwrap_or_default());
    out
}

crate::registry::exports! {
    "format/user-config-refs" {
        "USER_CONFIG_KEY" => ok || crate::registry::RegExp { source: USER_CONFIG_KEY_SOURCE.to_owned(), flags: String::new() };
        "userConfigKeyIn" => ok |inner: String| user_config_key_in(&inner).map(str::to_owned);
        "validOptionKeyIn" => ok |inner: String| valid_option_key_in(&inner).map(str::to_owned);
        "userConfigReferences" => ok |text: String| user_config_references(&text);
        "replaceUserConfigReferences" => async ok |text: String, replace: Callback|
            replace_user_config_references(&text, &replace).await;
    }
}
