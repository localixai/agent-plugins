//! Port of `src/marketplace/node-deps.ts`, switched.
//!
//! The plan of a plugin's Node.js package install (plugins/loading.md, "Node.js package
//! dependencies"). Pure: `NodeDepsInput` in, a plan out; the host copies the two files, runs the
//! command through `CommandRunner` and moves `node_modules` (docs/API.md, "Node.js package
//! dependencies").

use indexmap::IndexMap;

use super::command_source::WithSignal;
use super::node_deps_checks::{CHECKS, Prepared, Ready, arguments, prepare};
use super::node_deps_types::{
    INSTALL_TIMEOUT_MS, Install, InstallReason, InstallRun, InstallStage, Lockfiles, MoveBack,
    NodeDepsInput, NodeDepsInstall, NodeDepsPlan, NodeDepsResult, SkipWhy, StagedFile,
};
use crate::format::config_directory::Join;
use crate::format::issues::{PluginIssue, error};
use crate::format::path_text::is_within;
use crate::ports::Signal;
use crate::ports::command_runner::{CommandRun, CommandSpec};
use crate::registry::{Callback, Port};

async fn install_of(ready: &Ready, input: &NodeDepsInput, join: &dyn Join) -> NodeDepsInstall {
    let (install_dir, plugin_root) = (&input.install_dir, &input.plugin_root);
    let files = ["package.json", ready.lockfile.file_name()];
    let mut copy = Vec::new();
    for file in files {
        copy.push(StagedFile {
            from: join.join(plugin_root, file).await,
            to: join.join(install_dir, file).await,
        });
    }
    NodeDepsInstall {
        action: Install::Install,
        reason: InstallReason::CopiedVersionDirectory,
        package_manager: ready.manager,
        lockfile: ready.lockfile,
        frozen: true,
        ignore_scripts: true,
        run: InstallRun {
            spec: CommandSpec::Exec {
                command: ready.manager.as_str().to_owned(),
                args: arguments(ready.manager)
                    .iter()
                    .map(|arg| (*arg).to_owned())
                    .collect(),
            },
            cwd: install_dir.clone(),
            timeout_ms: INSTALL_TIMEOUT_MS,
        },
        stage: InstallStage {
            directory: install_dir.clone(),
            copy,
            move_back: MoveBack {
                from: join.join(install_dir, "node_modules").await,
                to: join.join(plugin_root, "node_modules").await,
                replace_existing: true,
            },
        },
    }
}

/// Paths of the host, as `PathJoin` makes them: a Windows path (`C:\p\.deps`) and a path in
/// another case name the same folder. The guard errs toward refusing (judgment): a refusal costs a
/// folder with another name, a miss lets the package manager read `.npmrc`, `.env` and
/// `bunfig.toml`.
fn comparable(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// The install folder must be a folder of its own: "the package manager runs in a folder of its
/// own that holds only a copy of the checked dependency list, so npm and Bun don't read the
/// plugin's `.npmrc`, `.env`, or `bunfig.toml`". A folder that is the plugin or inside it would
/// read them.
fn folder_issues(input: &NodeDepsInput) -> Vec<PluginIssue> {
    if is_within(
        &comparable(&input.plugin_root),
        &comparable(&input.install_dir),
    ) {
        vec![error(
            "installDir",
            &format!(
                "The install folder \"{}\" must lie outside the plugin directory \"{}\"",
                input.install_dir, input.plugin_root
            ),
        )]
    } else {
        Vec::new()
    }
}

/// Whether and how Claude Code installs a plugin's Node.js packages.
pub async fn plan_node_deps_install(input: &NodeDepsInput, join: &dyn Join) -> NodeDepsResult {
    let ready = match prepare(input) {
        Prepared::Skip(skip) => {
            return NodeDepsResult {
                plan: NodeDepsPlan::Skip(skip),
                issues: Vec::new(),
            };
        }
        Prepared::Ready(ready) => ready,
    };
    let refused = CHECKS.iter().find_map(|check| check(&ready, input));
    if let Some(refused) = refused {
        return NodeDepsResult {
            plan: NodeDepsPlan::Skip(refused),
            issues: Vec::new(),
        };
    }
    let issues = folder_issues(input);
    if issues.is_empty() {
        NodeDepsResult {
            plan: NodeDepsPlan::Install(Box::new(install_of(&ready, input, join).await)),
            issues: Vec::new(),
        }
    } else {
        NodeDepsResult {
            plan: NodeDepsPlan::Skip(
                SkipWhy::InstallFolder {
                    install_dir: input.install_dir.clone(),
                }
                .into(),
            ),
            issues,
        }
    }
}

/// The process the host runs for an install: the plan, the host's environment and abort signal;
/// output merged so a failure shows its cause.
#[must_use]
pub fn node_deps_run(
    plan: &NodeDepsInstall,
    env: IndexMap<String, String>,
    signal: Signal,
) -> CommandRun {
    CommandRun {
        spec: plan.run.spec.clone(),
        cwd: plan.run.cwd.clone(),
        env,
        stdin: None,
        timeout_ms: Some(plan.run.timeout_ms),
        merge_stderr: Some(true),
        signal,
    }
}

/// `NodeDepsInput` as the host passed it: a record that holds its `join`, so it crossed as a port
/// and each member is read from it.
async fn planned_from_host(input: Port) -> NodeDepsResult {
    let join = input.member::<Callback>("join").await;
    let data = NodeDepsInput {
        origin: input.member("origin").await,
        plugin_root: input.member("pluginRoot").await,
        install_dir: input.member("installDir").await,
        package_json: input.member("packageJson").await,
        lockfiles: input.member::<Lockfiles>("lockfiles").await,
        other_lockfiles: input.member("otherLockfiles").await,
        available: input.member("available").await,
        default_registry: input.member("defaultRegistry").await,
    };
    plan_node_deps_install(&data, &join).await
}

crate::registry::exports! {
    "marketplace/node-deps" {
        "planNodeDepsInstall" => async ok |input: Port| planned_from_host(input).await;
        "nodeDepsRun" => async ok |plan: NodeDepsInstall, env: IndexMap<String, String>, with: WithSignal|
            node_deps_run(&plan, env, with.signal);
    }
}
