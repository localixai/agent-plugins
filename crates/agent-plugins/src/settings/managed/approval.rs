//! Port of `src/settings/managed/approval.ts`, switched.
//!
//! Which delivered settings need the user's approval before they apply (server-managed-settings.md,
//! "Security approval dialogs" and "Environment variables and the approval dialog"). Pure: the host
//! shows the dialog; the kit says what it must name.

use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

use crate::format::env_flag::{is_falsy_env, is_truthy_env};
use crate::js::{JsValue, Pattern, regex_test, trim};
use crate::protocol::is_record::is_record;

/// What a delivered setting needs approval for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApprovalKind {
    /// A setting that executes a shell command.
    ShellCommand,
    /// A sandbox binary path.
    SandboxBinary,
    /// A sandbox setting that reroutes traffic or weakens isolation.
    SandboxNetworkIsolation,
    /// An environment variable.
    CustomEnv,
    /// A hook.
    Hook,
}

/// One item the dialog names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ApprovalItem {
    /// What kind of setting.
    pub kind: ApprovalKind,
    /// The setting (or `env.NAME`) the dialog names.
    pub key: String,
}

fn item(kind: ApprovalKind, key: String) -> ApprovalItem {
    ApprovalItem { kind, key }
}

/// "settings that execute shell commands, such as `apiKeyHelper`, `statusLine`, and `otelHeadersHelper`".
const SHELL_COMMAND_KEYS: [&str; 3] = ["apiKeyHelper", "statusLine", "otelHeadersHelper"];

/// "`sandbox.bwrapPath`, `sandbox.socatPath`, and `sandbox.ripgrep`".
const SANDBOX_BINARY_KEYS: [&str; 3] = ["bwrapPath", "socatPath", "ripgrep"];

/// The sandbox settings "that let the sandbox proxy read, reroute, or authenticate traffic, or that weaken the sandbox's isolation".
const SANDBOX_ISOLATION_PATHS: [&[&str]; 11] = [
    &["network", "tlsTerminate"],
    &["network", "httpProxyPort"],
    &["network", "socksProxyPort"],
    &["allowAppleEvents"],
    &["allowNestedUserNamespaces"],
    &["enableWeakerNestedSandbox"],
    &["enableWeakerNetworkIsolation"],
    &["filesystem", "disabled"],
    &["network", "allowAllUnixSockets"],
    &["network", "allowUnixSockets"],
    &["network", "allowMachLookup"],
];

fn at<'a>(block: &'a JsValue, path: &[&str]) -> Option<&'a JsValue> {
    path.iter().try_fold(block, |inner, name| {
        if is_record(inner) {
            inner.get(name)
        } else {
            None
        }
    })
}

fn is_set(value: Option<&JsValue>) -> bool {
    value.is_some_and(|value| !value.is_nullish())
}

/// A `sandbox.credentials` block with only `deny` rules "restricts the sandbox without giving the proxy a credential".
fn credentials_need_approval(credentials: Option<&JsValue>) -> bool {
    let Some(members) = credentials.and_then(JsValue::as_object) else {
        return false;
    };
    members.iter().any(|(name, value)| {
        if name != "files" && name != "envVars" {
            return is_set(Some(value));
        }
        value.as_array().is_some_and(|entries| {
            entries
                .iter()
                .any(|entry| at(entry, &["mode"]).and_then(JsValue::as_str) != Some("deny"))
        })
    })
}

fn sandbox_items(sandbox: Option<&JsValue>) -> Vec<ApprovalItem> {
    let Some(sandbox) = sandbox.filter(|sandbox| is_record(sandbox)) else {
        return Vec::new();
    };
    let binaries = SANDBOX_BINARY_KEYS
        .iter()
        .filter(|name| is_set(sandbox.get(name)))
        .map(|name| item(ApprovalKind::SandboxBinary, format!("sandbox.{name}")));
    let isolation = SANDBOX_ISOLATION_PATHS
        .iter()
        .filter(|path| is_set(at(sandbox, path)))
        .map(|path| {
            item(
                ApprovalKind::SandboxNetworkIsolation,
                format!("sandbox.{}", path.join(".")),
            )
        });
    let credentials = credentials_need_approval(sandbox.get("credentials")).then(|| {
        item(
            ApprovalKind::SandboxNetworkIsolation,
            "sandbox.credentials".to_owned(),
        )
    });
    binaries.chain(isolation).chain(credentials).collect()
}

