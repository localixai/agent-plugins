//! Port of `src/hooks/handlers/command/subprocess-env.ts`, switched.
//!
//! The environment of a subprocess Claude Code starts (env-vars.md, "Set variables in settings
//! files", "Precedence", `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB`, `CLAUDECODE`,
//! `CLAUDE_CODE_CHILD_SESSION`, `CLAUDE_CODE_SESSION_ID` and `CLAUDE_PID`; hooks.md, "Common input
//! fields"): the host's environment with the settings files' `env` written over it, its
//! credentials removed when the scrub is on (and the Anthropic ones under the HIPAA configuration),
//! the `OTEL_*` exporter variables removed ("from every subprocess it spawns"), and the variables
//! Claude Code sets for that kind of subprocess. One pure function, so a host builds the
//! environment of its own Bash, PowerShell, stdio MCP and Monitor processes the way the kit builds
//! a hook's.
//!
//! Which kind gets which variable is the docs' own list:
//!
//! | variable                    | hook | Bash | PowerShell | stdio MCP | Monitor |
//! | --------------------------- | ---- | ---- | ---------- | --------- | ------- |
//! | `CLAUDECODE`                | yes  | yes  | yes        | yes       | yes     |
//! | `CLAUDE_CODE_CHILD_SESSION` | yes  | yes  | yes        | no        | yes     |
//! | `CLAUDE_CODE_SESSION_ID`    | yes  | yes  | yes        | yes       | no      |
//! | `CLAUDE_PID`                | yes  | yes  | yes        | no        | no      |
//! | `CLAUDE_EFFORT`             | yes  | yes  | no         | no        | no      |
//!
//! Judgments (spec/judgments.md): `CLAUDECODE` is set for a Monitor process too (the docs list the
//! tools that get it by example); a variable the table does not give a kind is neither set nor
//! removed when the host's own environment holds it; `CLAUDE_ENV_FILE` of the host's environment
//! reaches no subprocess (a hook gets the file of its event from `processEnvironment`, a Bash
//! command runs the files); a variable of a session or process id the host did not give is not set.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::{in_key_order, number_to_string};
use crate::loading::resolve::subprocess_scrub::{ScrubOptions, scrub_environment};
use crate::settings::env::settings_env::apply_settings_env;

/// The kinds of subprocess.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubprocessKind {
    /// A hook's command.
    Hook,
    /// The Bash tool.
    Bash,
    /// The PowerShell tool.
    Powershell,
    /// A stdio MCP server.
    Mcp,
    /// A monitor.
    Monitor,
}

/// The kinds, in the package's order.
pub const SUBPROCESS_KINDS: [SubprocessKind; 5] = [
    SubprocessKind::Hook,
    SubprocessKind::Bash,
    SubprocessKind::Powershell,
    SubprocessKind::Mcp,
    SubprocessKind::Monitor,
];

/// Which of the optional variables a kind of subprocess gets.
#[derive(Clone, Copy)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the docs' table: one flag per variable"
)]
struct KindVariables {
    child_session: bool,
    session_id: bool,
    process_id: bool,
    /// env-vars.md, `CLAUDE_EFFORT`: "Set automatically in Bash tool subprocesses and hook commands".
    effort: bool,
}

const KIND_VARIABLES: [(SubprocessKind, KindVariables); 5] = [
    (
        SubprocessKind::Hook,
        KindVariables {
            child_session: true,
            session_id: true,
            process_id: true,
            effort: true,
        },
    ),
    (
        SubprocessKind::Bash,
        KindVariables {
            child_session: true,
            session_id: true,
            process_id: true,
            effort: true,
        },
    ),
    (
        SubprocessKind::Powershell,
        KindVariables {
            child_session: true,
            session_id: true,
            process_id: true,
            effort: false,
        },
    ),
    (
        SubprocessKind::Mcp,
        KindVariables {
            child_session: false,
            session_id: true,
            process_id: false,
            effort: false,
        },
    ),
    (
        SubprocessKind::Monitor,
        KindVariables {
            child_session: true,
            session_id: false,
            process_id: false,
            effort: false,
        },
    ),
];

/// `KIND_VARIABLES[kind]`: every kind has its row; `None` is not reached.
fn kind_variables(kind: SubprocessKind) -> Option<KindVariables> {
    KIND_VARIABLES
        .iter()
        .find(|(one, _)| *one == kind)
        .map(|(_, variables)| *variables)
}

