//! Port of `src/settings/managed/wsl-chain.ts`, switched.
//!
//! `wslInheritsWindowsSettings` (settings-reference.md, `wslInheritsWindowsSettings`; managed-settings.md,
//! "How Claude Code combines managed sources", "When an admin document counts as present"): which
//! documents a WSL session composes. The host reads the Windows `HKLM` value and the Windows
//! managed settings file (through `C:\Program Files\ClaudeCode`), `/etc/claude-code` and `HKCU`;
//! this decides which of them count. An `AdminDocument` (`admin-source.ts`) is held as the value it
//! is: the answer hands the documents on whole.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, warning};
use crate::js::JsValue;

/// The documents the host read; each an `AdminDocument`, `undefined` when the source has none.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WslDocuments {
    /// The `HKLM` registry value.
    #[serde(default)]
    pub hklm: JsValue,
    /// The managed settings file and drop-ins under `C:\Program Files\ClaudeCode`.
    #[serde(default)]
    pub windows_file: JsValue,
    /// The managed settings file and drop-ins under `/etc/claude-code`.
    #[serde(default)]
    pub etc_file: JsValue,
    /// The `HKCU` registry value.
    #[serde(default)]
    pub hkcu: JsValue,
}

/// The documents a WSL session composes.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WslChain {
    /// The documents that compose, with the sources `composeManaged` ranks them as.
    pub documents: Vec<JsValue>,
    /// The Windows policy chain is on.
    pub chain_on: bool,
    /// What is wrong with the value.
    pub issues: Vec<PluginIssue>,
}

/// What a document says of the key: `on` is the chain, `set` whether it says anything, `invalid` an unreadable value.
#[derive(Clone, Copy)]
struct Flag {
    set: bool,
    on: bool,
    invalid: bool,
}

const UNSET: Flag = Flag {
    set: false,
    on: false,
    invalid: false,
};

/// "Any other value counts as a present admin document with the chain turned on."
const INVALID: Flag = Flag {
    set: true,
    on: true,
    invalid: true,
};

/// "Claude Code reads `true` and `false` with or without quotes and reads `null` as removing the key."
fn boolean_text(value: &JsValue) -> Option<bool> {
    match value {
        JsValue::Bool(read) => Some(*read),
        JsValue::String(text) if text == "true" => Some(true),
        JsValue::String(text) if text == "false" => Some(false),
        JsValue::Undefined
        | JsValue::Null
        | JsValue::Number(_)
        | JsValue::String(_)
        | JsValue::Array(_)
        | JsValue::Object(_) => None,
    }
}

/// A document as the TypeScript's `document?.…` reads it: a present one, or `None`.
fn existing(document: &JsValue) -> Option<&JsValue> {
    (!document.is_undefined()).then_some(document)
}

fn flag_of(document: Option<&JsValue>) -> Flag {
    let value = document
        .and_then(|document| document.get("raw"))
        .and_then(|raw| raw.get("wslInheritsWindowsSettings"));
    let Some(value) = value.filter(|value| !value.is_nullish()) else {
        return UNSET;
    };
    match boolean_text(value) {
        None => INVALID,
        Some(read) => Flag {
            set: true,
            on: read,
            invalid: false,
        },
    }
}

fn is_present(document: Option<&JsValue>) -> bool {
    document.is_some_and(|document| {
        document.get("present").and_then(JsValue::as_bool) == Some(true)
            || flag_of(Some(document)).invalid
    })
}

/// The chain is on when the highest Windows admin document that sets the key turns it on.
fn chain_flag(windows: &[Option<&JsValue>]) -> Flag {
    windows
        .iter()
        .map(|document| flag_of(*document))
        .find(|flag| flag.set)
        .unwrap_or(UNSET)
}

/// The documents a WSL session composes. "Claude Code honors it only when set in the HKLM registry
/// key or in a managed settings file or drop-in under `C:\Program Files\ClaudeCode\`". With the
/// chain on, the Windows documents stand above `/etc/claude-code`, which is read "only when no
/// Windows admin document is present", and `HKCU` joins "only when HKCU also sets the key to `true`".
/// With it off, WSL reads `/etc/claude-code` alone; an `HKLM` value or Windows-folder file that
/// exists but cannot be read keeps `/etc/claude-code` from applying either way.
#[must_use]
pub fn wsl_chain(documents: &WslDocuments) -> WslChain {
    let windows = [existing(&documents.hklm), existing(&documents.windows_file)];
    let flag = chain_flag(&windows);
    let windows_present = windows.iter().any(|document| is_present(*document));
    let windows_unreadable = windows.iter().any(|document| {
        document.is_some_and(|document| {
            document
                .get("unreadable")
                .is_some_and(|cause| !cause.is_undefined())
        })
    });
    let etc = if windows_unreadable || (flag.on && windows_present) {
        None
    } else {
        existing(&documents.etc_file)
    };
    let hkcu_joins = flag.on && flag_of(existing(&documents.hkcu)).on;
    let composed = if flag.on {
        vec![
            existing(&documents.hklm),
            existing(&documents.windows_file),
            etc,
            if hkcu_joins {
                existing(&documents.hkcu)
            } else {
                None
            },
        ]
    } else {
        vec![etc]
    };
    let issues = if flag.invalid {
        vec![warning(
            "managed:wslInheritsWindowsSettings",
            "\"wslInheritsWindowsSettings\" must be true or false; the Windows policy chain is treated as on",
        )]
    } else {
        Vec::new()
    };
    WslChain {
        documents: composed
            .into_iter()
            .flatten()
            .map(|document| {
                if is_present(Some(document)) {
                    present(document)
                } else {
                    document.clone()
                }
            })
            .collect(),
        chain_on: flag.on,
        issues,
    }
}

/// `{ ...document, present: true }`: the member keeps its place when the document has it.
fn present(document: &JsValue) -> JsValue {
    let mut members = document.as_object().cloned().unwrap_or_default();
    members.insert("present".to_owned(), JsValue::Bool(true));
    JsValue::Object(members)
}

crate::registry::exports! {
    "settings/managed/wsl-chain" {
        "wslChain" => ok |documents: WslDocuments| wsl_chain(&documents);
    }
}