/// Variables that "always" need approval "at any non-empty value": proxy and base-URL variables, the OTLP endpoint, the 1M-context switch.
static ALWAYS: Pattern = LazyLock::new(|| {
    Regex::new(
        r"^(?i-u:(?:(?:HTTPS?|ALL|NO)_PROXY|ANTHROPIC_(?:BEDROCK_|VERTEX_|FOUNDRY_|AWS_)?BASE_URL|OTEL_EXPORTER_OTLP_(?:[A-Za-z0-9_]+_)?ENDPOINT|CLAUDE_CODE_DISABLE_1M_CONTEXT))$",
    )
    .ok()
});
/// Decided by value: a truthy one "only turns tracking, reporting, or other nonessential traffic off".
const PRIVACY_TOGGLES: [&str; 5] = [
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
    "DISABLE_ERROR_REPORTING",
    "DISABLE_TELEMETRY",
    "DO_NOT_TRACK",
    "API_FORCE_IDLE_TIMEOUT",
];
/// A falsy value "only turns detailed tracing or raw API body capture off".
const TRACING_TOGGLES: [&str; 2] = ["ENABLE_BETA_TRACING_DETAILED", "OTEL_LOG_RAW_API_BODIES"];

/// Header names "that name a credential, an org or tenant selector, a routing or host override, or an API-behavior header".
static RISKY_HEADER_WORDS: Pattern = LazyLock::new(|| {
    Regex::new(
        r"(?i-u:auth|api-?key|token|secret|credential|cookie|org|tenant|account|workspace|host|forward|route|proxy|beta|version|client|bedrock|amzn)",
    )
    .ok()
});
static HEADER_TOKEN: Pattern = LazyLock::new(|| Regex::new(r"^[!#$%&'*+\-.^_`|~0-9A-Za-z]+$").ok());
const TAB: u32 = 9;
/// A character an HTTP header value cannot carry: a control character other than the tab, or DEL.
fn cannot_carry(text: &str) -> bool {
    text.chars().any(|character| {
        (u32::from(character) < 32 && u32::from(character) != TAB) || character == '\u{7f}'
    })
}

/// Whether `ANTHROPIC_CUSTOM_HEADERS` lines need approval: tagging headers such as `Accept-Language` do not.
fn headers_need_approval(value: &str) -> bool {
    value
        .split('\n')
        .filter(|line| !trim(line).is_empty())
        .any(|line| {
            let colon = line.find(':');
            let name = colon.map_or_else(|| trim(line), |at| trim(line.get(..at).unwrap_or("")));
            let content = colon.map_or("", |at| line.get(at + 1..).unwrap_or(""));
            colon.is_none()
                || !regex_test(&HEADER_TOKEN, name)
                || cannot_carry(content)
                || regex_test(&RISKY_HEADER_WORDS, name)
        })
}

fn env_needs_approval(name: &str, value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    if PRIVACY_TOGGLES.contains(&name) {
        return !is_truthy_env(Some(value));
    }
    if TRACING_TOGGLES.contains(&name) {
        return !is_falsy_env(Some(value));
    }
    if name == "ANTHROPIC_CUSTOM_HEADERS" {
        return headers_need_approval(value);
    }
    regex_test(&ALWAYS, name)
}

fn env_items(env: Option<&JsValue>) -> Vec<ApprovalItem> {
    let Some(members) = env.and_then(JsValue::as_object) else {
        return Vec::new();
    };
    members
        .iter()
        .filter(|(name, value)| {
            value
                .as_str()
                .is_some_and(|text| env_needs_approval(name, text))
        })
        .map(|(name, _)| item(ApprovalKind::CustomEnv, format!("env.{name}")))
        .collect()
}

/// Any hook definition, in any event.
fn hook_items(hooks: Option<&JsValue>) -> Vec<ApprovalItem> {
    let Some(members) = hooks.and_then(JsValue::as_object) else {
        return Vec::new();
    };
    let defined = members
        .values()
        .any(|groups| groups.as_array().is_some_and(|groups| !groups.is_empty()));
    if defined {
        vec![item(ApprovalKind::Hook, "hooks".to_owned())]
    } else {
        Vec::new()
    }
}

/// The items of delivered settings that need approval, in a stable order. `claudeMd` is not one:
/// "it's instruction text for Claude rather than a command Claude Code runs".
#[must_use]
pub fn approval_items(settings: &JsValue) -> Vec<ApprovalItem> {
    let shell = SHELL_COMMAND_KEYS
        .iter()
        .filter(|name| is_set(settings.get(name)))
        .map(|name| item(ApprovalKind::ShellCommand, (*name).to_owned()));
    shell
        .chain(sandbox_items(settings.get("sandbox")))
        .chain(env_items(settings.get("env")))
        .chain(hook_items(settings.get("hooks")))
        .collect()
}

crate::registry::exports! {
    "settings/managed/approval" {
        "approvalItems" => ok |settings: JsValue| approval_items(&settings);
    }
}
