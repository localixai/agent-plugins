//! Port of `src/loading/resolve/surfaces.ts`, switched.
//!
//! Where a string is substituted, finer than a component where the docs split one: a hook runs in
//! exec form or shell form (or through PowerShell), and an MCP server is stdio or remote, whose
//! `headersHelper` runs through a shell.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::field_map::VariableComponent;
use super::substitute::{
    EnvironmentExpansion, FieldPolicy, ReadsAsEmptyRule, ShellPaths, Substituted,
    SubstitutionScope, UserConfigPolicy,
};
use super::substituter::substitute_template;
use super::variables::PathRendering;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;

/// Where a string is substituted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SubstitutionSurface {
    /// An exec-form hook.
    HookExec,
    /// A shell-form hook.
    HookShell,
    /// A hook run through PowerShell.
    HookPowershell,
    /// A monitor's `command`.
    MonitorCommand,
    /// A stdio MCP server.
    McpStdio,
    /// A remote MCP server's `url` and `headers`.
    McpRemote,
    /// A remote MCP server's `headersHelper`.
    McpHeadersHelper,
    /// A language server.
    Lsp,
    /// Skill, command and agent content.
    MarkdownContent,
    /// A skill's `!` command for Bash.
    SkillBash,
    /// A skill's `!` command for PowerShell.
    SkillPowershell,
}

/// Every surface.
pub const SUBSTITUTION_SURFACES: [SubstitutionSurface; 11] = [
    SubstitutionSurface::HookExec,
    SubstitutionSurface::HookShell,
    SubstitutionSurface::HookPowershell,
    SubstitutionSurface::MonitorCommand,
    SubstitutionSurface::McpStdio,
    SubstitutionSurface::McpRemote,
    SubstitutionSurface::McpHeadersHelper,
    SubstitutionSurface::Lsp,
    SubstitutionSurface::MarkdownContent,
    SubstitutionSurface::SkillBash,
    SubstitutionSurface::SkillPowershell,
];

impl SubstitutionSurface {
    /// The surface's name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::HookExec => "hook-exec",
            Self::HookShell => "hook-shell",
            Self::HookPowershell => "hook-powershell",
            Self::MonitorCommand => "monitor-command",
            Self::McpStdio => "mcp-stdio",
            Self::McpRemote => "mcp-remote",
            Self::McpHeadersHelper => "mcp-headers-helper",
            Self::Lsp => "lsp",
            Self::MarkdownContent => "markdown-content",
            Self::SkillBash => "skill-bash",
            Self::SkillPowershell => "skill-powershell",
        }
    }
}

/// The component whose row of the table each surface belongs to.
#[must_use]
pub const fn surface_component(surface: SubstitutionSurface) -> VariableComponent {
    match surface {
        SubstitutionSurface::HookExec
        | SubstitutionSurface::HookShell
        | SubstitutionSurface::HookPowershell => VariableComponent::HookCommand,
        SubstitutionSurface::MonitorCommand => VariableComponent::MonitorCommand,
        SubstitutionSurface::McpStdio => VariableComponent::McpStdio,
        SubstitutionSurface::McpRemote | SubstitutionSurface::McpHeadersHelper => {
            VariableComponent::McpRemote
        }
        SubstitutionSurface::Lsp => VariableComponent::Lsp,
        SubstitutionSurface::MarkdownContent
        | SubstitutionSurface::SkillBash
        | SubstitutionSurface::SkillPowershell => VariableComponent::MarkdownContent,
    }
}

const fn policy(
    user_config: UserConfigPolicy,
    environment: EnvironmentExpansion,
    paths: PathRendering,
) -> FieldPolicy {
    FieldPolicy {
        user_config,
        environment,
        paths,
        reads_as_empty: None,
    }
}

/// Skill and agent content: a sensitive option is a placeholder, paths are written in.
const MARKDOWN: FieldPolicy = policy(
    UserConfigPolicy::NonSensitive,
    EnvironmentExpansion::Literal,
    PathRendering::Normalized,
);

