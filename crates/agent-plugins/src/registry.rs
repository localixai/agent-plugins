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
//! [`Handle`] (`registry/handle.rs`). docs/RUST.md, "The boundary", has the protocol.

mod exports;
mod handle;
mod probe;
mod remote;
mod steps;
mod task;
mod wire;

pub(crate) use exports::exports;
pub(crate) use handle::methods;
pub use handle::{Handle, Methods, release};
pub use remote::{Callback, Port};
pub use steps::{resume, resume_json, start, start_json};
pub use task::{Need, Reply, Started, Step, cancel};
pub use wire::{Answer, Bytes, Map, RegExp, Set};
#[doc(hidden)]
pub use wire::{apply, fallible, nullable, ok, ready, run, sign, take_args};

use serde::Deserialize as _;

use crate::js::JsValue;
use crate::js::json_parse::json_parse;

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
    crate::format::agent_name::EXPORTS,
    crate::format::agent_record::EXPORTS,
    crate::format::canonical_json::EXPORTS,
    crate::format::code_points::EXPORTS,
    crate::format::config_directory::EXPORTS,
    crate::format::core::command::EXPORTS,
    crate::format::core::document::EXPORTS,
    crate::format::core::extensions::EXPORTS,
    crate::format::core::manifest::EXPORTS,
    crate::format::core::mcp_server::EXPORTS,
    crate::format::core::name::EXPORTS,
    crate::format::core::placeholders::EXPORTS,
    crate::format::core::remote::EXPORTS,
    crate::format::core::subprocess::EXPORTS,
    crate::format::core::versions::EXPORTS,
    crate::format::credential_env::EXPORTS,
    crate::format::defined::EXPORTS,
    crate::format::env_flag::EXPORTS,
    crate::format::env_number::EXPORTS,
    crate::format::frontmatter_values::EXPORTS,
    crate::format::helper_credentials::EXPORTS,
    crate::format::helper_headers::EXPORTS,
    crate::format::host_errors::EXPORTS,
    crate::format::issues::EXPORTS,
    crate::format::link_rule::EXPORTS,
    crate::format::listing::EXPORTS,
    crate::format::marketplace::blocked_hosts::EXPORTS,
    crate::format::marketplace::display_fields::EXPORTS,
    crate::format::marketplace::entry_append::EXPORTS,
    crate::format::marketplace::header_names::EXPORTS,
    crate::format::marketplace::ids::EXPORTS,
    crate::format::marketplace::marketplace_name_lists::EXPORTS,
    crate::format::marketplace::marketplace_names::EXPORTS,
    crate::format::marketplace::plugin_source_fields::EXPORTS,
    crate::format::marketplace::renames::EXPORTS,
    crate::format::marketplace::source_failures::EXPORTS,
    crate::format::mcp_elicitation_check::EXPORTS,
    crate::format::mcp_prompts::EXPORTS,
    crate::format::mcp_reserved::EXPORTS,
    crate::format::mcp_tool::EXPORTS,
    crate::format::mcp_tool_schema::EXPORTS,
    crate::format::mcpb_manifest::EXPORTS,
    crate::format::messages::EXPORTS,
    crate::format::mod_files::EXPORTS,
    crate::format::model_id::EXPORTS,
    crate::format::names::EXPORTS,
    crate::format::path_escape::EXPORTS,
    crate::format::path_text::EXPORTS,
    crate::format::permission_rule_shape::EXPORTS,
    crate::format::read_json::EXPORTS,
    crate::format::remote_mcp_credentials::EXPORTS,
    crate::format::settings_files::EXPORTS,
    crate::format::skill_name::EXPORTS,
    crate::format::theme::EXPORTS,
    crate::format::theme_color::EXPORTS,
    crate::format::theme_tokens::EXPORTS,
    crate::format::theme_write::EXPORTS,
    crate::format::token_estimate::EXPORTS,
    crate::format::utf8::EXPORTS,
    crate::format::validate::finding::EXPORTS,
    crate::format::validate::mcp_remote_checks::EXPORTS,
    crate::format::validate::path_claims::EXPORTS,
    crate::format::validate::plugin_name::EXPORTS,
    crate::format::validate::received::EXPORTS,
    crate::format::validate::shell_command::EXPORTS,
    crate::format::validate::unknown_fields::EXPORTS,
    crate::format::versions::EXPORTS,
    crate::format::wildcard::EXPORTS,
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
    let raw =
        json_parse(args).map_err(|problem| format!("the arguments are not JSON: {problem}"))?;
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
