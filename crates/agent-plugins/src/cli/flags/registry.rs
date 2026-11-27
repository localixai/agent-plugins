//! Ported from `src/cli/flags/registry.ts`; only the core uses it now, so it has no wrapper.
//!
//! One strategy per flag (cli-reference.md, "CLI flags"). A flag added to `FLAG_NAMES` fails to
//! compile until it has a row; the session plan never switches on a flag's name. The table is a
//! `match` over [`FlagName`], each row the strategy of its flag, given the value as the command
//! line gave it (a repeatable flag's values, a switch's `true`, the rest the text) and answering a
//! [`Contribution`].

use serde::Serialize;
use serde::de::{Deserialize, DeserializeOwned};

use super::add_dir::add_dir;
use super::agents::{agent, agents};
use super::channels::{channels, development_channels};
use super::mcp::{mcp_config, strict_mcp_config};
use super::modes::{bare, disable_slash_commands, restricted, safe_mode};
use super::plugins::{plugin_dir, plugin_url};
use super::session::{permission_mode, tools};
use super::settings::{setting_sources, settings};
use super::subagent_prompt::{
    append_subagent_prompt, append_subagent_prompt_file, forward_subagent_text,
};
use super::types::{Contribution, FlagContext, FlagName};
use crate::js::JsValue;
use crate::js::value::to_js_value;

/// A strategy's own record as the `Contribution` it is.
fn contribution(record: &impl Serialize) -> Contribution {
    to_js_value(record)
        .ok()
        .and_then(|value| Contribution::deserialize(value).ok())
        .unwrap_or_default()
}

/// The flag's value as its strategy takes it; one the host typed otherwise reads as empty.
fn value_of<T: DeserializeOwned + Default>(value: &JsValue) -> T {
    T::deserialize(value.clone()).unwrap_or_default()
}

/// `FLAG_STRATEGIES[name](value, context)`.
#[must_use]
pub fn flag_strategy(name: FlagName, value: &JsValue, context: &FlagContext) -> Contribution {
    let text = || value_of::<String>(value);
    let list = || value_of::<Vec<String>>(value);
    match name {
        FlagName::PluginDir => plugin_dir(&list(), context),
        FlagName::PluginUrl => plugin_url(&list(), context),
        FlagName::Bare => contribution(&bare()),
        FlagName::SafeMode => contribution(&safe_mode()),
        FlagName::Restricted => contribution(&restricted()),
        FlagName::SettingSources => setting_sources(&text()),
        FlagName::Settings => settings(&text(), context),
        FlagName::DisableSlashCommands => contribution(&disable_slash_commands()),
        FlagName::AddDir => contribution(&add_dir(&list(), context)),
        FlagName::Agent => agent(&text()),
        FlagName::Agents => agents(&text(), context),
        FlagName::AppendSubagentSystemPrompt => append_subagent_prompt(&text(), context),
        FlagName::AppendSubagentSystemPromptFile => append_subagent_prompt_file(&text(), context),
        FlagName::ForwardSubagentText => forward_subagent_text(true, context),
        FlagName::Channels => channels(&list()),
        FlagName::DangerouslyLoadDevelopmentChannels => development_channels(&list(), context),
        FlagName::McpConfig => mcp_config(&list(), context),
        FlagName::StrictMcpConfig => strict_mcp_config(),
        FlagName::PermissionMode => permission_mode(&text(), context),
        FlagName::Tools => tools(&text()),
    }
}