/// The differences between surfaces, from the docs:
/// - `${user_config.*}`: exec-form `args`, MCP config, LSP config (manifest-reference, "Reference a
///   saved value"); never shell-form hooks, monitors or `headersHelper` ("Fields that run through a
///   shell"); non-sensitive only in skill and agent content.
/// - `${VAR}`: MCP `command`, `args`, `env`, `url`, `headers` (mcp.md, not `headersHelper`) and
///   monitor `command` (components.md, "Monitors"). Judgment: nowhere else; a hook or LSP command
///   keeps `${HOME}` for the process, and the docs give the kit no expansion to do.
/// - Windows paths: forward slashes, except exec-form hooks (native) and PowerShell hooks, where
///   the placeholders become `${env:NAME}` (hooks.md, "Reference scripts by path in PowerShell hooks").
///   A bash shell-form hook keeps `${NAME}` for the shell to read from the hook's environment, as
///   Claude Code does (oracle 2.1.322), whatever the scope's `shellPaths`.
/// - `mcp-remote`: credential variables read as empty in `url` and `headers` (mcp.md, "Credential
///   variables that read as empty"), `readsAsEmptyInRemoteMcp`.
/// - A hook with no `shell` field runs in PowerShell on Windows without Git Bash (hooks.md, "Exec
///   form and shell form"); the kit cannot tell, so the caller picks `hook-shell` or `hook-powershell`.
/// - A skill's `!` command is skill content (skills.md, "Available string substitutions") that the
///   Bash or the PowerShell tool runs ("How injected commands run"): `skill-bash` and
///   `skill-powershell`, the text the shell receives; the permission check reads the same command
///   as `markdown-content`.
#[must_use]
pub const fn surface_policy_of(surface: SubstitutionSurface) -> FieldPolicy {
    use EnvironmentExpansion::{Expand, Literal};
    use PathRendering::{EnvironmentReference, Native, Normalized, ShellReference};
    use UserConfigPolicy::{Allow, Refuse};
    match surface {
        SubstitutionSurface::HookExec => policy(Allow, Literal, Native),
        SubstitutionSurface::HookShell => policy(Refuse, Literal, ShellReference),
        SubstitutionSurface::HookPowershell => policy(Refuse, Literal, EnvironmentReference),
        SubstitutionSurface::MonitorCommand => policy(Refuse, Expand, Normalized),
        SubstitutionSurface::McpStdio => policy(Allow, Expand, Normalized),
        SubstitutionSurface::McpRemote => FieldPolicy {
            reads_as_empty: Some(ReadsAsEmptyRule::RemoteMcp),
            ..policy(Allow, Expand, Normalized)
        },
        SubstitutionSurface::McpHeadersHelper => policy(Refuse, Literal, Normalized),
        SubstitutionSurface::Lsp => policy(Allow, Literal, Normalized),
        SubstitutionSurface::MarkdownContent
        | SubstitutionSurface::SkillBash
        | SubstitutionSurface::SkillPowershell => MARKDOWN,
    }
}

/// `SURFACE_POLICY` as the host holds it: one policy per surface.
fn surface_policy_table() -> IndexMap<&'static str, FieldPolicy> {
    SUBSTITUTION_SURFACES
        .into_iter()
        .map(|surface| (surface.as_str(), surface_policy_of(surface)))
        .collect()
}

/// `SURFACE_COMPONENT` as the host holds it.
fn surface_component_table() -> IndexMap<&'static str, VariableComponent> {
    SUBSTITUTION_SURFACES
        .into_iter()
        .map(|surface| (surface.as_str(), surface_component(surface)))
        .collect()
}

/// How a POSIX shell's text carries a path under each `shellPaths`.
const fn posix(mode: ShellPaths) -> PathRendering {
    match mode {
        ShellPaths::Pasted => PathRendering::Normalized,
        ShellPaths::EnvironmentReference => PathRendering::ShellReference,
    }
}

/// A skill's PowerShell `!` command: the reference is `${env:NAME}`.
const fn powershell(mode: ShellPaths) -> PathRendering {
    match mode {
        ShellPaths::Pasted => PathRendering::Normalized,
        ShellPaths::EnvironmentReference => PathRendering::EnvironmentReference,
    }
}

