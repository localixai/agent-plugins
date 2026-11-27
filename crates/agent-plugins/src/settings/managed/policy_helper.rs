//! Port of `src/settings/managed/policy-helper.ts`, switched.
//!
//! `policyHelper`: an executable that computes managed settings at startup (settings-reference.md,
//! `policyHelper`, "Write the helper output", "Helper failures", `policyHelper.path`,
//! `policyHelper.timeoutMs`, `policyHelper.refreshIntervalMs`; managed-settings.md, "Compute the
//! policy with a helper program"). This module judges the key and the helper's output; running the
//! process is `helper-run.ts`, through the command port.

use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::json_parse::json_parse;
use crate::js::{JsValue, is_integer, number_to_string};
use crate::settings::layers::PolicyScope;
use crate::settings::read_layer::read_settings_layer;

/// The timeout when the key sets none.
pub const HELPER_DEFAULT_TIMEOUT_MS: f64 = 10_000.0;
/// The least timeout the key may set.
pub const HELPER_MIN_TIMEOUT_MS: f64 = 1000.0;
/// The least refresh interval the key may set, but `0`.
pub const HELPER_MIN_REFRESH_MS: f64 = 60_000.0;
/// "Reads a JSON envelope from stdout, capped at 1 MiB"; a helper that writes more to stdout or to stderr fails.
pub const HELPER_OUTPUT_CAP_BYTES: usize = 1024 * 1024;

/// The SDK's `policyHelper` with the timeout filled in: `refreshIntervalMs` stays unset until the settings say.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicyHelper {
    /// The executable.
    pub path: String,
    /// The time budget of a run, in milliseconds.
    pub timeout_ms: f64,
    /// How often the helper runs again; `0` never.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_interval_ms: Option<f64>,
}

/// The key read: the helper, or the issue that drops it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HelperKey {
    /// The helper.
    Helper {
        /// The helper.
        helper: PolicyHelper,
    },
    /// Why it is dropped.
    Issue {
        /// The issue.
        issue: PluginIssue,
    },
}

fn integer(value: Option<&JsValue>) -> Option<f64> {
    value
        .and_then(JsValue::as_f64)
        .filter(|number| is_integer(*number))
}

/// Why the `policyHelper` value is invalid, or `None` when it is a valid object.
fn invalid_reason(value: &JsValue) -> Option<String> {
    let JsValue::Object(record) = value else {
        return Some("must be an object with a path, not a bare path".to_owned());
    };
    if !record.get("path").is_some_and(JsValue::is_string) {
        return Some("needs a \"path\" text".to_owned());
    }
    let timeout_ms = record
        .get("timeoutMs")
        .filter(|value| !value.is_undefined());
    let refresh_interval_ms = record
        .get("refreshIntervalMs")
        .filter(|value| !value.is_undefined());
    if timeout_ms.is_some() && !integer(timeout_ms).is_some_and(|ms| ms >= HELPER_MIN_TIMEOUT_MS) {
        return Some(format!(
            "\"timeoutMs\" must be an integer of at least {}",
            number_to_string(HELPER_MIN_TIMEOUT_MS)
        ));
    }
    let refresh_ok = refresh_interval_ms.is_none()
        || integer(refresh_interval_ms).is_some_and(|ms| ms == 0.0 || ms >= HELPER_MIN_REFRESH_MS);
    if refresh_ok {
        None
    } else {
        Some(format!(
            "\"refreshIntervalMs\" must be 0 or an integer of at least {}",
            number_to_string(HELPER_MIN_REFRESH_MS)
        ))
    }
}

/// Reads the key's value. An invalid value, such as a bare path or a `timeoutMs` below its minimum,
/// is "a dropped entry": the session starts on the remaining managed settings without a helper.
#[must_use]
pub fn read_policy_helper(value: &JsValue) -> HelperKey {
    let reason = invalid_reason(value);
    let path = value.get("path").and_then(JsValue::as_str);
    let (None, Some(path)) = (&reason, path) else {
        let reason = reason.as_deref().unwrap_or("is invalid");
        return HelperKey::Issue {
            issue: warning(
                "managed:policyHelper",
                &format!("\"policyHelper\" {reason}; the entry is dropped and no helper runs"),
            ),
        };
    };
    HelperKey::Helper {
        helper: PolicyHelper {
            path: path.to_owned(),
            timeout_ms: integer(value.get("timeoutMs")).unwrap_or(HELPER_DEFAULT_TIMEOUT_MS),
            refresh_interval_ms: integer(value.get("refreshIntervalMs")),
        },
    }
}

/// The OS families `policyHelper.path` is judged for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PathFlavor {
    /// macOS and Linux.
    Posix,
    /// Windows.
    Windows,
}

/// The flavors.
pub const PATH_FLAVORS: [PathFlavor; 2] = [PathFlavor::Posix, PathFlavor::Windows];

/// `/^\.{1,2}$/`.
fn is_dot_segment(segment: &str) -> bool {
    segment == "." || segment == ".."
}

/// "An absolute path in normalized form, without `.` or `..` segments."
fn posix_fault(path: &str) -> Option<&'static str> {
    let Some(rest) = path.strip_prefix('/') else {
        return Some("is not an absolute path");
    };
    let segments: Vec<&str> = rest.split('/').collect();
    if segments.iter().any(|segment| is_dot_segment(segment)) {
        return Some("has a \".\" or \"..\" segment");
    }
    if segments.iter().any(|segment| segment.is_empty()) && path != "/" {
        return Some("is not in normalized form");
    }
    None
}

static WINDOWS_ROOT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^(?:[A-Za-z]:[\\/]|\\\\[^\\/]+[\\/][^\\/]+[\\/])").ok());

