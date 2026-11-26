//! The crate's functions by name: the one boundary a host in another language calls through
//! (the npm package's thin TypeScript wrapper today, any language with a WebAssembly runtime or a
//! C ABI next). A Rust host calls the modules directly and never needs it.
//!
//! A function is named `<module>#<function>`: the module is the npm package's path under `src/`
//! without `.ts`, the function its exported name (`format/path-text#isWithin`), so a wrapper's
//! file and the Rust port it calls are one name apart. [`call`] takes the arguments as a JSON
//! array and answers one JSON object:
//!
//! - `{"result": value}`: what the function returned;
//! - `{"throws": message}`: the function's own error, which the wrapper throws as an `Error` with
//!   that message, as the TypeScript did;
//! - `{"fault": why}`: a bug on one side of the boundary (an unknown name, arguments that do not
//!   decode), never an answer of the function.
//!
//! Arguments and results are [`JsValue`]s, so JavaScript's own values cross whole: what JSON has
//! no spelling for is tagged as the wrapper writes it (`src/wasm/wire.ts`), `{"$undefined":
//! true}` and `{"$number": "NaN" | "Infinity" | "-Infinity" | "-0"}` (and `$set`, `$map`,
//! `$bytes`, `$regexp`: [`Set`], [`Map`], [`Bytes`], [`RegExp`]); a result that is `None` is `undefined`
//! unless the export says `nullable`. Each module declares its exports with `exports!` at the end of its file, and
//! [`MODULES`] lists those modules.
//!
//! An export that waits on the host (it takes a port, a callback or an `AbortSignal`, or the
//! TypeScript declares it `async`) is driven by the host with a plain loop, no callback crossing
//! the boundary: [`start`] runs it until it waits and answers a [`Step`], the [`Need`]s it waits
//! on; the host does each (calls its port's method, its callback) and gives the answers to
//! [`resume`], until the step is done. The values the host keeps are numbers here
//! ([`Port`], [`Callback`], the `Signal` of `crate::ports`), and a result with methods is a
//! [`Handle`] (`registry/handle.rs`). A host's throw ends the task unless [`catch_host`] catches
//! it (`registry/catch.rs`). docs/RUST.md, "The boundary", has the protocol.

mod catch;
mod exports;
mod handle;
mod probe;
mod remote;
mod steps;
mod task;
mod wire;
mod wire_json;

pub use catch::{Thrown, catch_host};
pub(crate) use exports::exports;
pub(crate) use handle::methods;
pub use handle::{Function, Handle, Methods, release};
pub use remote::{Callback, Port};
#[doc(hidden)]
pub use steps::settled;
pub use steps::{resume, resume_json, start, start_json};
pub use task::{Need, Reply, Started, Step, cancel};
pub use wire::{Answer, Bytes, Map, RegExp, Rest, Set};
#[doc(hidden)]
pub use wire::{apply, fallible, nullable, ok, ready, run, sign, take_args};

use serde::Deserialize as _;

use crate::js::JsValue;

/// One module's exports, as `exports!` declares them.
#[derive(Debug, Clone, Copy)]
pub struct Module {
    /// The npm package's module path under `src/`, without `.ts`.
    pub name: &'static str,
    /// Its exported functions, in the order the module declares them.
    pub functions: &'static [&'static str],
    /// Answers a call of one of `functions`, or `None` for a name it does not export.
    pub call: fn(&str, &[JsValue]) -> Option<Answer>,
    /// Starts one of `functions` as a task (`start`), or `None` for a name it does not export.
    pub start: fn(&str, &[JsValue]) -> Option<Started>,
}

