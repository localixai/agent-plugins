//! Port of `src/hooks/engine.ts`, switched.
//!
//! The engine over the real stages: select, build, run through the ports, read, merge. It is a
//! handle the host holds (docs/RUST.md, "Handles"), made with the host's ports, so every dispatch
//! is one call. The registry a host dispatches crosses by serial (docs/RUST.md, "The host
//! session"): the edge numbers each registered handler by identity, and sends an entry only when
//! the engine does not hold it from the dispatch before, so an unchanged registry crosses as
//! numbers and a `once` claim is told apart by its serial.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, PoisonError};

use serde::Deserialize;

use super::dispatch::config::{Cap, DispatchConfig, DispatchRequest, Stage};
use super::dispatch::dispatch_event;
use super::output::stage::{OutputStage, OutputStageOptions, create_output_stage};
use super::select::once::{Claims, OnceClaims};
use super::verdict::stop_block_cap::{StopBlockCap, create_stop_block_cap};
use crate::js::{JsObject, JsValue};
use crate::registry::{Callback, Handle, Own, Port};

/// The output stage the host gave: the core's own or its object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StageArg {
    Core(Own<OutputStage>),
    Host(Port),
}

/// The Stop counter the host gave: the core's own or its object.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CapArg {
    Core(Own<StopBlockCap>),
    Host(Port),
}

/// What the engine cannot read off a port (`HookEngineOptions`), but its two functions,
/// `userConfigOf` and `ifEvaluator`, which cross on their own ([`HostFunctions`]).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEngineOptions {
    #[serde(default)]
    output: Option<StageArg>,
    #[serde(default)]
    windows: Option<bool>,
    #[serde(default)]
    default_shell: Option<String>,
    #[serde(default)]
    start_directory: Option<String>,
    #[serde(default)]
    temp_directory: Option<String>,
    #[serde(default)]
    http_policy: Option<JsValue>,
    #[serde(default)]
    process_id: Option<f64>,
    #[serde(default)]
    hipaa: Option<bool>,
    #[serde(default)]
    settings_env: Option<JsValue>,
    #[serde(default)]
    env_files: Option<Port>,
    #[serde(default)]
    stop_cap: Option<CapArg>,
    #[serde(default)]
    check_plugin_directories: Option<bool>,
}

/// The ports the run stage reads (`runPortsOf`): only the ones the host gave, `commands` as the
/// runner.
fn run_ports_of(ports: &JsValue) -> JsValue {
    let mut run = JsObject::new();
    let renamed = [
        ("commands", "runner"),
        ("locations", "locations"),
        ("environment", "environment"),
        ("observer", "observer"),
        ("http", "http"),
        ("mcp", "mcp"),
        ("models", "models"),
        ("clock", "clock"),
    ];
    for (from, to) in renamed {
        if let Some(port) = ports.get(from).filter(|port| !port.is_undefined()) {
            run.insert(to.to_owned(), port.clone());
        }
    }
    JsValue::Object(run)
}

fn port_of(ports: &JsValue, name: &str) -> Option<Port> {
    Port::deserialize(ports.get(name)?.clone()).ok()
}

/// What the host sets about its machine, as each run context carries it; each only when set.
fn run_options_of(options: &HookEngineOptions, user_config_of: Option<Callback>) -> JsObject {
    let mut run = JsObject::new();
    let mut set = |name: &str, value: Option<JsValue>| {
        if let Some(value) = value {
            run.insert(name.to_owned(), value);
        }
    };
    set(
        "startDirectory",
        options.start_directory.clone().map(JsValue::from),
    );
    set(
        "tempDirectory",
        options.temp_directory.clone().map(JsValue::from),
    );
    set("processId", options.process_id.map(JsValue::Number));
    set(
        "hipaa",
        (options.hipaa == Some(true)).then_some(JsValue::Bool(true)),
    );
    set("settingsEnv", options.settings_env.clone());
    let user_config_of =
        user_config_of.and_then(|callback| crate::js::value::to_js_value(&callback).ok());
    set("userConfigOf", user_config_of);
    set("httpPolicy", options.http_policy.clone());
    let env_files = options
        .env_files
        .and_then(|files| crate::js::value::to_js_value(&files).ok());
    set("envFiles", env_files);
    let check = options.check_plugin_directories != Some(false);
    set("checkPluginDirectory", check.then_some(JsValue::Bool(true)));
    run
}

