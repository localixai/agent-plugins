//! Port of `src/cli/uninstall-plan.ts`, switched.
//!
//! `claude plugin uninstall <plugin>` as two decisions around the host's writes (plugins/
//! cli-reference.md, "plugin uninstall"): `startUninstall` reads the arguments and the installed
//! plugins and says what to remove, or what the command ends with; the host then switches the plugin
//! off in the scope's settings, removes its install record and reads the installed plugins back, and
//! `concludeUninstall` says what the command prints and which saved state (options, secrets, data
//! directory) goes. The retention rules are `uninstallRetention`'s (G8). What the docs leave open is
//! in spec/judgments.md.

use serde::{Deserialize, Serialize};

use super::plugin_ref::{matching_ids, name_of, qualified_commands};
use super::records_refusal::records_refusal;
use super::result::{
    CommandOutput, Extras, ResultFields, command_failed, command_succeeded, usage_error,
};
use super::scope::{ChosenOrError, ScopeChoice, ScopedCommand, choose_scope};
use super::uninstall_data::{
    DATA_PRESERVED, Keeping, Remaining, Retention, RetentionFacts, uninstall_retention,
};
use super::uninstall_flags::{UninstallFlags, flag_problem};
use super::uninstall_refusal::{StillSwitchedOn, still_on_reason};
use crate::js::JsValue;
use crate::marketplace::dependency_graph::{GraphFacts, dependents_of};
use crate::marketplace::dependency_prune::plan_uninstall;
use crate::marketplace::install_messages::RefusingCommand;
use crate::marketplace::install_scopes::InstallScope;

/// What `uninstall` was given.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallArguments {
    /// `name` or `name@marketplace`.
    pub plugin: String,
    /// `--scope`.
    #[serde(default)]
    pub scope: Option<String>,
    /// `--keep-data`.
    #[serde(default)]
    pub keep_data: Option<bool>,
    /// `--prune`.
    #[serde(default)]
    pub prune: Option<bool>,
    /// `-y`.
    #[serde(default)]
    pub yes: Option<bool>,
    /// `--json`.
    #[serde(default)]
    pub json: Option<bool>,
}

/// What `startUninstall` reads.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallFacts {
    /// What is installed, at every scope.
    pub graph: GraphFacts,
    /// Both stdin and stdout are a terminal.
    pub tty: bool,
    /// What `readInstalledPlugins` reported for records it cannot read: the command refuses (§16.6.2).
    #[serde(default)]
    pub unreadable_records: Option<String>,
}

/// `skip`: `-y`; `ask`: the host asks `Remove? [y/N]` and reports the answer; `none`: nothing to remove.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Confirm {
    /// Nothing to remove.
    None,
    /// `-y`.
    Skip,
    /// Ask first.
    Ask,
}

/// What `--prune` removes with the plugin, and whether the user is asked first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PruneStep {
    /// The auto-installed dependencies no remaining plugin needs.
    pub orphans: Vec<String>,
    /// Whether the user is asked.
    pub confirm: Confirm,
}

/// What to remove.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveStart {
    /// The plugin as the user typed it, for the failure line.
    pub argument: String,
    /// Its id.
    pub id: String,
    /// The scope.
    pub scope: InstallScope,
    /// `--keep-data`.
    pub keep_data: bool,
    /// `--json`.
    pub json: bool,
    /// What `--prune` removes.
    pub prune: PruneStep,
    /// The enabled plugins that depend on it, by name: not refused, the line ends `— warning: required by a, c` (oracle 2.1.322).
    pub required_by: Vec<String>,
}

/// What `startUninstall` answers.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum UninstallStart {
    /// The command ends.
    Done {
        /// What it prints.
        output: CommandOutput,
    },
    /// The host removes the plugin.
    Remove(RemoveStart),
}

/// What the host found after switching the plugin off and removing its record.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AfterRemoval {
    /// The settings file that still switches, or may still switch, the plugin on at the scope.
    #[serde(default)]
    pub settings_still_on: Option<StillSwitchedOn>,
    /// The installed plugins read back; `'unreadable'` when the list could not be read.
    pub remaining: Remaining,
    /// The answer to `Remove? [y/N]`, when `prune.confirm` was `ask`.
    #[serde(default)]
    pub prune_confirmed: Option<bool>,
}

/// State the host deletes for one uninstalled plugin: only what the retention rules let go.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Cleanup {
    /// The plugin.
    pub id: String,
    /// Its stored options and secrets.
    pub delete_saved: bool,
    /// The folder under `~/.claude/plugins/data/` to delete; absent when the directory stays.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete_data_folder: Option<String>,
}

