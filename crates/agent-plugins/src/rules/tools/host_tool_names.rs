//! Port of `src/rules/tools/host-tool-names.ts`, switched.
//!
//! How a host's tool names correspond to Claude Code's, stated once by the host. The docs name
//! only Claude Code's tools, so the kit reads every tool name in those terms: what a subagent's
//! `tools` and `disallowedTools` match, what a hook's `matcher` and `tool_name` see, what a
//! permission rule and a mod's `tool.call` matcher match. Judgment (spec/judgments.md).
//!
//! Across the boundary the mapping is a handle: `claudeName` asks the host's `callName`, which the
//! handle keeps from the call that made it.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::Deserialize;

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::{Callback, Handle, Port};

/// A host tool's Claude Code tool, or tools.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum ToClaude {
    /// One tool.
    One(String),
    /// Several, or none.
    Many(Vec<String>),
}

fn list_of(to: ToClaude) -> Vec<String> {
    match to {
        ToClaude::One(name) => vec![name],
        ToClaude::Many(names) => names,
    }
}

/// The Claude Code name of one call, when it depends on the input (`callName`).
pub type CallName =
    Arc<dyn Fn(String, JsValue) -> BoxFuture<'static, Option<String>> + Send + Sync>;

/// What the host states (`HostToolNames`).
#[derive(Clone, Default)]
pub struct HostToolNames {
    /// A host tool name and the Claude Code tool, or tools, it stands for: a built-in
    /// (`write_file` to `Write`, `task` to `Agent`, so `Agent(type)` reads it), an MCP tool in
    /// `mcp__<server>__<tool>` form, or a list when one host tool does the work of several
    /// (`Search` to `['Grep', 'Glob']`). A name left out, or given an empty list, is the same on
    /// both sides.
    pub to_claude: Option<IndexMap<String, ToClaude>>,
    /// The Claude Code name of one call, when it depends on the input (`Search` with a `pattern`
    /// is `Grep`, with only a glob `Glob`). Read only when it answers one of the names the tool
    /// lists; otherwise, and without it, a call carries the first name listed.
    pub call_name: Option<CallName>,
    /// Claude Code tools the host has no tool for: never granted (the restored `Glob`/`Grep`).
    pub missing: Option<Vec<String>>,
}

/// `HostToolNames` without its function, as data crosses.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostToolNamesData {
    /// `toClaude`.
    #[serde(default)]
    pub to_claude: Option<IndexMap<String, ToClaude>>,
    /// `missing`.
    #[serde(default)]
    pub missing: Option<Vec<String>>,
}

/// `HostToolNames` as it crosses: data, or, when it holds `callName`, a port whose members are
/// read.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum HostToolNamesGiven {
    /// The host's record, holding a function.
    Host(Port),
    /// Plain data.
    Data(HostToolNamesData),
}

impl HostToolNamesGiven {
    /// The names, `callName` asking the host.
    pub async fn read(self) -> HostToolNames {
        match self {
            Self::Data(data) => HostToolNames {
                to_claude: data.to_claude,
                call_name: None,
                missing: data.missing,
            },
            Self::Host(port) => {
                let call_name: Option<Callback> = port.member("callName").await;
                HostToolNames {
                    to_claude: port.member("toClaude").await,
                    call_name: call_name.map(|callback| {
                        let call: CallName = Arc::new(move |host, input| {
                            Box::pin(
                                callback.call::<Option<String>>(vec![JsValue::from(host), input]),
                            )
                        });
                        call
                    }),
                    missing: port.member("missing").await,
                }
            }
        }
    }
}

/// A `HostToolNames` read both ways.
pub struct ToolNames {
    to_claude: IndexMap<String, Vec<String>>,
    missing: Vec<String>,
    call_name: Option<CallName>,
}

impl std::fmt::Debug for ToolNames {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ToolNames")
            .field("to_claude", &self.to_claude)
            .field("missing", &self.missing)
            .finish_non_exhaustive()
    }
}

impl ToolNames {
    /// Every Claude Code name a host tool stands for, in the host's order; the name itself when
    /// unmapped.
    #[must_use]
    pub fn claude_names(&self, host: &str) -> Vec<String> {
        self.to_claude
            .get(host)
            .cloned()
            .unwrap_or_else(|| vec![host.to_owned()])
    }