/// The functions among the options, each the host's.
#[derive(Clone, Copy, Debug, Default)]
pub struct HostFunctions {
    /// `userConfigOf`.
    pub user_config_of: Option<Callback>,
    /// `ifEvaluator`.
    pub if_evaluator: Option<Callback>,
}

/// The engine of `createHookEngine`.
#[derive(Debug)]
pub struct HookEngine {
    /// What its dispatches read.
    config: DispatchConfig,
    /// The entries of the registry dispatched last, by serial.
    held: Mutex<BTreeMap<u32, JsValue>>,
    /// The `once` handlers its dispatches hold and spent.
    claims: Arc<OnceClaims>,
}

/// A registry as the edge sends it: its serials in order, and the entries the engine does not
/// hold yet.
#[derive(Debug, Deserialize)]
pub struct SentRegistry {
    serials: Vec<u32>,
    #[serde(default)]
    fresh: Vec<(u32, JsValue)>,
}

impl HookEngine {
    /// What its dispatches read: a session's dispatch reads it too (`host/session_engine.rs`).
    #[must_use]
    pub const fn config(&self) -> &DispatchConfig {
        &self.config
    }

    /// The registry's entries under their serials; the engine keeps only these.
    fn entries_of(&self, sent: SentRegistry) -> Vec<(u32, JsValue)> {
        let mut held = self.held.lock().unwrap_or_else(PoisonError::into_inner);
        held.extend(sent.fresh);
        let kept: BTreeSet<u32> = sent.serials.iter().copied().collect();
        held.retain(|serial, _| kept.contains(serial));
        sent.serials
            .iter()
            .filter_map(|serial| Some((*serial, held.get(serial)?.clone())))
            .collect()
    }

    /// One dispatch of `registry`.
    ///
    /// # Errors
    ///
    /// The facts are not an input the event's schema reads.
    pub async fn dispatch(
        &self,
        registry: SentRegistry,
        request: DispatchRequest,
    ) -> Result<JsObject, String> {
        let entries = self.entries_of(registry);
        let claims: Arc<dyn Claims> = Arc::clone(&self.claims) as Arc<dyn Claims>;
        dispatch_event(entries, request, &self.config, &claims, true).await
    }
}

crate::registry::methods! {
    HookEngine {
        "dispatch" => async fallible |this, registry: SentRegistry, request: DispatchRequest|
            this.dispatch(registry, request).await;
    }
}

/// The engine over the real stages: select, build, run through the ports, read, merge.
///
/// # Errors
///
/// `ports` has no environment port.
pub fn create_hook_engine(
    ports: &JsValue,
    options: HookEngineOptions,
    functions: HostFunctions,
) -> Result<HookEngine, &'static str> {
    let scratch = port_of(ports, "scratch");
    let run_options = run_options_of(&options, functions.user_config_of);
    let output = match options.output {
        Some(StageArg::Core(stage)) => Stage::Core(*stage),
        Some(StageArg::Host(stage)) => Stage::Host(stage),
        None => Stage::Core(create_output_stage(OutputStageOptions { scratch })),
    };
    let stop_cap = match options.stop_cap {
        Some(CapArg::Core(cap)) => Cap::Core(cap.0),
        Some(CapArg::Host(cap)) => Cap::Host(cap),
        None => Cap::Core(Arc::new(create_stop_block_cap())),
    };
    let environment =
        port_of(ports, "environment").ok_or("createHookEngine: ports.environment is not a port")?;
    let config = DispatchConfig {
        output,
        run_ports: run_ports_of(ports),
        environment,
        clock: port_of(ports, "clock"),
        observer: port_of(ports, "observer"),
        if_evaluator: functions.if_evaluator,
        model_catalog: port_of(ports, "modelCatalog"),
        windows: options.windows.unwrap_or(false),
        default_shell: options.default_shell.unwrap_or_else(|| "bash".to_owned()),
        run_options,
        env_files: options.env_files,
        stop_cap: Some(stop_cap),
        scratch,
    };
    Ok(HookEngine {
        config,
        held: Mutex::new(BTreeMap::new()),
        claims: Arc::new(OnceClaims::default()),
    })
}

crate::registry::exports! {
    "hooks/engine" {
        "createHookEngine" => fallible |ports: JsValue, options: Option<HookEngineOptions>,
            user_config_of: Option<Callback>, if_evaluator: Option<Callback>|
            create_hook_engine(&ports, options.unwrap_or_default(), HostFunctions {
                user_config_of,
                if_evaluator,
            })
            .map(Handle::own);
    }
}
