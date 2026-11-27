//! The types of `src/loading/resolve/scope.ts`, a module of types only (docs/RUST.md: they are
//! declared where the TypeScript first needs them, here). A [`FieldPolicy`] whose `readsAsEmpty`
//! is the host's function crosses as a port, so an export takes a [`PolicyArg`] and reads it.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::super::user_config::ResolvedUserConfig;
use super::super::variables::{PathRendering, VariableScope};
use crate::format::mcp_env::ReadsAsEmpty;
use crate::format::remote_mcp_credentials::reads_as_empty_in_remote_mcp;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Callback, Function, Port};

/// What a plugin's `${...}` references resolve against. Everything is data or a port: the kit asks
/// `Locations` for paths (and so for the data folder, only when referenced) and `Environment` for
/// `${VAR}`; it knows no machine. Its own members hold no function, so it crosses as data whose
/// `locations` and `environment` are ports (`{"$port": n}`).
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubstitutionScope<L = Port, E = Port> {
    /// The plugin identifier the host uses (`name@marketplace`, or the bare name).
    pub plugin_id: String,
    /// The host's locations.
    pub locations: L,
    /// The host's environment.
    pub environment: E,
    /// The saved options and their declarations.
    #[serde(default)]
    pub user_config: ResolvedUserConfig,
    /// True on a Windows host: its paths come back with backslashes and the docs write them with
    /// forward slashes in most fields (plugins/manifest-reference.md, "Quoting and path
    /// separators"). The kit cannot tell (L3), so the host says.
    #[serde(default)]
    pub windows: bool,
    /// Variables another layer owns, substituted in the same single pass: skills pass
    /// `CLAUDE_SKILL_DIR`, `CLAUDE_SESSION_ID`, `CLAUDE_EFFORT`. They win over `${VAR}` of the
    /// environment.
    #[serde(default)]
    pub extra: Option<IndexMap<String, String>>,
    /// The names of `extra` whose value is a path (`CLAUDE_SKILL_DIR`): a surface writes them as it
    /// writes the three path variables, so a shell-parsed one gets the reference, not the path.
    #[serde(default)]
    pub extra_paths: Option<Vec<String>>,
    /// How a string a shell parses (a monitor `command`, an MCP `headersHelper`, a skill's `!`
    /// command; never a bash shell-form hook, which always keeps the reference) writes the path
    /// variables. `environment-reference` (the default but for a monitor): the braced POSIX
    /// reference `${CLAUDE_PLUGIN_ROOT}`, with the value in the process environment, so a folder
    /// name holding `$(...)`, a backtick or a quote is never parsed as shell (judgment,
    /// spec/judgments.md, as Claude Code does for PowerShell hooks, hooks.md). `pasted` writes the
    /// path itself, the docs' text: the default for a monitor `command`, whose path variables are
    /// never exported (Agent Plugins Extended §9.2.4, J9.17).
    #[serde(default)]
    pub shell_paths: Option<ShellPaths>,
    /// Cancels the work.
    #[serde(default)]
    pub signal: Option<Signal>,
}

impl<L: Clone, E> SubstitutionScope<L, E> {
    /// What the path variables are read from (`pluginVariableValue`'s scope).
    pub fn variable_scope(&self) -> VariableScope<L> {
        VariableScope {
            plugin_id: self.plugin_id.clone(),
            locations: self.locations.clone(),
            signal: self.signal.clone(),
        }
    }
}

/// The two ways a string a shell parses carries a path variable (`SubstitutionScope.shellPaths`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellPaths {
    /// The path itself.
    Pasted,
    /// The reference, the value in the process environment.
    EnvironmentReference,
}

/// Which `${user_config.KEY}` values a field takes (plugins/manifest-reference.md, "Reference a
/// saved value").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserConfigPolicy {
    /// Any value.
    Allow,
    /// A sensitive value becomes a placeholder.
    NonSensitive,
    /// The reference stays as written.
    Refuse,
}

/// Whether `${VAR}` reads the environment.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EnvironmentExpansion {
    /// `${VAR}` and `${VAR:-default}` read the environment.
    Expand,
    /// Left as written.
    Literal,
}

/// Names read as empty even when set, a default ignored (MCP `url` and `headers` credentials):
/// the core's own rule, or a function of the host's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadsAsEmptyRule {
    /// `readsAsEmptyInRemoteMcp` (`format/remote-mcp-credentials`).
    RemoteMcp,
    /// The host's function.
    Host(Callback),
}

impl ReadsAsEmpty for ReadsAsEmptyRule {
    fn reads_as_empty<'a>(&'a self, name: &'a str) -> BoxFuture<'a, bool> {
        match self {
            Self::RemoteMcp => Box::pin(std::future::ready(reads_as_empty_in_remote_mcp(name))),
            Self::Host(callback) => callback.reads_as_empty(name),
        }
    }
}

/// `readsAsEmptyInRemoteMcp` as the host holds it: a function of the core's.
struct RemoteMcpReads;

crate::registry::methods! {
    RemoteMcpReads {
        "call" => ok |_this, name: String| reads_as_empty_in_remote_mcp(&name);
    }
}

impl Serialize for ReadsAsEmptyRule {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::RemoteMcp => Function::new(RemoteMcpReads).serialize(serializer),
            Self::Host(callback) => callback.serialize(serializer),
        }
    }
}

/// What one field accepts: the differences the docs list per plugin component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldPolicy {
    /// `allow`: any value. `non-sensitive`: a sensitive value becomes a placeholder (skill and
    /// agent content). `refuse`: the reference stays as written (shell fields; the caller reports
    /// it).
    pub user_config: UserConfigPolicy,
    /// `expand`: `${VAR}` and `${VAR:-default}` read the environment. `literal`: left as written.
    pub environment: EnvironmentExpansion,
    /// `normalized`: forward slashes on a Windows host. `native`: as the host reports them
    /// (exec-form hooks). `environment-reference`: PowerShell's `${env:NAME}`, left to the process
    /// environment. `shell-reference`: the POSIX `${NAME}`, left to the process environment (the
    /// strings a shell parses, `SubstitutionScope.shellPaths`).
    pub paths: PathRendering,
    /// Names read as empty even when set, a default ignored (MCP `url` and `headers` credentials).
    /// A policy that holds the host's function crosses as a port ([`PolicyArg`]).
    #[serde(skip_deserializing, skip_serializing_if = "Option::is_none")]
    pub reads_as_empty: Option<ReadsAsEmptyRule>,
}

/// A policy as the host passes it: data, or a port when its `readsAsEmpty` is a function.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(untagged)]
pub enum PolicyArg {
    /// A policy holding a function, its members read one by one.
    Host(Port),
    /// A policy of data only.
    Data(FieldPolicy),
}

impl PolicyArg {
    /// The policy, its members read from the host when it is a port.
    pub async fn read(self) -> FieldPolicy {
        match self {
            Self::Data(policy) => policy,
            Self::Host(port) => {
                let user_config = port.member("userConfig");
                let environment = port.member("environment");
                let paths = port.member("paths");
                let reads = port.member::<Option<Callback>>("readsAsEmpty");
                FieldPolicy {
                    user_config: user_config.await,
                    environment: environment.await,
                    paths: paths.await,
                    reads_as_empty: reads.await.map(ReadsAsEmptyRule::Host),
                }
            }
        }
    }
}
