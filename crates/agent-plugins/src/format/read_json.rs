//! Port of `src/format/read-json.ts`, switched.
//!
//! Reading a JSON file of a plugin: the bytes come through the `PluginFiles` port, and the core
//! parses them.

use crate::js::JsValue;
use crate::js::json_parse::json_parse;
use crate::ports::files::PluginFiles;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;
use serde::Serialize;

/// A JSON file's value, or the reason it cannot be used; `missing` when there was no text to
/// parse.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JsonRead {
    /// The parsed value.
    Value {
        /// What the text holds.
        value: JsValue,
    },
    /// Why there is no value.
    Problem {
        /// The parser's reason, or that the file could not be read.
        problem: String,
        /// Set when there was no text to parse.
        #[serde(skip_serializing_if = "std::ops::Not::not")]
        missing: bool,
    },
}

/// The value of a JSON text, or the parser's reason it is not JSON; never a panic.
#[must_use]
pub fn parse_json(text: &str) -> JsonRead {
    match json_parse(text) {
        Ok(value) => JsonRead::Value { value },
        Err(problem) => JsonRead::Problem {
            problem,
            missing: false,
        },
    }
}

/// The message a file that is no JSON gives (plugins/troubleshooting.md;
/// plugins/create-marketplace.md): `Invalid JSON syntax: <reason>`.
#[must_use]
pub fn invalid_json_syntax(problem: &str) -> String {
    format!("Invalid JSON syntax: {problem}")
}

fn unreadable() -> JsonRead {
    JsonRead::Problem {
        problem: "the file could not be read".to_owned(),
        missing: true,
    }
}

/// Reads and parses one file of the plugin; a missing file or invalid JSON is a `problem`, never
/// a panic.
pub fn read_json<'a>(
    files: &'a dyn PluginFiles,
    path: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, JsonRead> {
    Box::pin(async move {
        match files.read_text(path, signal).await {
            None => unreadable(),
            Some(text) => parse_json(&text),
        }
    })
}

/// What Claude Code's JSON parser says of a file that opens with a UTF-8 byte order mark.
pub const BOM_PARSE_ERROR: &str = "JSON Parse error: Unrecognized token ''";

const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];

/// Whether the bytes open with a UTF-8 byte order mark.
#[must_use]
pub fn opens_with_bom(bytes: &[u8]) -> bool {
    BOM.iter()
        .enumerate()
        .all(|(index, byte)| bytes.get(index) == Some(byte))
}

/// Like `read_json`, but read as bytes so a byte order mark `read_text` may drop is seen: a file
/// that opens with one is not JSON (`BOM_PARSE_ERROR`). For `hooks/hooks.json` and the hooks
/// files a manifest names, which load no hooks with a BOM (spec §2.1 item 2, J2.3; load oracle
/// 2.1.319 and 2.1.322: Hooks (0)). One read either way.
pub fn read_json_without_bom<'a>(
    files: &'a dyn PluginFiles,
    path: &'a str,
    signal: Option<&'a Signal>,
) -> BoxFuture<'a, JsonRead> {
    Box::pin(async move {
        let Some(bytes) = files.read_bytes(path, signal).await else {
            return unreadable();
        };
        if opens_with_bom(&bytes) {
            return JsonRead::Problem {
                problem: BOM_PARSE_ERROR.to_owned(),
                missing: false,
            };
        }
        parse_json(&String::from_utf8_lossy(&bytes))
    })
}

crate::registry::exports! {
    "format/read-json" {
        "readJson" => async ok |files: Port, path: String, signal: Option<Signal>|
            read_json(&files, &path, signal.as_ref()).await;
        "readJsonWithoutBom" => async ok |files: Port, path: String, signal: Option<Signal>|
            read_json_without_bom(&files, &path, signal.as_ref()).await;
    }
}
