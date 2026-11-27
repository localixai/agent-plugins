//! Port of `src/loading/resolve/shell-prefix.ts`, switched.
//!
//! `CLAUDE_CODE_SHELL_PREFIX` (env-vars.md, "Variables"; spec §3.6, §7.3.2.3 item 5, §8.2.1): a
//! command that wraps the shell commands the client spawns. Each runs as `<prefix> '<command line>'`,
//! the whole line one shell-quoted argument the wrapper re-runs with a shell (`exec bash -c "$1"`).
//! Pure: the host runs the `CommandSpec` through its `CommandRunner`.

use serde::Deserialize;

use crate::js::trim;
use crate::ports::command_runner::{CommandSpec, ShellKind};
use crate::ports::environment::Environment;
use crate::registry::Port;

/// The variable's name.
pub const SHELL_PREFIX_VARIABLE: &str = "CLAUDE_CODE_SHELL_PREFIX";

/// The prefix in force: the variable's value as written, absent when it is unset or blank (as for hooks).
pub async fn shell_prefix_of(environment: &impl Environment) -> Option<String> {
    let value = environment.get(SHELL_PREFIX_VARIABLE).await?;
    (!trim(&value).is_empty()).then_some(value)
}

/// One word quoted for a POSIX shell: single quotes, an embedded `'` written `'\''`.
#[must_use]
pub fn posix_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// `<prefix> '<line>'`: the prefix as written, then the line as its one quoted argument.
#[must_use]
pub fn shell_prefixed(prefix: &str, line: &str) -> String {
    format!("{prefix} {}", posix_quote(line))
}

/// What a stdio server's start reads of its config (`Pick<McpStdioServerConfig, 'command' | 'args'>`).
#[derive(Clone, Debug, Deserialize)]
pub struct StdioStart {
    /// The program.
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Option<Vec<String>>,
}

/// How a stdio MCP server starts (spec §8.2.1, J8.26): without a prefix, its `command` and `args` as
/// an exec; with one, the line of `command` and each `args` element, each POSIX-quoted and joined by
/// spaces, passed quoted once more after the prefix, through bash, the shell hook commands use by
/// default (`shell` when the host's default differs, as on Windows without Git Bash).
pub async fn mcp_stdio_start(
    server: &StdioStart,
    environment: &impl Environment,
    shell: Option<ShellKind>,
) -> CommandSpec {
    let args = server.args.clone().unwrap_or_default();
    let Some(prefix) = shell_prefix_of(environment).await else {
        return CommandSpec::Exec {
            command: server.command.clone(),
            args,
        };
    };
    let line: Vec<String> = std::iter::once(&server.command)
        .chain(&args)
        .map(|word| posix_quote(word))
        .collect();
    CommandSpec::Shell {
        command: shell_prefixed(&prefix, &line.join(" ")),
        shell: shell.unwrap_or(ShellKind::Bash),
    }
}

crate::registry::exports! {
    "loading/resolve/shell-prefix" {
        "SHELL_PREFIX_VARIABLE" => ok || SHELL_PREFIX_VARIABLE;
        "shellPrefixOf" => async ok |environment: Port| shell_prefix_of(&environment).await;
        "posixQuote" => ok |text: String| posix_quote(&text);
        "shellPrefixed" => ok |prefix: String, line: String| shell_prefixed(&prefix, &line);
        "mcpStdioStart" => async ok |server: StdioStart, environment: Port, shell: Option<ShellKind>|
            mcp_stdio_start(&server, &environment, shell).await;
    }
}
