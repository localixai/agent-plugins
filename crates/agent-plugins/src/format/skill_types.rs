//! Port of `src/format/skill-types.ts`, switched.
//!
//! The model of a skill and of a command (skills.md, "Configure skills"; plugins/components.md,
//! "Skills" and "Commands"). A command file is "the older format" and "supports the same
//! frontmatter" as a skill (skills.md, "Choose where skills load"), so both carry one
//! `InvocableFrontmatter`. The core holds the two constants and the `BodyReader`; the types stay in
//! the TypeScript.

use std::sync::Arc;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::ports::{BoxFuture, Signal};
use crate::registry::{Callback, Function, Own};

/// The shells `shell` accepts (skills.md, frontmatter reference: "`bash` (default) or `powershell`").
pub const SKILL_SHELLS: &[&str] = &["bash", "powershell"];

/// What differs per origin; a registry typed over the union (K1), read by the expansion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "one flag per documented behaviour, as the TypeScript interface has"
)]
pub struct OriginTraits {
    /// `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`: "Substituted only in plugin skills".
    pub substitutes_plugin_variables: bool,
    /// `${CLAUDE_PROJECT_DIR}` and `${CLAUDE_SESSION_ID}`. A synced skill is not substituted these
    /// in a session on the owner's machine: "the two placeholders reach Claude as literal text"
    /// (skills.md, "How Claude Code handles the body of a synced skill"). An isolated session (cloud,
    /// Cowork) is the host's decision, as for `runs_shell`.
    pub substitutes_session_variables: bool,
    /// Whether `` !`command` `` blocks run. A skill synced from claude.ai "doesn't run `!` commands"
    /// in a session on the owner's machine (skills.md, "How Claude Code handles the body of a synced
    /// skill"); a cloud or Cowork session is the host's decision, not an origin's.
    pub runs_shell: bool,
    /// Whether `skillOverrides` applies: "Plugin skills are not affected by `skillOverrides`. Manage
    /// those through `/plugin` instead" (skills.md, "Override skill visibility from settings").
    pub honours_skill_overrides: bool,
    /// Whether the files the expanded text names with `@` are attached. A synced skill's body, in a
    /// session on the owner's machine, "doesn't attach the files that `@` references name the way it
    /// does for a local skill" (skills.md, "How Claude Code handles the body of a synced skill"); an
    /// isolated session (cloud, Cowork) keeps the local behaviour, as for `runs_shell`.
    pub attaches_file_references: bool,
}

/// The shell of a `!` command (`shell` of the frontmatter).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillShell {
    /// Bash.
    Bash,
    /// PowerShell.
    Powershell,
}

/// What each origin may do with a skill's text, by the origin's kind.
#[must_use]
pub fn origin_traits() -> IndexMap<&'static str, OriginTraits> {
    IndexMap::from([
        (
            "plugin",
            OriginTraits {
                substitutes_plugin_variables: true,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: false,
                attaches_file_references: true,
            },
        ),
        (
            "managed",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "user",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "project",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
        (
            "synced",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: false,
                runs_shell: false,
                honours_skill_overrides: true,
                attaches_file_references: false,
            },
        ),
        // "set one skill to `"off"` in `skillOverrides` to hide it" (skills.md, "Remove a skill").
        (
            "bundled",
            OriginTraits {
                substitutes_plugin_variables: false,
                substitutes_session_variables: true,
                runs_shell: true,
                honours_skill_overrides: true,
                attaches_file_references: true,
            },
        ),
    ])
}

/// Reads the body of a skill or command the catalog holds without it (`BodyReader`). The read
/// happens when it is called, through the host's file port, and nothing is memoized: a body edited
/// since the scan is what the call returns (skills.md, "Edit a skill during a session"). It is
/// `None` when the file is gone or no longer loads as a skill, and it rejects with the port's own
/// error when the host's port refuses the read.
///
/// The TypeScript's function is one of two here: the host's own (a `HostSkill`'s, a test's), a
/// [`Callback`]; or the core's (a scan's, a session's), which crosses as an async function of the
/// core's and, given back, is the core's own object again.
#[derive(Clone)]
pub enum BodyReader {
    /// The host's function.
    Host(Callback),
    /// The core's reader, one object however often it crosses.
    Core(Arc<CoreBody>),
}

/// A reader of the core's: what calling a core [`BodyReader`] does.
pub trait ReadBody: Send + Sync + 'static {
    /// The body as it is now.
    fn read_body<'a>(&'a self, signal: Option<&'a Signal>) -> BoxFuture<'a, Option<String>>;
}

/// A body known already (a session's own entry): the reader answers it.
#[derive(Debug)]
pub struct KnownBody(pub String);

impl ReadBody for KnownBody {
    fn read_body<'a>(&'a self, _signal: Option<&'a Signal>) -> BoxFuture<'a, Option<String>> {
        Box::pin(std::future::ready(Some(self.0.clone())))
    }
}

impl BodyReader {
    /// A reader of the core's.
    pub fn core(reader: impl ReadBody) -> Self {
        Self::Core(Arc::new(CoreBody(Box::new(reader))))
    }

    /// The body as the reader gives it now.
    pub async fn read(&self, signal: Option<&Signal>) -> Option<String> {
        match self {
            Self::Host(callback) => {
                let args = signal
                    .and_then(|signal| crate::js::value::to_js_value(signal).ok())
                    .map_or_else(Vec::new, |signal| vec![signal]);
                callback.call(args).await
            }
            Self::Core(reader) => reader.0.read_body(signal).await,
        }
    }
}

impl std::fmt::Debug for CoreBody {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("CoreBody")
    }
}

impl std::fmt::Debug for BodyReader {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Host(callback) => formatter.debug_tuple("Host").field(callback).finish(),
            Self::Core(_) => formatter.write_str("Core"),
        }
    }
}

/// A core reader as the host holds it: a function whose async `call` reads.
pub struct CoreBody(Box<dyn ReadBody>);

crate::registry::methods! {
    CoreBody {
        "call" => async ok |this, signal: Option<Signal>| this.0.read_body(signal.as_ref()).await;
    }
}

impl Serialize for BodyReader {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Host(callback) => callback.serialize(serializer),
            Self::Core(reader) => Function::new_async(Arc::clone(reader)).serialize(serializer),
        }
    }
}

/// A reader as it crosses back: the core's own, or the host's function.
#[derive(Deserialize)]
#[serde(untagged)]
enum Crossed {
    Core(Own<CoreBody>),
    Host(Callback),
}

impl<'de> Deserialize<'de> for BodyReader {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Crossed::deserialize(deserializer)? {
            Crossed::Core(own) => Self::Core(own.0),
            Crossed::Host(callback) => Self::Host(callback),
        })
    }
}

crate::registry::exports! {
    "format/skill-types" {
        "SKILL_SHELLS" => ok || SKILL_SHELLS;
        "ORIGIN_TRAITS" => ok origin_traits;
    }
}