/// What a subprocess's environment is built from.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubprocessEnvironmentInput {
    /// The kind of subprocess.
    pub kind: SubprocessKind,
    /// The host process's environment.
    pub environment: IndexMap<String, String>,
    /// `resolveSettingsEnv(...).values`: written over `environment`.
    #[serde(default)]
    pub settings_env: Option<IndexMap<String, String>>,
    /// The host's variable names are case-insensitive.
    pub windows: bool,
    /// The session's id, for `CLAUDE_CODE_SESSION_ID`.
    #[serde(default)]
    pub session_id: Option<String>,
    /// The host process's id, for `CLAUDE_PID`.
    #[serde(default)]
    pub process_id: Option<f64>,
    /// The effort level when the process starts, for `CLAUDE_EFFORT` ("only set when the model
    /// supports" it).
    #[serde(default)]
    pub effort: Option<String>,
    /// The HIPAA configuration applies to the session: the Anthropic credentials are removed.
    #[serde(default)]
    pub hipaa: Option<bool>,
}

/// hipaa-setup.md, "Anthropic credentials in commands, hooks, and MCP servers": "With the HIPAA
/// configuration applied, Claude Code removes the credentials it uses to reach Anthropic, such as
/// `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN`, from the environment of the shell commands,
/// hooks, and MCP servers it starts"; cloud provider and GitHub credentials stay (Agent Plugins
/// Extended §7.3.2.3 item 2). Judgment (spec/judgments.md): the credentials are the environment
/// variables of authentication.md, "Authentication precedence", that reach Anthropic, so
/// `CLAUDE_CODE_OAUTH_TOKEN` too; matched in any case on Windows.
const ANTHROPIC_CREDENTIALS: [&str; 3] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// `/^OTEL_/`, and `/^OTEL_/i` (without `u`, `i` folds ASCII letters only).
fn is_exporter(name: &str, windows: bool) -> bool {
    let prefix = name.as_bytes().get(..5);
    if windows {
        prefix.is_some_and(|start| start.eq_ignore_ascii_case(b"OTEL_"))
    } else {
        prefix == Some(b"OTEL_")
    }
}

const ENV_FILE: &str = "CLAUDE_ENV_FILE";

/// The variables Claude Code sets for this kind of subprocess.
fn set_by_claude_code(input: &SubprocessEnvironmentInput) -> IndexMap<String, String> {
    let mut set = IndexMap::from([("CLAUDECODE".to_owned(), "1".to_owned())]);
    let Some(kind) = kind_variables(input.kind) else {
        return set;
    };
    if kind.child_session {
        set.insert("CLAUDE_CODE_CHILD_SESSION".to_owned(), "1".to_owned());
    }
    if let Some(id) = input.session_id.as_ref().filter(|_| kind.session_id) {
        set.insert("CLAUDE_CODE_SESSION_ID".to_owned(), id.clone());
    }
    if let Some(id) = input.process_id.filter(|_| kind.process_id) {
        set.insert("CLAUDE_PID".to_owned(), number_to_string(id));
    }
    if let Some(effort) = input.effort.as_ref().filter(|_| kind.effort) {
        set.insert("CLAUDE_EFFORT".to_owned(), effort.clone());
    }
    set
}

/// The environment a subprocess of this kind inherits, with what Claude Code sets for it.
#[must_use]
pub fn subprocess_environment(input: &SubprocessEnvironmentInput) -> IndexMap<String, String> {
    let windows = input.windows;
    let none = IndexMap::new();
    let parent = apply_settings_env(
        &input.environment,
        input.settings_env.as_ref().unwrap_or(&none),
        windows,
    );
    let scrubbed = scrub_environment(&parent, &ScrubOptions::default());
    let name_of = |name: &str| -> String {
        if windows {
            name.to_uppercase()
        } else {
            name.to_owned()
        }
    };
    let is_env_file = |name: &str| name_of(name) == ENV_FILE;
    let is_credential = |name: &str| {
        input.hipaa == Some(true) && ANTHROPIC_CREDENTIALS.contains(&name_of(name).as_str())
    };
    let mut environment: IndexMap<String, String> = in_key_order(scrubbed)
        .into_iter()
        .filter(|(name, _)| {
            !is_exporter(name, windows) && !is_env_file(name) && !is_credential(name)
        })
        .collect();
    for (name, value) in set_by_claude_code(input) {
        environment.insert(name, value);
    }
    in_key_order(environment)
}

crate::registry::exports! {
    "hooks/handlers/command/subprocess-env" {
        "SUBPROCESS_KINDS" => ok || SUBPROCESS_KINDS;
        "subprocessEnvironment" => ok |input: SubprocessEnvironmentInput| subprocess_environment(&input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_exporter_prefix_folds_ascii_only_on_windows() {
        assert!(is_exporter("OTEL_X", false));
        assert!(!is_exporter("otel_x", false));
        assert!(is_exporter("otel_x", true));
        assert!(!is_exporter("OTE", true));
        assert!(!is_exporter("OTEL\u{17f}", true));
    }
}
