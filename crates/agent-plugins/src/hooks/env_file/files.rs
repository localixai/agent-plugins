//! Port of `src/hooks/env-file/files.ts`, switched.
//!
//! What a command hook asks for its `CLAUDE_ENV_FILE`: the path for this event and this hook's shell,
//! or none. `shell` is `undefined` for a hook in exec form, which runs no shell.

use indexmap::IndexMap;
use serde::Deserialize;

use super::events::{env_file_cleared_by, env_file_kind_of, shell_has_env_file};
use super::ports::{EnvFileKind, EnvFileStore};
use super::powershell_reader::{
    PowershellReadOptions, PowershellVariables, read_powershell_variables,
};
use crate::js::promise_all;
use crate::ports::command_runner::ShellKind;
use crate::ports::{BoxFuture, Signal};
use crate::registry::{Handle, Port};

/// What `powershellVariables` takes.
#[derive(Clone, Copy, Debug, Deserialize)]
pub struct PowershellOptions {
    /// A Windows host.
    pub windows: bool,
    /// A sandboxed PowerShell command receives none of the variables.
    pub sandboxed: bool,
}

/// What `createEnvFiles` takes.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvFilesOptions {
    /// The session the files are of: every call to the store names it.
    pub session_id: String,
    /// The script `CLAUDE_ENV_FILE` named in the launch environment ("any script you set `CLAUDE_ENV_FILE`
    /// to before launch"). Judgment (spec/judgments.md): it is a third source that a shell runs first;
    /// the hooks of the session never write it, they get the session's own files.
    #[serde(default)]
    pub launch_script: Option<String>,
}

/// The files of one session: what hooks write them, and what the shells that run later read from them.
pub struct EnvFiles<S> {
    store: S,
    session_id: String,
    launch_script: Option<String>,
}

/// The files of one session over the host's store.
#[must_use]
pub fn create_env_files<S: EnvFileStore>(store: S, options: EnvFilesOptions) -> EnvFiles<S> {
    EnvFiles {
        store,
        session_id: options.session_id,
        launch_script: options.launch_script,
    }
}

impl<S: EnvFileStore> EnvFiles<S> {
    /// The path for this event and this hook's shell, or none.
    pub async fn path_for(
        &self,
        event: &str,
        shell: Option<ShellKind>,
        signal: &Signal,
    ) -> Option<String> {
        let kind = env_file_kind_of(event);
        match kind {
            Some(kind) if shell_has_env_file(shell) => {
                Some(self.store.path(&self.session_id, kind, signal).await)
            }
            _ => None,
        }
    }

    /// Run before an event's hooks: a `CwdChanged` clears what `CwdChanged` and `FileChanged` hooks wrote.
    pub async fn before_event(&self, event: &str, signal: &Signal) {
        let kind = env_file_cleared_by(event);
        if let Some(kind) = kind {
            self.store.clear(&self.session_id, kind, signal).await;
        }
    }

    /// The scripts a Bash command runs before itself, in order: a script `CLAUDE_ENV_FILE` named at
    /// launch, the file of `SessionStart` and `Setup`, the file of `CwdChanged` and `FileChanged`
    /// ("Each Bash command runs the file's contents as shell code before the command itself").
    pub async fn scripts(&self, signal: &Signal) -> Vec<String> {
        let mut scripts: Vec<String> = self.launch_script.iter().cloned().collect();
        scripts.push(
            self.store
                .path(&self.session_id, EnvFileKind::Session, signal)
                .await,
        );
        scripts.push(
            self.store
                .path(&self.session_id, EnvFileKind::Cwd, signal)
                .await,
        );
        scripts
    }

    /// The variables a PowerShell command receives (`readPowershellVariables` over every script).
    pub async fn powershell_variables(
        &self,
        options: PowershellOptions,
        signal: &Signal,
    ) -> PowershellVariables {
        if options.sandboxed {
            return PowershellVariables {
                variables: IndexMap::new(),
                issues: Vec::new(),
            };
        }
        let paths = self.scripts(signal).await;
        let reads: Vec<BoxFuture<'_, Option<String>>> = paths
            .iter()
            .map(|path| self.store.read(path, signal))
            .collect();
        let texts = promise_all(reads).await;
        let joined = texts.into_iter().flatten().collect::<Vec<_>>().join("\n");
        read_powershell_variables(
            &joined,
            PowershellReadOptions {
                windows: options.windows,
            },
        )
    }
}

crate::registry::methods! {
    EnvFiles<Port> {
        "pathFor" => async ok |this, event: String, shell: Option<ShellKind>, signal: Signal|
            this.path_for(&event, shell, &signal).await;
        "beforeEvent" => async ok |this, event: String, signal: Signal|
            this.before_event(&event, &signal).await;
        "scripts" => async ok |this, signal: Signal| this.scripts(&signal).await;
        "powershellVariables" => async ok |this, options: PowershellOptions, signal: Signal|
            this.powershell_variables(options, &signal).await;
    }
}

crate::registry::exports! {
    "hooks/env-file/files" {
        "createEnvFiles" => ok |store: Port, options: EnvFilesOptions|
            Handle::new(create_env_files(store, options));
    }
}
