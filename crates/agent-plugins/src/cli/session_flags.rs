//! Port of `src/cli/session-flags.ts`, switched.
//!
//! The flags of one `claude` run as a plan (cli-reference.md, "CLI flags"; plugins/cli-reference.md,
//! "Flags that load a plugin for one session"). Each flag is a strategy over `FlagName`
//! (`flags/registry.ts`); the plan is what they add up to, handed to the modules that own each
//! behaviour: settings layers, session plugins, the tool filter, the permission mode, the agents,
//! the channel verdict, the MCP sources. Pure over what the host parsed and read. A plan with
//! `refusals` is a run that exits `1` without starting.

use serde::Deserialize;

use super::flags::add_dir::{LoadFacts, add_dir_loads};
use super::flags::agents::session_agent;
use super::flags::registry::flag_strategy;
use super::flags::sideload::{plugin_dirs_variable_refusal, with_sideload_closing};
use super::flags::subagent_prompt::forward_subagent_text_variable;
use super::flags::types::{
    Contribution, FLAG_NAMES, FlagContext, FlagFacts, FlagName, GivenFlags, SessionPlan,
};
use super::result::{CommandOutput, usage_error};
use crate::format::env_flag::is_truthy_env;
use crate::js::JsValue;
use crate::js::value::{JsObject, to_js_value};

/// `EMPTY`. The plan is an object built by spreads, as the TypeScript builds it, so its members
/// keep the order JavaScript gives them: `EMPTY`'s, then each member a flag adds, where it first
/// adds it.
fn empty() -> JsObject {
    let empty_list = || JsValue::Array(Vec::new());
    [
        ("refusals", empty_list()),
        ("warnings", empty_list()),
        ("sessionPlugins", empty_list()),
        ("skipped", empty_list()),
        ("readsStoredCredentials", JsValue::Bool(true)),
        ("env", JsValue::Object(JsObject::new())),
        ("additionalDirectories", empty_list()),
        ("forwardSubagentText", JsValue::Bool(false)),
        ("channels", empty_list()),
        ("mcpConfigs", empty_list()),
        ("strictMcpConfig", JsValue::Bool(false)),
        ("optsIntoTaskTools", JsValue::Bool(false)),
        ("restricted", JsValue::Bool(false)),
        ("fileToolsConfined", JsValue::Bool(false)),
        ("cloudSessions", JsValue::from("allowed")),
        ("addDirLoads", empty_list()),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_owned(), value))
    .collect()
}

/// A member of the plan as its type, or the type's default when it is absent.
fn member<T: for<'de> Deserialize<'de> + Default>(plan: &JsObject, key: &str) -> T {
    plan.get(key)
        .cloned()
        .and_then(|value| T::deserialize(value).ok())
        .unwrap_or_default()
}

/// A value as the plan holds it.
fn value_of<T: serde::Serialize>(value: &T) -> JsValue {
    to_js_value(value).unwrap_or_default()
}

/// Two flags that each name a set of settings scopes leave the scopes both name.
fn scopes_of(before: Option<Vec<String>>, added: Option<Vec<String>>) -> Option<Vec<String>> {
    let added = added?;
    Some(match before {
        None => added,
        Some(before) => before.into_iter().filter(|s| added.contains(s)).collect(),
    })
}

/// `[...plan[key], ...(added[key] ?? [])]`.
fn grown(plan: &JsObject, added: &JsObject, key: &str) -> JsValue {
    JsValue::Array(
        [plan, added]
            .into_iter()
            .filter_map(|object| object.get(key).and_then(JsValue::as_array))
            .flatten()
            .cloned()
            .collect(),
    )
}