/// Every module with exports.
pub const MODULES: &[Module] = &[
    crate::agents::builtin_model::EXPORTS,
    crate::agents::catalog::builtin::EXPORTS,
    crate::agents::catalog::invoke::EXPORTS,
    crate::agents::catalog::types::EXPORTS,
    crate::agents::handback_denial::EXPORTS,
    crate::agents::mcp_scope::EXPORTS,
    crate::agents::memory::EXPORTS,
    crate::agents::model_value::EXPORTS,
    crate::agents::permission_baseline::EXPORTS,
    crate::agents::preload_skills::EXPORTS,
    crate::agents::prompt_surface::EXPORTS,
    crate::agents::teammate::EXPORTS,
    crate::agents::tool_filters::EXPORTS,
    crate::agents::workflow::attempts::EXPORTS,
    crate::agents::workflow::limits::EXPORTS,
    crate::agents::workflow::relaunch::EXPORTS,
    crate::agents::workflow::replay::EXPORTS,
    crate::agents::workflow::saved::EXPORTS,
    crate::agents::workflow::structured_output::EXPORTS,
    crate::agents::workflow::usage_limit::EXPORTS,
    crate::cli::background_session::EXPORTS,
    crate::cli::clean_configuration::EXPORTS,
    crate::cli::command_table::EXPORTS,
    crate::cli::flags::types::EXPORTS,
    crate::cli::flags::values::EXPORTS,
    crate::cli::lfs_pointer_line::EXPORTS,
    crate::cli::uninstall_flags::EXPORTS,
    crate::cli::uninstall_refusal::EXPORTS,
    crate::convert::adapters::claude_code_runner_source::EXPORTS,
    crate::convert::adapters::core_profiles_source::EXPORTS,
    crate::convert::adapters::core_runner_source::EXPORTS,
    crate::convert::namespaces::EXPORTS,
    crate::convert::notes::EXPORTS,
    crate::convert::placeholders::EXPORTS,
    crate::convert::profiles::EXPORTS,
    crate::convert::types::EXPORTS,
    crate::format::agent_name::EXPORTS,
    crate::format::agent_record::EXPORTS,
    crate::format::agent_scalars::EXPORTS,
    crate::format::canonical_json::EXPORTS,
    crate::format::code_points::EXPORTS,
    crate::format::config_directory::EXPORTS,
    crate::format::core::command::EXPORTS,
    crate::format::core::dual::EXPORTS,
    crate::format::core::extensions::EXPORTS,
    crate::format::core::hooks::EXPORTS,
    crate::format::core::icon::EXPORTS,
    crate::format::core::manifest::EXPORTS,
    crate::format::core::mcp::EXPORTS,
    crate::format::core::mcp_server::EXPORTS,
    crate::format::core::name::EXPORTS,
    crate::format::core::placeholders::EXPORTS,
    crate::format::core::remote::EXPORTS,
    crate::format::core::skill::EXPORTS,
    crate::format::core::subprocess::EXPORTS,
    crate::format::core::versions::EXPORTS,
    crate::format::credential_env::EXPORTS,
    crate::format::defined::EXPORTS,
    crate::format::env_flag::EXPORTS,
    crate::format::env_number::EXPORTS,
    crate::format::frontmatter::EXPORTS,
    crate::format::frontmatter_values::EXPORTS,
    crate::format::frontmatter_write::EXPORTS,
    crate::format::helper_credentials::EXPORTS,
    crate::format::helper_headers::EXPORTS,
    crate::format::host_errors::EXPORTS,
    crate::format::issues::EXPORTS,
    crate::format::layout::EXPORTS,
    crate::format::link_rule::EXPORTS,
    crate::format::listing::EXPORTS,
    crate::format::marketplace::archive_url::EXPORTS,
    crate::format::marketplace::blocked_hosts::EXPORTS,
    crate::format::marketplace::display_fields::EXPORTS,
    crate::format::marketplace::entry_append::EXPORTS,
    crate::format::marketplace::entry_checks::EXPORTS,
    crate::format::marketplace::header_names::EXPORTS,
    crate::format::marketplace::ids::EXPORTS,
    crate::format::marketplace::marketplace_name_lists::EXPORTS,
    crate::format::marketplace::marketplace_names::EXPORTS,
    crate::format::marketplace::marketplace_schema::EXPORTS,
    crate::format::marketplace::plugin_source_fields::EXPORTS,
    crate::format::marketplace::renames::EXPORTS,
    crate::format::marketplace::source_failures::EXPORTS,
    crate::format::marketplace::types::EXPORTS,
    crate::format::mcp_elicitation_form::EXPORTS,
    crate::format::mcp_knobs::EXPORTS,
    crate::format::mcp_output::EXPORTS,
    crate::format::mcp_prompts::EXPORTS,
    crate::format::mcp_reserved::EXPORTS,
    crate::format::mcp_tool::EXPORTS,
    crate::format::mcp_tool_schema::EXPORTS,
    crate::format::mcpb_manifest::EXPORTS,
    crate::format::messages::EXPORTS,
    crate::format::mod_files::EXPORTS,
    crate::format::model_id::EXPORTS,
    crate::format::names::EXPORTS,
    crate::format::output_style::EXPORTS,
    crate::format::path_escape::EXPORTS,
    crate::format::path_text::EXPORTS,
    crate::format::permission_rule_shape::EXPORTS,
    crate::format::read_json::EXPORTS,
    crate::format::remote_mcp_credentials::EXPORTS,
    crate::format::settings_files::EXPORTS,
    crate::format::skill_name::EXPORTS,
    crate::format::skill_types::EXPORTS,
    crate::format::theme::EXPORTS,
    crate::format::theme_color::EXPORTS,
    crate::format::theme_tokens::EXPORTS,
    crate::format::theme_write::EXPORTS,
    crate::format::token_estimate::EXPORTS,
    crate::format::validate::component_frontmatter::EXPORTS,
    crate::format::validate::finding::EXPORTS,
    crate::format::validate::mcp_remote_checks::EXPORTS,
    crate::format::validate::path_claims::EXPORTS,
    crate::format::validate::plugin_name::EXPORTS,
    crate::format::validate::plugin_paths::EXPORTS,
    crate::format::validate::received::EXPORTS,
    crate::format::validate::shell_command::EXPORTS,
    crate::format::validate::unknown_fields::EXPORTS,
    crate::format::versions::EXPORTS,
    crate::format::wildcard::EXPORTS,
    crate::foundation::folder_read::EXPORTS,
    crate::foundation::folder_walk::EXPORTS,
    crate::foundation::project_directories::EXPORTS,
    crate::hooks::core::output::EXPORTS,
    crate::hooks::core::profile_results::EXPORTS,
    crate::hooks::core::profiles::EXPORTS,
    crate::hooks::env_file::events::EXPORTS,
    crate::hooks::env_file::ports::EXPORTS,
    crate::hooks::env_file::powershell_reader::EXPORTS,
    crate::hooks::handlers::command::shell::EXPORTS,
    crate::hooks::handlers::remote::failure_run::EXPORTS,
    crate::hooks::handlers::remote::http_env::EXPORTS,
    crate::hooks::handlers::remote::http_policy::EXPORTS,
    crate::hooks::handlers::remote::mcp_input::EXPORTS,
    crate::hooks::handlers::remote::ok_false::EXPORTS,
    crate::hooks::handlers::remote::prompt_text::EXPORTS,
    crate::hooks::handlers::timeouts::EXPORTS,
    crate::hooks::input::conditional_fields::EXPORTS,
    crate::hooks::input::permission_mode::EXPORTS,
    crate::hooks::output::caps::EXPORTS,
    crate::hooks::output::defer::EXPORTS,
    crate::hooks::output::merge_rules::EXPORTS,
    crate::hooks::output::pre_tool_use_outcome::EXPORTS,
    crate::hooks::output::reminder_tags::EXPORTS,
    crate::hooks::output::terminal_sequence::EXPORTS,
    crate::hooks::output::text_cap::EXPORTS,
    crate::hooks::output::tool_output::EXPORTS,
    crate::hooks::output::tool_shape::EXPORTS,
    crate::hooks::output::worktree_path::EXPORTS,
    crate::hooks::register::EXPORTS,
    crate::hooks::select::event_catalog::EXPORTS,
    crate::hooks::select::gates::EXPORTS,
    crate::hooks::select::lock_gates::EXPORTS,
    crate::hooks::select::matcher_values::EXPORTS,
    crate::hooks::source_kind::EXPORTS,
    crate::hooks::verdict::exit_codes::EXPORTS,
    crate::hooks::verdict::model_switch::EXPORTS,
    crate::hooks::verdict::notices::EXPORTS,
    crate::hooks::verdict::plain_stdout::EXPORTS,
    crate::hooks::verdict::stdout_kind::EXPORTS,
    crate::loading::background_trust::EXPORTS,
    crate::loading::bin::EXPORTS,
    crate::loading::channels::EXPORTS,
    crate::loading::command_debug::EXPORTS,
    crate::loading::core::entry_fields::EXPORTS,
    crate::loading::declared_shape::EXPORTS,
    crate::loading::merge_hooks::EXPORTS,
    crate::loading::plugin_debug::EXPORTS,
    crate::loading::plugin_folder::EXPORTS,
    crate::loading::plugin_scaffold::EXPORTS,
    crate::loading::resolve::data_id::EXPORTS,
    crate::loading::resolve::field_map::EXPORTS,
    crate::loading::resolve::headers_helper_origins::EXPORTS,
    crate::loading::resolve::lsp_startup::EXPORTS,
    crate::loading::resolve::powershell_no_program::EXPORTS,
    crate::loading::resolve::shell_guard_hook::EXPORTS,
    crate::loading::resolve::shell_guard_monitor::EXPORTS,
    crate::loading::resolve::shell_prefix::EXPORTS,
    crate::loading::resolve::without_plugin::EXPORTS,
    crate::loading::trust_gates::EXPORTS,
    crate::loading::validate::links::EXPORTS,
    crate::loading::validate::paths::EXPORTS,
    crate::loading::validate::report::EXPORTS,
    crate::marketplace::archive_scan::EXPORTS,
    crate::marketplace::command_mode::EXPORTS,
    crate::marketplace::command_output::EXPORTS,
    crate::marketplace::folder_collision::EXPORTS,
    crate::marketplace::git_transport::EXPORTS,
    crate::marketplace::header_map::EXPORTS,
    crate::marketplace::header_policy::EXPORTS,
    crate::marketplace::headless_report::EXPORTS,
    crate::marketplace::http_download::EXPORTS,
    crate::marketplace::install_failures::EXPORTS,
    crate::marketplace::install_hosts::EXPORTS,
    crate::marketplace::lfs_pointer::EXPORTS,
    crate::marketplace::missing_plugin_diagnosis::EXPORTS,
    crate::marketplace::name_conflicts::EXPORTS,
    crate::marketplace::plugin_content::EXPORTS,
    crate::marketplace::plugin_footprint::EXPORTS,
    crate::marketplace::plugin_hints::EXPORTS,
    crate::marketplace::reload_summary::EXPORTS,
    crate::marketplace::remote_plugins::EXPORTS,
    crate::marketplace::seed_registry::EXPORTS,
    crate::marketplace::session_origins::EXPORTS,
    crate::marketplace::submodules::EXPORTS,
    crate::marketplace::telemetry_names::EXPORTS,
    crate::marketplace::version_hash::EXPORTS,
    crate::protocol::effort::EXPORTS,
    crate::protocol::exit_reasons::EXPORTS,
    crate::protocol::hook_config::EXPORTS,
    crate::protocol::hook_events::EXPORTS,
    crate::protocol::mods_api_names::EXPORTS,
    crate::protocol::mods_limits::EXPORTS,
    crate::protocol::mods_settings::EXPORTS,
    crate::protocol::mods_tiers::EXPORTS,
    crate::protocol::mods_ui_names::EXPORTS,
    crate::protocol::permission_modes::EXPORTS,
    crate::protocol::settings_keys::EXPORTS,
    crate::protocol::tool_names::EXPORTS,
    crate::rules::bash::dialect::EXPORTS,
    crate::rules::bash::keywords::EXPORTS,
    crate::rules::bash::powershell_aliases::EXPORTS,
    crate::rules::bash::safe_env::EXPORTS,
    crate::rules::bash::text_pattern::EXPORTS,
    crate::rules::decide::approval_memory::EXPORTS,
    crate::rules::decide::hook_permission_updates::EXPORTS,
    crate::rules::decide::hook_rewrite::EXPORTS,
    crate::rules::decide::no_approval_surface::EXPORTS,
    crate::rules::decide::sed_script::EXPORTS,
    crate::rules::decide::unanswered::EXPORTS,
    crate::rules::decide::write_block_messages::EXPORTS,
    crate::rules::path::posix::EXPORTS,
    crate::rules::restricts::EXPORTS,
    crate::rules::sources::EXPORTS,
    crate::rules::target::input::EXPORTS,
    crate::rules::target::mcp::EXPORTS,
    crate::rules::tools::inputs::exit_plan_mode::EXPORTS,
    crate::rules::tools::inputs::grep::EXPORTS,
    crate::rules::tools::inputs::web_search::EXPORTS,
    crate::rules::tools::names::EXPORTS,
    crate::rules::tools::responses::structured_patch::EXPORTS,
    crate::rules::tools::rule_formats::EXPORTS,
    crate::rules::tools::semantics::edit::EXPORTS,
    crate::rules::tools::semantics::encoding::EXPORTS,
    crate::rules::tools::semantics::read_window::EXPORTS,
    crate::rules::tools::semantics::web_fetch::EXPORTS,
    crate::rules::tools::semantics::web_search::EXPORTS,
    crate::rules::tools::shell_selection::EXPORTS,
    crate::skills::arguments::EXPORTS,
    crate::skills::expand_policy::EXPORTS,
    crate::skills::folder_line::EXPORTS,
    crate::skills::fork::EXPORTS,
    crate::skills::hook_limits::EXPORTS,
    crate::skills::lifecycle::EXPORTS,
    crate::skills::mention::EXPORTS,
    crate::skills::scan::names::EXPORTS,
    crate::skills::shell_refusal::EXPORTS,
    crate::skills::shell_scan::EXPORTS,
    crate::skills::skill_rule::EXPORTS,
    crate::updates::auto_update_skips::EXPORTS,
    crate::updates::env_flags::EXPORTS,
    crate::updates::update_targets::EXPORTS,
    probe::EXPORTS,
];