    fn mapped_to(&self, claude: &str) -> Vec<String> {
        self.to_claude
            .iter()
            .filter(|(_, to)| to.iter().any(|name| name == claude))
            .map(|(host, _)| host.clone())
            .collect()
    }

    /// An unmapped Claude Code name is the host tool of that name, unless the host lacks it or
    /// that host name stands for another Claude Code tool.
    fn same(&self, claude: &str) -> Vec<String> {
        if self.missing.iter().any(|name| name == claude) || self.to_claude.contains_key(claude) {
            Vec::new()
        } else {
            vec![claude.to_owned()]
        }
    }

    /// The host tools that stand for a Claude Code tool; empty when the host has none.
    #[must_use]
    pub fn host_names(&self, claude: &str) -> Vec<String> {
        let mapped = self.mapped_to(claude);
        if mapped.is_empty() {
            self.same(claude)
        } else {
            mapped
        }
    }

    /// The Claude Code name of a call of a host tool: `callName` over `input`, else the first
    /// listed.
    pub async fn claude_name(&self, host: &str, input: JsValue) -> String {
        let listed = self.claude_names(host);
        let answer = match &self.call_name {
            Some(call_name) => call_name(host.to_owned(), input).await,
            None => None,
        };
        match answer {
            Some(answer) if listed.contains(&answer) => answer,
            _ => listed.into_iter().next().unwrap_or_else(|| host.to_owned()),
        }
    }

    /// Whether the host has a tool for this Claude Code name.
    #[must_use]
    pub fn has(&self, claude: &str) -> bool {
        !self.host_names(claude).is_empty()
    }
}

/// The mapping both ways; `None` is the host whose tools are Claude Code's.
#[must_use]
pub fn tool_names_of(names: Option<HostToolNames>) -> ToolNames {
    let names = names.unwrap_or_default();
    let to_claude = names
        .to_claude
        .unwrap_or_default()
        .into_iter()
        .map(|(host, to)| (host, list_of(to)))
        .filter(|(_, to)| !to.is_empty())
        .collect();
    ToolNames {
        to_claude,
        missing: names.missing.unwrap_or_default(),
        call_name: names.call_name,
    }
}

crate::registry::methods! {
    ToolNames {
        "claudeName" => sync async ok |this, host_name: String, input: JsValue| this.claude_name(&host_name, input).await;
        "claudeNames" => ok |this, host_name: String| this.claude_names(&host_name);
        "hostNames" => ok |this, claude_name: String| this.host_names(&claude_name);
        "has" => ok |this, claude_name: String| this.has(&claude_name);
    }
}

async fn tool_names_given(names: Option<HostToolNamesGiven>) -> Handle {
    let names = match names {
        Some(given) => Some(given.read().await),
        None => None,
    };
    Handle::new(tool_names_of(names))
}

crate::registry::exports! {
    "rules/tools/host-tool-names" {
        "toolNamesOf" => async ok |names: Option<HostToolNamesGiven>| tool_names_given(names).await;
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::{HostToolNames, ToClaude, tool_names_of};
    use crate::js::JsValue;
    use crate::ports::now;

    #[test]
    fn a_mapped_name_is_read_both_ways_and_a_missing_one_never_granted() {
        let to_claude: IndexMap<String, ToClaude> = [
            (
                "Search".to_owned(),
                ToClaude::Many(vec!["Grep".to_owned(), "Glob".to_owned()]),
            ),
            ("write_file".to_owned(), ToClaude::One("Write".to_owned())),
        ]
        .into_iter()
        .collect();
        let names = tool_names_of(Some(HostToolNames {
            to_claude: Some(to_claude),
            call_name: None,
            missing: Some(vec!["Read".to_owned()]),
        }));
        assert_eq!(names.host_names("Glob"), vec!["Search"]);
        assert!(!names.has("Read"));
        assert!(names.has("Bash"));
        assert_eq!(
            now(names.claude_name("Search", JsValue::Undefined)).as_deref(),
            Some("Grep")
        );
    }
}
