//! Port of `src/settings/mcp/command-match.ts`, switched.
//!
//! A `serverCommand` entry against a server (managed-mcp.md, "How `serverCommand` entries match"):
//! "Every argument, in order", so `["npx", "-y", "server"]` matches neither `["npx", "server"]` nor
//! `["npx", "-y", "server", "--flag"]`. The `env` block is not compared: the caller passes the command
//! and arguments only. Both sides are `${VAR}`-expanded before they are compared ("Environment
//! variables in `serverCommand` and `serverUrl` entries"); `expandEntry` is the entry side, and the
//! server's command arrives expanded from the live environment.

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::Callback;

/// The entry-side `${VAR}` expansion, a function of the host's: its answer may wait, and a Rust
/// host passes a closure, answered at once.
pub trait ExpandEntry: Send + Sync {
    /// The text with its references expanded.
    fn expand<'a>(&'a self, text: &'a str) -> BoxFuture<'a, String>;
}

impl<F: Fn(&str) -> String + Send + Sync> ExpandEntry for F {
    fn expand<'a>(&'a self, text: &'a str) -> BoxFuture<'a, String> {
        Box::pin(std::future::ready(self(text)))
    }
}

impl ExpandEntry for Callback {
    fn expand<'a>(&'a self, text: &'a str) -> BoxFuture<'a, String> {
        Box::pin(self.call(vec![JsValue::from(text)]))
    }
}

/// Whether the entry's arguments, each expanded, are the server's command and arguments.
pub async fn command_matches(
    entry: &[String],
    server: &[String],
    expand_entry: &dyn ExpandEntry,
) -> bool {
    let mut wanted = Vec::with_capacity(entry.len());
    for part in entry {
        wanted.push(expand_entry.expand(part).await);
    }
    wanted.len() == server.len() && wanted.iter().zip(server).all(|(part, other)| part == other)
}

crate::registry::exports! {
    "settings/mcp/command-match" {
        "commandMatches" => async ok |entry: Vec<String>, server: Vec<String>, expand_entry: Callback|
            command_matches(&entry, &server, &expand_entry).await;
    }
}