/// Answers a call: `name` is `<module>#<function>`, `args` the arguments as they decoded. A
/// handle's method (`<handle>#<method>`) is answered too, when it answers at once.
#[must_use]
pub fn answer(name: &str, args: &[JsValue]) -> Answer {
    let Some((module, function)) = name.split_once('#') else {
        return Answer::Fault(format!("`{name}` names no function: <module>#<function>"));
    };
    if module.parse::<u32>().is_ok() {
        return match start(name, args) {
            Step::Done(answer) => answer,
            Step::Needs { task, .. } => {
                cancel(task);
                Answer::Fault(format!("`{name}` waits on the host: a host starts it"))
            }
            Step::Rethrow { .. } => Answer::Fault(format!("`{name}` waits on the host")),
        };
    }
    MODULES
        .iter()
        .find(|candidate| candidate.name == module)
        .and_then(|found| (found.call)(function, args))
        .unwrap_or_else(|| Answer::Fault(format!("`{name}` is not exported")))
}

/// The arguments' JSON text as values, each wire tag read back as the value it stands for.
pub(crate) fn decoded(args: &str) -> Result<Vec<JsValue>, String> {
    let raw = wire_json::read(args)
        .map_err(|problem| format!("the arguments are not JSON: {problem}"))?;
    match JsValue::deserialize(raw) {
        Ok(JsValue::Array(list)) => Ok(list),
        Ok(_) => Err("the arguments are not a JSON array".to_owned()),
        Err(error) => Err(format!("the arguments do not decode: {error}")),
    }
}

/// Answers a call given as JSON text: `args` is the arguments' JSON array in the wire's
/// spelling, the answer the JSON text of an [`Answer`].
#[must_use]
pub fn call(name: &str, args: &str) -> String {
    decoded(args)
        .map_or_else(Answer::Fault, |list| answer(name, &list))
        .to_json_text()
}