/// The surfaces whose text a shell parses and where Claude Code pastes the paths, and how each
/// writes a path under the scope's `shellPaths`: a monitor `command`, an MCP `headersHelper`
/// (components.md, "Monitors"; mcp.md, "Use dynamic headers") and a skill's `!` command (skills.md,
/// "How injected commands run"), whose PowerShell reference is `${env:NAME}`. Hooks are not here:
/// Claude Code never pastes into them, a bash shell-form hook keeps `${NAME}` (oracle 2.1.322) and a
/// PowerShell hook gets `${env:NAME}` (hooks.md, "Reference scripts by path in PowerShell hooks").
fn shell_parsed(surface: SubstitutionSurface) -> Option<fn(ShellPaths) -> PathRendering> {
    match surface {
        SubstitutionSurface::MonitorCommand
        | SubstitutionSurface::McpHeadersHelper
        | SubstitutionSurface::SkillBash => Some(posix),
        SubstitutionSurface::SkillPowershell => Some(powershell),
        SubstitutionSurface::HookExec
        | SubstitutionSurface::HookShell
        | SubstitutionSurface::HookPowershell
        | SubstitutionSurface::McpStdio
        | SubstitutionSurface::McpRemote
        | SubstitutionSurface::Lsp
        | SubstitutionSurface::MarkdownContent => None,
    }
}

/// The default of `SubstitutionScope.shellPaths`, per surface: the one place it is decided
/// (spec/judgments.md). A monitor `command` is pasted, its path variables never exported (Agent
/// Plugins Extended §9.2.4, Judgment J9.17), so the reference is a host's explicit opt-in there; a
/// `headersHelper` and a skill's `!` command get the reference (J3.5 allows it).
const fn default_shell_paths(surface: SubstitutionSurface) -> Option<ShellPaths> {
    match surface {
        SubstitutionSurface::MonitorCommand => Some(ShellPaths::Pasted),
        SubstitutionSurface::HookExec
        | SubstitutionSurface::HookShell
        | SubstitutionSurface::HookPowershell
        | SubstitutionSurface::McpStdio
        | SubstitutionSurface::McpRemote
        | SubstitutionSurface::McpHeadersHelper
        | SubstitutionSurface::Lsp
        | SubstitutionSurface::MarkdownContent
        | SubstitutionSurface::SkillBash
        | SubstitutionSurface::SkillPowershell => None,
    }
}
const OTHER_DEFAULT: ShellPaths = ShellPaths::EnvironmentReference;

/// `Pick<SubstitutionScope, 'shellPaths'>`: the one member these read.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellPathsScope {
    /// `SubstitutionScope.shellPaths`.
    #[serde(default)]
    pub shell_paths: Option<ShellPaths>,
}

/// The scope's `shellPaths`, or the surface's default.
#[must_use]
pub fn shell_paths_of(scope: ShellPathsScope, surface: SubstitutionSurface) -> ShellPaths {
    scope
        .shell_paths
        .or_else(|| default_shell_paths(surface))
        .unwrap_or(OTHER_DEFAULT)
}

/// The policy of a surface in this scope: `SURFACE_POLICY`, with a shell-parsed surface's paths as
/// the scope's `shellPaths` make them. Every resolver asks this, so the choice is made once.
#[must_use]
pub fn surface_policy(surface: SubstitutionSurface, scope: ShellPathsScope) -> FieldPolicy {
    match shell_parsed(surface) {
        None => surface_policy_of(surface),
        Some(rendering) => FieldPolicy {
            paths: rendering(shell_paths_of(scope, surface)),
            ..surface_policy_of(surface)
        },
    }
}

/// `substituteTemplate` with the policy of a surface in this scope.
pub async fn substitute_surface<L: Locations + Clone, E: Environment>(
    surface: SubstitutionSurface,
    template: &str,
    scope: &SubstitutionScope<L, E>,
    path: &str,
) -> Substituted {
    let shell = ShellPathsScope {
        shell_paths: scope.shell_paths,
    };
    substitute_template(template, scope, &surface_policy(surface, shell), path).await
}

crate::registry::exports! {
    "loading/resolve/surfaces" {
        "SUBSTITUTION_SURFACES" => ok || SUBSTITUTION_SURFACES;
        "SURFACE_COMPONENT" => ok surface_component_table;
        "SURFACE_POLICY" => ok surface_policy_table;
        "substituteSurface" => async ok |surface: SubstitutionSurface, template: String, scope: SubstitutionScope, path: Option<String>|
            substitute_surface(surface, &template, &scope, &path.unwrap_or_default()).await;
    }
}
