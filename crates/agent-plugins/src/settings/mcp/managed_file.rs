//! Port of `src/settings/mcp/managed-file.ts`, switched.
//!
//! Where Claude Code looks for `managed-mcp.json` (managed-mcp.md, "Deploy managed-mcp.json"): "Linux
//! and WSL" share a row, so the platform union names both and the registry gives them one path.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::format::mcp_config::read_mcp_config;
use crate::format::read_json::{JsonRead, parse_json};
use crate::js::JsObject;
use crate::ports::Signal;
use crate::ports::files::TextFileReader;
use crate::protocol::is_record::is_record;

/// The platforms, in the docs' order.
pub const MANAGED_MCP_PLATFORMS: [&str; 4] = ["macos", "linux", "wsl", "windows"];

/// A platform of `MANAGED_MCP_PLATFORMS`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ManagedMcpPlatform {
    /// macOS.
    Macos,
    /// Linux.
    Linux,
    /// WSL.
    Wsl,
    /// Windows.
    Windows,
}

const LINUX_PATH: &str = "/etc/claude-code/managed-mcp.json";

/// `MANAGED_MCP_FILE_PATHS`.
const fn managed_mcp_file_paths(platform: ManagedMcpPlatform) -> &'static str {
    match platform {
        ManagedMcpPlatform::Macos => "/Library/Application Support/ClaudeCode/managed-mcp.json",
        ManagedMcpPlatform::Linux | ManagedMcpPlatform::Wsl => LINUX_PATH,
        ManagedMcpPlatform::Windows => "C:\\Program Files\\ClaudeCode\\managed-mcp.json",
    }
}

/// Where the file of `platform` is.
#[must_use]
pub const fn managed_mcp_file_path(platform: ManagedMcpPlatform) -> &'static str {
    managed_mcp_file_paths(platform)
}

/// The servers a deployed `managed-mcp.json` defines, already read like a project `.mcp.json`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManagedMcpFile {
    /// The servers by name (`PluginMcpServer`s).
    pub servers: JsObject,
}

/// The file read.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ManagedMcpFileRead {
    /// Undefined when there is no file, or Claude Code "can't read or parse" it: no exclusive control then.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<ManagedMcpFile>,
    /// What was dropped, or why the file takes no control.
    pub issues: Vec<PluginIssue>,
}

const FILE_NAME: &str = "managed-mcp.json";

fn unreadable(reason: &str) -> ManagedMcpFileRead {
    ManagedMcpFileRead {
        file: None,
        issues: vec![error(
            FILE_NAME,
            &format!(
                "The enterprise MCP config could not be read and takes no control of MCP servers: {reason}"
            ),
        )],
    }
}

/// The file's text: "the same format as a project `.mcp.json` file" (a `mcpServers` wrapper; the
/// reader of `.mcp.json` also takes the bare map). Only a file Claude Code "can read and parse" takes
/// exclusive control, so text that is no JSON object is an issue and no file. An entry Claude Code
/// would drop is an issue and the others stay (spec/judgments.md for the wording and the empty file).
#[must_use]
pub fn read_managed_mcp_file(text: Option<&str>) -> ManagedMcpFileRead {
    let Some(text) = text else {
        return ManagedMcpFileRead {
            file: None,
            issues: Vec::new(),
        };
    };
    let value = match parse_json(text) {
        JsonRead::Problem { problem, .. } => return unreadable(&problem),
        JsonRead::Value { value } => value,
    };
    if !is_record(&value) {
        return unreadable("the file must hold a JSON object");
    }
    let read = read_mcp_config(&value, FILE_NAME);
    ManagedMcpFileRead {
        file: Some(ManagedMcpFile {
            servers: read.servers,
        }),
        issues: read.issues,
    }
}

/// `{ signal }` of `loadManagedMcpFile`.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct LoadOptions {
    /// Cancels the read.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// Reads the file of `platform` through the host's reader; a machine without it has no file.
pub async fn load_managed_mcp_file(
    reader: &impl TextFileReader,
    platform: ManagedMcpPlatform,
    options: &LoadOptions,
) -> ManagedMcpFileRead {
    let text = reader
        .read_text(managed_mcp_file_path(platform), options.signal.as_ref())
        .await;
    read_managed_mcp_file(text.as_deref())
}

crate::registry::exports! {
    "settings/mcp/managed-file" {
        "MANAGED_MCP_PLATFORMS" => ok || MANAGED_MCP_PLATFORMS;
        "managedMcpFilePath" => ok |platform: ManagedMcpPlatform| managed_mcp_file_path(platform);
        "readManagedMcpFile" => ok |text: Option<String>| read_managed_mcp_file(text.as_deref());
        "loadManagedMcpFile" => async ok |reader: crate::registry::Port, platform: ManagedMcpPlatform, options: Option<LoadOptions>|
            load_managed_mcp_file(&reader, platform, &options.unwrap_or_default()).await;
    }
}