/// What the command prints and which state goes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallConclusion {
    /// What it prints.
    pub output: CommandOutput,
    /// `--json`.
    pub json: bool,
    /// Whether the host removes the install record (not when the settings still switch it on).
    pub remove_record: bool,
    /// The state to delete, one entry per plugin that has any.
    pub cleanup: Vec<Cleanup>,
    /// The orphaned dependencies the host also uninstalls from the scope.
    pub remove_pruned: Vec<String>,
}

const fn done(output: CommandOutput) -> UninstallStart {
    UninstallStart::Done { output }
}

fn failure_line(argument: &str, reason: &str) -> String {
    format!("Failed to uninstall plugin \"{argument}\": {reason}")
}

fn failed(message: &str, code: &str, fields: Option<ResultFields>) -> CommandOutput {
    command_failed(
        "uninstall",
        message,
        Extras {
            failure_code: Some(code.to_owned()),
            fields,
        },
    )
}

fn not_installed(argument: &str, scope: InstallScope) -> UninstallStart {
    let reason = format!(
        "Plugin \"{argument}\" is not installed at scope {}",
        scope.as_str()
    );
    done(failed(
        &failure_line(argument, &reason),
        "not_installed",
        None,
    ))
}

fn ambiguous(argument: &str, ids: &[String]) -> UninstallStart {
    let reason = format!(
        "\"{argument}\" is installed from more than one marketplace; run one of: {}",
        qualified_commands("uninstall", ids).join(", ")
    );
    done(failed(
        &failure_line(argument, &reason),
        "ambiguous_plugin",
        None,
    ))
}

fn prune_step(
    args: &UninstallArguments,
    graph: &GraphFacts,
    id: &str,
    scope: InstallScope,
) -> PruneStep {
    if args.prune != Some(true) {
        return PruneStep {
            orphans: Vec::new(),
            confirm: Confirm::None,
        };
    }
    let orphans = plan_uninstall(id, scope, graph, true).pruned;
    let confirm = if orphans.is_empty() {
        Confirm::None
    } else if args.yes == Some(true) {
        Confirm::Skip
    } else {
        Confirm::Ask
    };
    PruneStep { orphans, confirm }
}

/// What to remove, or what the command ends with before anything is touched.
#[must_use]
pub fn start_uninstall(args: &UninstallArguments, facts: &UninstallFacts) -> UninstallStart {
    let scope = match choose_scope(ScopedCommand::Uninstall, args.scope.as_deref()) {
        ChosenOrError::Error(output) => return done(output),
        ChosenOrError::Choice(ScopeChoice::Scope { scope }) => scope,
        // `uninstall` acts on one scope, `user` when `--scope` is omitted (`DefaultedCommand`).
        ChosenOrError::Choice(ScopeChoice::AutoDetect { .. } | ScopeChoice::AllScopes { .. }) => {
            InstallScope::User
        }
    };
    let flags = UninstallFlags {
        prune: args.prune,
        yes: args.yes,
        json: args.json,
    };
    if let Some(problem) = flag_problem(flags, facts.tty) {
        return done(usage_error(problem));
    }
    let unreadable = facts.unreadable_records.as_deref();
    if let Some(records) = records_refusal(RefusingCommand::Uninstall, &args.plugin, unreadable) {
        return done(records);
    }
    let here: Vec<String> = facts
        .graph
        .installed
        .iter()
        .filter(|record| record.scope == scope)
        .map(|record| record.id.clone())
        .collect();
    let matching = matching_ids(&args.plugin, &here);
    let Some(id) = matching.first() else {
        return not_installed(&args.plugin, scope);
    };
    if matching.len() > 1 {
        return ambiguous(&args.plugin, &matching);
    }
    UninstallStart::Remove(RemoveStart {
        argument: args.plugin.clone(),
        id: id.clone(),
        scope,
        keep_data: args.keep_data == Some(true),
        json: args.json == Some(true),
        prune: prune_step(args, &facts.graph, id, scope),
        required_by: dependents_of(id, &facts.graph)
            .iter()
            .map(|one| name_of(one))
            .collect(),
    })
}

fn result_fields(start: &RemoveStart) -> ResultFields {
    [
        ("pluginId".to_owned(), JsValue::from(start.id.as_str())),
        ("scope".to_owned(), JsValue::from(start.scope.as_str())),
    ]
    .into_iter()
    .collect()
}

fn still_on(start: &RemoveStart, fact: &StillSwitchedOn) -> UninstallConclusion {
    let name = name_of(&start.id);
    let reason = format!("\"{name}\" was not uninstalled: {}", still_on_reason(fact));
    let message = failure_line(&start.argument, &reason);
    UninstallConclusion {
        output: failed(&message, "settings_still_on", Some(result_fields(start))),
        json: start.json,
        remove_record: false,
        cleanup: Vec::new(),
        remove_pruned: Vec::new(),
    }
}