/// "On Windows, a drive-letter or UNC path that ends in `.exe`."
fn windows_fault(path: &str) -> Option<&'static str> {
    let Some(root) = WINDOWS_ROOT.as_ref().and_then(|root| root.find(path)) else {
        return Some("is not a drive-letter or UNC path");
    };
    if !path.to_lowercase().ends_with(".exe") {
        return Some("does not end in .exe");
    }
    let segments: Vec<&str> = path
        .get(root.end()..)
        .unwrap_or_default()
        .split(['\\', '/'])
        .collect();
    if segments.iter().any(|segment| is_dot_segment(segment)) {
        return Some("has a \".\" or \"..\" segment");
    }
    segments
        .iter()
        .any(|segment| segment.is_empty())
        .then_some("is not in normalized form")
}

/// Why `path` breaks the rules of `policyHelper.path`; a run with such a path fails.
#[must_use]
pub fn helper_path_fault(path: &str, flavor: PathFlavor) -> Option<&'static str> {
    match flavor {
        PathFlavor::Posix => posix_fault(path),
        PathFlavor::Windows => windows_fault(path),
    }
}

/// A settings repair that left something unreadable behind: the key ignored for
/// its shape, not one entry skipped, and not a key this source simply does not carry (one it "is not
/// read from", or a global config key). `UNREPAIRED` is `/(?:; the key is ignored|was present but invalid)/`.
const UNREPAIRED: [&str; 2] = ["; the key is ignored", "was present but invalid"];
const NOT_CARRIED: [&str; 2] = [" is not read from ", " is a global config key "];

fn is_violation(message: &str) -> bool {
    UNREPAIRED.iter().any(|marker| message.contains(marker))
        && !NOT_CARRIED.iter().any(|marker| message.contains(marker))
}

/// What the helper's output comes to.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum HelperOutput {
    /// The emitted settings; `undefined` when the envelope omits them.
    Settings {
        /// The settings, left out when `undefined`.
        #[serde(skip_serializing_if = "JsValue::is_undefined")]
        settings: JsValue,
    },
    /// Why the run fails.
    Failure {
        /// The reason.
        failure: String,
    },
}

/// The `managedSettings` of an envelope that holds it, read as managed settings are, or why the run fails.
fn read_envelope(envelope: &JsValue) -> HelperOutput {
    let emitted = envelope.get("managedSettings").cloned().unwrap_or_default();
    if emitted.is_undefined() {
        return HelperOutput::Settings {
            settings: JsValue::Undefined,
        };
    }
    if !emitted.is_object() {
        return HelperOutput::Failure {
            failure: "\"managedSettings\" is not a JSON object".to_owned(),
        };
    }
    let read = read_settings_layer(PolicyScope::Managed, &emitted, Some("policyHelper"));
    match read
        .issues
        .iter()
        .find(|issue| is_violation(&issue.message))
    {
        None => HelperOutput::Settings {
            settings: read.layer.settings,
        },
        Some(left) => HelperOutput::Failure {
            failure: format!(
                "\"managedSettings\" has a schema violation that cannot be repaired: {}",
                left.message
            ),
        },
    }
}

const NOT_ONE_OBJECT: &str = "stdout is not a single JSON object";

/// What the helper's output comes to. A bare settings object with no `managedSettings` key "parses
/// with `managedSettings` undefined and applies nothing, and Claude Code reports no error". A schema
/// violation that survives the entry repairs fails the run: the helper's settings are held to a
/// stricter standard than a file's (managed-settings.md, "Find entries Claude Code dropped").
#[must_use]
pub fn read_helper_output(stdout: &str, stderr: &str) -> HelperOutput {
    if stdout.len() > HELPER_OUTPUT_CAP_BYTES || stderr.len() > HELPER_OUTPUT_CAP_BYTES {
        return HelperOutput::Failure {
            failure: "the helper wrote more than 1 MiB to stdout or stderr".to_owned(),
        };
    }
    match json_parse(stdout) {
        Ok(envelope) if envelope.is_object() => read_envelope(&envelope),
        _ => HelperOutput::Failure {
            failure: NOT_ONE_OBJECT.to_owned(),
        },
    }
}

crate::registry::exports! {
    "settings/managed/policy-helper" {
        "HELPER_DEFAULT_TIMEOUT_MS" => ok || HELPER_DEFAULT_TIMEOUT_MS;
        "HELPER_MIN_TIMEOUT_MS" => ok || HELPER_MIN_TIMEOUT_MS;
        "HELPER_MIN_REFRESH_MS" => ok || HELPER_MIN_REFRESH_MS;
        "HELPER_OUTPUT_CAP_BYTES" => ok || HELPER_OUTPUT_CAP_BYTES;
        "readPolicyHelper" => ok |value: JsValue| read_policy_helper(&value);
        "PATH_FLAVORS" => ok || PATH_FLAVORS;
        "helperPathFault" => ok |path: String, flavor: PathFlavor| helper_path_fault(&path, flavor);
        "readHelperOutput" => ok |stdout: String, stderr: String| read_helper_output(&stdout, &stderr);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_paths() {
        assert_eq!(
            helper_path_fault(r"C:\tools\helper.exe", PathFlavor::Windows),
            None
        );
        assert_eq!(
            helper_path_fault(r"\\server\share\h.EXE", PathFlavor::Windows),
            None
        );
        assert_eq!(
            helper_path_fault(r"C:\tools\..\h.exe", PathFlavor::Windows),
            Some("has a \".\" or \"..\" segment")
        );
        assert_eq!(
            helper_path_fault("/usr/bin/h", PathFlavor::Windows),
            Some("is not a drive-letter or UNC path")
        );
    }
}