/// What `added` changes: lists grow, a scalar replaces, scopes intersect.
fn combine(plan: JsObject, added: &Contribution) -> JsObject {
    let scopes = scopes_of(member(&plan, "settingScopes"), added.setting_scopes.clone());
    let added = if let JsValue::Object(members) = value_of(added) {
        members
    } else {
        JsObject::new()
    };
    let mut skipped: Vec<JsValue> = Vec::new();
    if let JsValue::Array(all) = grown(&plan, &added, "skipped") {
        for one in all {
            if !skipped.contains(&one) {
                skipped.push(one);
            }
        }
    }
    let mut env: JsObject = member(&plan, "env");
    env.extend(member::<JsObject>(&added, "env"));
    let lists = [
        "refusals",
        "warnings",
        "sessionPlugins",
        "additionalDirectories",
        "channels",
        "mcpConfigs",
    ]
    .map(|key| (key, grown(&plan, &added, key)));
    let mut next = plan;
    next.extend(added);
    for (key, list) in lists {
        next.insert(key.to_owned(), list);
    }
    next.insert("skipped".to_owned(), JsValue::Array(skipped));
    next.insert("env".to_owned(), JsValue::Object(env));
    if let Some(scopes) = scopes {
        next.insert("settingScopes".to_owned(), value_of(&scopes));
    }
    next
}

fn run_flag(name: FlagName, context: &FlagContext) -> Contribution {
    let key = to_js_value(&name).ok();
    let value = key
        .as_ref()
        .and_then(JsValue::as_str)
        .and_then(|key| context.given.get(key));
    match value {
        None | Some(JsValue::Undefined) => Contribution::default(),
        Some(value) => flag_strategy(name, value, context),
    }
}

/// `given[name]` as text, when it is.
fn given_text(given: &GivenFlags, name: &str) -> Option<String> {
    given.get(name).and_then(JsValue::as_str).map(str::to_owned)
}

/// What only the whole plan decides: the agent (`--agent` over the `agent` setting), what an
/// `--add-dir` directory loads (it follows the modes and the setting sources the flags chose) and
/// the variables that stand in for a flag.
fn settled(plan: JsObject, given: &GivenFlags, facts: &FlagFacts) -> JsObject {
    let named = session_agent(given_text(given, "--agent"), facts.agent_setting.clone());
    let loads = if member::<Vec<JsValue>>(&plan, "additionalDirectories").is_empty() {
        Vec::new()
    } else {
        add_dir_loads(&LoadFacts {
            bare: matches!(given.get("--bare"), Some(JsValue::Bool(true))),
            claude_md_enabled: is_truthy_env(
                facts
                    .env
                    .get("CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD")
                    .and_then(Option::as_deref),
            ),
            skipped: member(&plan, "skipped"),
            setting_scopes: member(&plan, "settingScopes"),
        })
    };
    let variable = plugin_dirs_variable_refusal(facts);
    let mut base = plan;
    base.insert("addDirLoads".to_owned(), value_of(&loads));
    if let Some(named) = named {
        base.insert("agent".to_owned(), JsValue::from(named));
    }
    let mut added = forward_subagent_text_variable(facts);
    if let Some(variable) = variable {
        added.refusals = Some(vec![variable]);
    }
    let mut combined = combine(base, &added);
    let refusals = with_sideload_closing(member(&combined, "refusals"));
    combined.insert("refusals".to_owned(), value_of(&refusals));
    combined
}

/// What the flags a run was given add up to, in the order of `FLAG_NAMES`.
#[must_use]
pub fn plan_session_flags(given: GivenFlags, facts: FlagFacts) -> JsObject {
    let context = FlagContext { given, facts };
    let flagged = FLAG_NAMES.iter().fold(empty(), |plan, name| {
        combine(plan, &run_flag(*name, &context))
    });
    settled(flagged, &context.given, &context.facts)
}

/// What the run ends with when the plan refuses it: exit `1`, the reasons on stderr, no result line.
#[must_use]
pub fn startup_refusal(plan: &SessionPlan) -> Option<CommandOutput> {
    if plan.refusals.is_empty() {
        None
    } else {
        Some(usage_error(&plan.refusals.join("\n")))
    }
}

crate::registry::exports! {
    "cli/session-flags" {
        "planSessionFlags" => ok |given: GivenFlags, facts: FlagFacts| plan_session_flags(given, facts);
        "startupRefusal" => ok |plan: SessionPlan| startup_refusal(&plan);
    }
}