fn cleanup_of(id: &str, retention: &Retention) -> Option<Cleanup> {
    let delete_saved = retention.saved == Keeping::Delete;
    let folder = (retention.data == Keeping::Delete).then(|| retention.data_folder.clone());
    if !delete_saved && folder.is_none() {
        return None;
    }
    Some(Cleanup {
        id: id.to_owned(),
        delete_saved,
        delete_data_folder: folder,
    })
}

/// The orphans to remove: those the user confirmed or `-y` allowed.
fn pruned_of(start: &RemoveStart, after: &AfterRemoval) -> Vec<String> {
    if start.prune.confirm == Confirm::Ask && after.prune_confirmed != Some(true) {
        return Vec::new();
    }
    start.prune.orphans.clone()
}

fn without_pruned(remaining: &Remaining, pruned: &[String], scope: InstallScope) -> Remaining {
    match remaining {
        Remaining::Records(records) => Remaining::Records(
            records
                .iter()
                .filter(|record| record.scope != scope || !pruned.contains(&record.id))
                .cloned()
                .collect(),
        ),
        Remaining::Unreadable(text) => Remaining::Unreadable(text.clone()),
    }
}

const KEPT_NOTE: &str = "The options, secrets and data directory stayed because the installed plugins could not be read back and the plugin may still be installed at another scope";

fn unreadable_note(retention: &Retention) -> String {
    format!(
        "{KEPT_NOTE}. To delete them, remove the saved options and secrets in /plugin or the settings, and the folder ~/.claude/plugins/data/{}/",
        retention.data_folder
    )
}

fn required_note(required_by: &[String]) -> String {
    if required_by.is_empty() {
        String::new()
    } else {
        format!(" — warning: required by {}", required_by.join(", "))
    }
}

fn success_message(start: &RemoveStart, extra: Vec<String>) -> String {
    std::iter::once(format!(
        "Successfully uninstalled plugin: {} (scope: {}){}",
        name_of(&start.id),
        start.scope.as_str(),
        required_note(&start.required_by)
    ))
    .chain(extra)
    .collect::<Vec<_>>()
    .join("\n")
}

fn json_fields(start: &RemoveStart, retention: &Retention) -> ResultFields {
    let mut fields = result_fields(start);
    fields.insert(
        "keptData".to_owned(),
        JsValue::Bool(retention.data == Keeping::Keep),
    );
    if let Some(saved_kept) = retention.saved_kept {
        fields.insert("savedKept".to_owned(), JsValue::from(saved_kept));
    }
    fields
}

fn retention_of(id: &str, start: &RemoveStart, remaining: &Remaining) -> Retention {
    uninstall_retention(&RetentionFacts {
        id: id.to_owned(),
        scope: start.scope,
        keep_data: Some(start.keep_data),
        remaining: remaining.clone(),
        case_insensitive_file_system: None,
    })
}

/// What the command prints and which state goes, once the host has switched the plugin off and removed its record.
#[must_use]
pub fn conclude_uninstall(start: &RemoveStart, after: &AfterRemoval) -> UninstallConclusion {
    if let Some(fact) = &after.settings_still_on {
        return still_on(start, fact);
    }
    let pruned = pruned_of(start, after);
    let remaining = without_pruned(&after.remaining, &pruned, start.scope);
    let retention = retention_of(&start.id, start, &remaining);
    let mut extra = Vec::new();
    if retention.saved_kept.is_some() {
        extra.push(unreadable_note(&retention));
    }
    if !pruned.is_empty() {
        extra.push(format!(
            "Removed {} auto-installed plugins: {}",
            pruned.len(),
            pruned.join(", ")
        ));
    }
    // Judgment: `--keep-data` also keeps the folders of the plugins `--prune` removes (spec/judgments.md).
    let pruned_cleanup = pruned
        .iter()
        .filter_map(|id| cleanup_of(id, &retention_of(id, start, &remaining)));
    let cleanup = cleanup_of(&start.id, &retention)
        .into_iter()
        .chain(pruned_cleanup)
        .collect();
    UninstallConclusion {
        output: command_succeeded(
            "uninstall",
            &success_message(start, extra),
            Extras {
                failure_code: None,
                fields: Some(json_fields(start, &retention)),
            },
        ),
        json: start.json,
        remove_record: true,
        cleanup,
        remove_pruned: pruned,
    }
}

/// What `/plugin` shows beside a plugin whose data directory stayed (`keptData`).
#[must_use]
pub fn data_preserved_mark(kept_data: bool) -> &'static str {
    if kept_data { DATA_PRESERVED } else { "" }
}

crate::registry::exports! {
    "cli/uninstall-plan" {
        "startUninstall" => ok |args: UninstallArguments, facts: UninstallFacts| start_uninstall(&args, &facts);
        "concludeUninstall" => ok |start: RemoveStart, after: AfterRemoval| conclude_uninstall(&start, &after);
        "dataPreservedMark" => ok |kept_data: bool| data_preserved_mark(kept_data);
    }
}
