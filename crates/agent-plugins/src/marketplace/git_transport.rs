//! Port of `src/marketplace/git-transport.ts`, switched.
//!
//! How Claude Code clones with the credentials already on the machine (plugins/install.md, "Add a
//! private marketplace"; env-vars.md, `CLAUDE_CODE_PLUGIN_PREFER_HTTPS`): pure, the host runs git. The
//! variable names that switch git's prompts off are the kit's choice (spec/judgments.md); the docs say
//! only that "Git's prompts and askpass programs are off, including one set in `GIT_ASKPASS`".

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use crate::js::JsValue;
use crate::ports::BoxFuture;
use crate::registry::{Callback, Port};

/// How a GitHub shorthand is cloned.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloneTransport {
    /// Over HTTPS.
    Https,
    /// Over SSH.
    Ssh,
}

/// Whether the user's SSH key authenticates to `github.com`: a function of the host's, so its
/// answer may wait. A Rust host passes a closure, answered at once.
pub trait SshAuthenticates: Send + Sync {
    /// The answer of the probe.
    fn authenticates(&self) -> BoxFuture<'_, bool>;
}

impl<F: Fn() -> bool + Send + Sync> SshAuthenticates for F {
    fn authenticates(&self) -> BoxFuture<'_, bool> {
        Box::pin(std::future::ready(self()))
    }
}

impl SshAuthenticates for Callback {
    fn authenticates(&self) -> BoxFuture<'_, bool> {
        Box::pin(self.call::<bool>(Vec::<JsValue>::new()))
    }
}

/// Facts about the machine the host reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShorthandFacts<A> {
    /// `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1`: "skip that check and always clone over HTTPS".
    pub prefer_https: bool,
    /// Whether the user's SSH key authenticates to `github.com`; consulted only without `prefer_https`.
    pub ssh_authenticates: A,
}

/// A GitHub `owner/repo` shorthand: "Claude Code checks whether your SSH key authenticates to
/// `github.com`, then clones over SSH if it does and over HTTPS if it doesn't"; `PREFER_HTTPS` skips
/// the check.
pub async fn shorthand_transport<A: SshAuthenticates>(facts: &ShorthandFacts<A>) -> CloneTransport {
    if facts.prefer_https {
        return CloneTransport::Https;
    }
    if facts.ssh_authenticates.authenticates().await {
        CloneTransport::Ssh
    } else {
        CloneTransport::Https
    }
}

/// Environment settings that make git fail instead of asking. They replace what the user's environment holds.
pub const NO_PROMPT_ENVIRONMENT: [(&str, &str); 4] = [
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_ASKPASS", ""),
    ("SSH_ASKPASS", ""),
    ("GCM_INTERACTIVE", "never"),
];

/// `NO_PROMPT_ENVIRONMENT` as the object the plan carries.
fn no_prompt_environment() -> IndexMap<String, String> {
    NO_PROMPT_ENVIRONMENT
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

/// Where the user's git setup names an SSH program; the host reads these three.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SshProgramFacts {
    /// `GIT_SSH_COMMAND`.
    #[serde(default)]
    pub git_ssh_command: Option<String>,
    /// `GIT_SSH`.
    #[serde(default)]
    pub git_ssh: Option<String>,
    /// `core.sshCommand` of the git configuration.
    #[serde(default)]
    pub core_ssh_command: Option<String>,
}

/// Which SSH program a clone runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum SshProgram {
    /// The program the user's git setup names.
    #[serde(rename = "user-configured")]
    UserConfigured,
    /// `ssh` with `BatchMode`, which never asks for a passphrase.
    #[serde(rename = "ssh-batch-mode")]
    SshBatchMode,
}

/// How a clone is run.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClonePlan {
    /// Set on top of the inherited environment.
    pub environment: IndexMap<String, String>,
    /// Runs the program the user's git setup names; otherwise `ssh` with `BatchMode`, which never asks for a passphrase.
    pub ssh_program: SshProgram,
    /// The options the client's own `ssh` runs with: `BatchMode=yes` and `StrictHostKeyChecking=yes`,
    /// so a host not already in `known_hosts` is refused, never accepted (plugins/troubleshooting.md,
    /// "SSH host key is not in your known_hosts file"). Empty for a user-named program, whose command
    /// line is not rewritten (J15.14).
    pub ssh_options: Vec<String>,
}

const OWN_SSH_OPTIONS: [&str; 4] = ["-o", "BatchMode=yes", "-o", "StrictHostKeyChecking=yes"];

fn named(value: Option<&str>) -> bool {
    !value.unwrap_or("").is_empty()
}

/// The environment and SSH program of a clone, for an HTTPS credential helper (prompts off), an SSH
/// key that "must work without a passphrase prompt", and "if your git setup names an SSH program ...
/// Claude Code runs that program".
#[must_use]
pub fn clone_plan(ssh: &SshProgramFacts) -> ClonePlan {
    let configured = [&ssh.git_ssh_command, &ssh.git_ssh, &ssh.core_ssh_command]
        .iter()
        .any(|value| named(value.as_deref()));
    ClonePlan {
        environment: no_prompt_environment(),
        ssh_program: if configured {
            SshProgram::UserConfigured
        } else {
            SshProgram::SshBatchMode
        },
        ssh_options: if configured {
            Vec::new()
        } else {
            OWN_SSH_OPTIONS
                .iter()
                .map(|option| (*option).to_owned())
                .collect()
        },
    }
}

/// The transports tried in order for a GitHub `owner/repo` (plugins/host-marketplace.md, "Grant access
/// to a private marketplace"): `ssh -T git@github.com` first, SSH when it succeeds; "if the probe
/// fails, or the SSH clone itself fails, it clones over HTTPS". `CLAUDE_CODE_PLUGIN_PREFER_HTTPS=1` skips
/// the probe.
pub async fn shorthand_attempts<A: SshAuthenticates>(
    facts: &ShorthandFacts<A>,
) -> Vec<CloneTransport> {
    if shorthand_transport(facts).await == CloneTransport::Ssh {
        vec![CloneTransport::Ssh, CloneTransport::Https]
    } else {
        vec![CloneTransport::Https]
    }
}

/// `ShorthandFacts` as it crosses: a record holding a function, so a port whose members are read
/// (docs/RUST.md, "Effects").
async fn shorthand_facts(record: Port) -> ShorthandFacts<Callback> {
    let prefer_https = record.member("preferHttps");
    let ssh_authenticates = record.member("sshAuthenticates");
    ShorthandFacts {
        prefer_https: prefer_https.await,
        ssh_authenticates: ssh_authenticates.await,
    }
}

crate::registry::exports! {
    "marketplace/git-transport" {
        "shorthandTransport" => async ok |facts: Port|
            shorthand_transport(&shorthand_facts(facts).await).await;
        "NO_PROMPT_ENVIRONMENT" => ok no_prompt_environment;
        "clonePlan" => ok |ssh: Option<SshProgramFacts>| clone_plan(&ssh.unwrap_or_default());
        "shorthandAttempts" => async ok |facts: Port|
            shorthand_attempts(&shorthand_facts(facts).await).await;
    }
}
