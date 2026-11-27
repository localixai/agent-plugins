//! The request, the ports and the skill of an expansion as they cross: `ExpandRequest`,
//! `ExpandSession` and `ExpandPorts` of `src/skills/expand-types.ts`, which holds only types and so
//! has no port of its own, and `Skill | Command` read as each module the expansion calls reads it.
//!
//! A request that holds `commandEnvironment` (a function) and the ports (`permissions` is one) are
//! values the host keeps, so their members are read one by one (docs/RUST.md, "Effects").

use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer};

use crate::agents::permission_baseline::PermissionMode;
use crate::js::JsValue;
use crate::loading::resolve::substitute::ShellPaths;
use crate::loading::resolve::user_config::ResolvedUserConfig;
use crate::ports::Signal;
use crate::registry::{Callback, Port, Set};
use crate::skills::execution::{Runnable, ShellAvailability, Trigger, TurnSession};
use crate::skills::expand_policy::{self, ExpandPolicy, SkillPluginSource};
use crate::skills::file_references;
use crate::skills::shell_arguments::ShellArguments;
use crate::skills::variables::{AllowedTools, VariableSource};

/// The values a skill's variables need beyond the request itself (see `SkillSession`).
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandSession {
    /// The id `Locations` knows the plugin by.
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// The saved `userConfig` values of the plugin.
    #[serde(default)]
    pub user_config: Option<ResolvedUserConfig>,
    /// A Windows host.
    #[serde(default)]
    pub windows: Option<bool>,
    /// The session runs in an isolated container, a cloud or Cowork session.
    #[serde(default)]
    pub isolated_session: Option<bool>,
    /// The absolute folder `skill.dir` is relative to.
    #[serde(default)]
    pub skill_root: Option<String>,
    /// How a `!` command writes the path variables for the shell.
    #[serde(default)]
    pub shell_paths: Option<ShellPaths>,
    /// Where the skill's plugin was loaded from; default `installed`. Only a plugin skill reads it.
    #[serde(default)]
    pub plugin_source: Option<SkillPluginSource>,
    /// How a `!` command receives the arguments (the host's `shellArguments`); default `pasted`.
    #[serde(default)]
    pub shell_arguments: Option<ShellArguments>,
}

/// `Omit<ExecutionContext, 'backgroundTasksDisabled'>`: who fired the skill and in what session.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestContext {
    /// False with `-p` or through the Agent SDK.
    pub interactive: bool,
    /// What fired the skill.
    pub trigger: Trigger,
    /// The commands of forked skills still running.
    #[serde(default)]
    pub running_forks: Option<Set<String>>,
}

/// One invocation's request.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpandRequest {
    /// The text after the command name, as typed.
    pub arguments: String,
    /// `${CLAUDE_SESSION_ID}`.
    pub session_id: String,
    /// The session's effort level: `${CLAUDE_EFFORT}` when the skill sets none and none is forced.
    #[serde(default)]
    pub effort: Option<String>,
    /// Auto mode changes what a command that needs approval does.
    #[serde(default)]
    pub permission_mode: Option<PermissionMode>,
    /// The `disableSkillShellExecution` setting.
    pub disable_shell_execution: bool,
    /// The shells the host has.
    pub shells: ShellAvailability,
    /// Who fired the skill and in what kind of session.
    pub context: RequestContext,
    /// The model allowlist, the effort cap and the model's supported levels.
    #[serde(default)]
    pub limits: Option<TurnSession>,
    /// What the variables need beyond the request.
    #[serde(default)]
    pub session: Option<ExpandSession>,
    /// The managed settings that take `allowed-tools` or `hooks` away.
    #[serde(default)]
    pub policy: Option<ExpandPolicy>,
    /// The session shell's working directory; default `Locations.projectDir()`.
    #[serde(default)]
    pub cwd: Option<String>,
    /// The `bashOutputMaxChars` setting.
    #[serde(default)]
    pub bash_output_max_chars: Option<f64>,
    /// Bare mode.
    #[serde(default)]
    pub bare: Option<bool>,
    /// The environment a command runs with, for the shell it runs in; default the host's.
    #[serde(default)]
    pub command_environment: Option<Callback>,
    /// Ends the work.
    pub signal: Signal,
}

/// `ExpandRequest` as it crosses: the host's record when it holds `commandEnvironment`, else data.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum RequestGiven {
    /// The request holds a function.
    Host(Port),
    /// The request is data.
    Data(Box<ExpandRequest>),
}

