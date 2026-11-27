//! Port of `src/settings/env/settings-env.ts`, switched.
//!
//! The `env` block of the settings files for a host that knows its folders' trust per file
//! (env-vars.md, "Set variables in settings files" and "Precedence"; settings-reference.md, `env`).
//! The same rules as `resolveEnv`, in the shape a host applies directly: the values to write over
//! its environment.

use std::collections::HashSet;

use indexmap::IndexMap;
use serde::Serialize;

use crate::js::{JsValue, in_key_order, is_truthy};
use crate::ports::BoxFuture;
use crate::registry::Port;
use crate::settings::env::resolve_env::{
    NotAppliedVariable, Rules, RulesInput, ScopeTest, Variables, resolve_env_blocks,
};
use crate::settings::key_spec::REPOSITORY_FILES;
use crate::settings::layers::{PolicyLayer, PolicyScope};

/// What `resolveSettingsEnv` asks the host about its machine.
pub struct SettingsEnvContext<'a> {
    /// Whether the user trusted the workspace (settings-reference.md, "When Claude Code applies
    /// `env` values"); asked of `project` and `local` only.
    pub is_folder_trusted: &'a dyn ScopeTest,
    /// The environment Claude Code was launched with: an off value of a project file never
    /// replaces one set here.
    pub environment: Option<Variables>,
    /// Claude Desktop or a self-hosted runner built the launch environment: "Claude Code ignores an
    /// `env` value from any settings file for a variable the launch environment already sets."
    pub launch_owns_environment: Option<bool>,
    /// The host's variable names are case-insensitive.
    pub windows: Option<bool>,
}

/// The variables of a file that reach the environment, and the ones that do not.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SettingsEnv {
    /// The variables to put in the environment, replacing what the shell exported.
    pub values: IndexMap<String, String>,
    /// The variables of a file that do not reach the environment, and why. Never a value.
    pub ignored: Vec<NotAppliedVariable>,
}

fn is_repository_scope(scope: PolicyScope) -> bool {
    REPOSITORY_FILES.contains(&scope)
}

/// The variables of the layers' `env` blocks that reach the environment.
pub async fn resolve_settings_env(
    layers: &[PolicyLayer],
    context: &SettingsEnvContext<'_>,
) -> SettingsEnv {
    let shell = context.environment.clone().unwrap_or_default();
    let starts_now = StartsNow {
        trusted: context.is_folder_trusted,
    };
    let rules = Rules {
        starts_now: &starts_now,
        input: RulesInput {
            shell: &shell,
            launch_environment: (context.launch_owns_environment == Some(true)).then_some(&shell),
            is_safe_at_startup: None,
            windows: Some(context.windows.unwrap_or(false)),
        },
    };
    let files = resolve_env_blocks(layers, &rules).await;
    SettingsEnv {
        values: files.settings,
        ignored: files.not_applied,
    }
}

/// `!isRepositoryScope(scope) || context.isFolderTrusted(scope)`.
struct StartsNow<'a> {
    trusted: &'a dyn ScopeTest,
}

impl ScopeTest for StartsNow<'_> {
    fn holds(&self, scope: PolicyScope) -> BoxFuture<'_, bool> {
        if is_repository_scope(scope) {
            self.trusted.holds(scope)
        } else {
            Box::pin(std::future::ready(true))
        }
    }
}

/// The environment after the settings' variables are written into it: "Claude Code writes each
/// `env` entry into the process environment, replacing the value inherited from the shell".
/// Nothing is ever removed: "In a settings file you can set a variable but you can't remove one".
/// On Windows a name replaces the inherited one whatever its case.
#[must_use]
pub fn apply_settings_env(
    inherited: &IndexMap<String, String>,
    settings: &IndexMap<String, String>,
    windows: bool,
) -> IndexMap<String, String> {
    let fold = |name: &str| -> String {
        if windows {
            name.to_uppercase()
        } else {
            name.to_owned()
        }
    };
    let replaced: HashSet<String> = settings.keys().map(|name| fold(name)).collect();
    let mut merged: IndexMap<String, String> = in_key_order(inherited.clone())
        .into_iter()
        .filter(|(name, _)| !replaced.contains(&fold(name)))
        .collect();
    for (name, value) in in_key_order(settings.clone()) {
        merged.insert(name, value);
    }
    in_key_order(merged)
}

/// The host's `isFolderTrusted`, called on its context (`this` the context), its answer read as
/// JavaScript's `||` reads it.
struct HostTrust {
    context: Port,
}

impl ScopeTest for HostTrust {
    fn holds(&self, scope: PolicyScope) -> BoxFuture<'_, bool> {
        let asked = self
            .context
            .method::<JsValue>("isFolderTrusted", vec![scope.name().into()]);
        Box::pin(async move { is_truthy(&asked.await) })
    }
}

/// `resolveSettingsEnv` over the host's context, its data members read one by one.
async fn resolve_settings_env_of(layers: &[PolicyLayer], context: Port) -> SettingsEnv {
    let read = SettingsEnvContext {
        is_folder_trusted: &HostTrust { context },
        environment: context.member("environment").await,
        launch_owns_environment: context.member("launchOwnsEnvironment").await,
        windows: context.member("windows").await,
    };
    resolve_settings_env(layers, &read).await
}

crate::registry::exports! {
    "settings/env/settings-env" {
        "resolveSettingsEnv" => async ok |layers: Vec<PolicyLayer>, context: Port|
            resolve_settings_env_of(&layers, context).await;
        "applySettingsEnv" => ok
            |inherited: IndexMap<String, String>, settings: IndexMap<String, String>, windows: bool|
            apply_settings_env(&inherited, &settings, windows);
    }
}