impl RequestGiven {
    /// The request's members.
    pub async fn read(self) -> ExpandRequest {
        match self {
            Self::Data(request) => *request,
            Self::Host(port) => ExpandRequest {
                arguments: port.member("arguments").await,
                session_id: port.member("sessionId").await,
                effort: port.member("effort").await,
                permission_mode: port.member("permissionMode").await,
                disable_shell_execution: port.member("disableShellExecution").await,
                shells: port.member("shells").await,
                context: port.member("context").await,
                limits: port.member("limits").await,
                session: port.member("session").await,
                policy: port.member("policy").await,
                cwd: port.member("cwd").await,
                bash_output_max_chars: port.member("bashOutputMaxChars").await,
                bare: port.member("bare").await,
                command_environment: port.member("commandEnvironment").await,
                signal: port.member("signal").await,
            },
        }
    }
}

/// `ExpandPorts`: the host's record, which `runInjections` reads itself, and the two ports the
/// expansion reads.
#[derive(Clone, Copy, Debug)]
pub struct ExpandPorts {
    /// The record: `runner`, `scratch`, `locations`, `environment`, `permissions`.
    pub record: Port,
    /// Where the paths come from.
    pub locations: Port,
    /// The process environment.
    pub environment: Port,
}

impl ExpandPorts {
    /// The ports of the host's record.
    pub async fn of(record: Port) -> Self {
        Self {
            record,
            locations: record.member("locations").await,
            environment: record.member("environment").await,
        }
    }
}

/// What of a skill or command only the expansion itself reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Own {
    /// The Markdown after the frontmatter.
    pub body: String,
    /// `arguments` of the frontmatter: the names of `$name`, in argument order.
    pub arguments: Vec<String>,
    /// `disallowed-tools`.
    pub disallowed_tools: Vec<String>,
    /// `origin.core`, read by `isCoreSkill`.
    pub origin: CoreMark,
}

/// The `core` mark of an origin: a core package's skill.
#[derive(Clone, Debug, Deserialize)]
pub struct CoreMark {
    /// `true` for a core package's skill.
    #[serde(default)]
    pub core: Option<JsValue>,
}

/// `ReferencedOrigin`: the origin as `attachesFileReferences` reads it.
#[derive(Clone, Debug, Deserialize)]
struct Referenced {
    origin: file_references::SkillOrigin,
}

/// `Skill | Command`, read once as the `Pick` each module the expansion calls reads.
#[derive(Clone, Debug)]
pub struct Source {
    /// What only the expansion reads.
    pub own: Own,
    /// How it runs (`execution.ts`): its command, name, `agent`, `context`, `shell`.
    pub runnable: Runnable,
    /// What the managed policy reads (`expand-policy.ts`): its origin, `context`, `hooks`.
    pub policy: expand_policy::Source,
    /// What its variables read (`variables.ts`): its origin and folder.
    pub variables: VariableSource,
    /// `allowed-tools`, as written.
    pub allowed: AllowedTools,
    /// Its origin, as `attachesFileReferences` reads it.
    pub references: file_references::SkillOrigin,
}

/// One view of the skill, a deserialize error as the boundary's.
fn view<T: DeserializeOwned, E: serde::de::Error>(skill: &JsValue) -> Result<T, E> {
    T::deserialize(skill.clone()).map_err(E::custom)
}

impl<'de> Deserialize<'de> for Source {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let skill = JsValue::deserialize(deserializer)?;
        let referenced: Referenced = view(&skill)?;
        Ok(Self {
            own: view(&skill)?,
            runnable: view(&skill)?,
            policy: view(&skill)?,
            variables: view(&skill)?,
            allowed: view(&skill)?,
            references: referenced.origin,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skill_is_read_as_each_view() {
        let skill = crate::js::json_parse::json_parse(
            r#"{"command":"x","name":"x","body":"b","arguments":[],"allowedTools":["Bash(ls)"],
            "disallowedTools":[],"origin":{"kind":"plugin","plugin":"p","core":true},"dir":"skills/x"}"#,
        )
        .unwrap();
        let source = Source::deserialize(skill).unwrap();
        assert_eq!(source.own.origin.core, Some(JsValue::Bool(true)));
        assert_eq!(source.runnable.command, "x");
        assert_eq!(source.allowed.allowed_tools, vec!["Bash(ls)".to_owned()]);
        assert_eq!(source.references.kind, "plugin");
        assert_eq!(source.variables.dir.as_deref(), Some("skills/x"));
    }
}
