//! Port of `src/format/mcp-output.ts`, switched.
//!
//! How large an MCP tool result may be and what happens to one that is too large (mcp.md, "MCP output
//! limits and warnings", "Raise the limit for a specific tool", "Images in tool results"; env-vars.md,
//! `MAX_MCP_OUTPUT_TOKENS`). Pure over data: saving a file, showing a warning and counting tokens with
//! a real tokenizer are the host's, handed in or done with the verdict.
//!
//! The limits hold the host's `countTokens` function, so a record of them crosses as a port whose
//! members are read (docs/RUST.md, "Effects"); `mcp_output_limits` hands the function back as it
//! was given.

use serde::{Deserialize, Serialize};

use super::env_number::positive_whole_env as positive_whole;
use super::mcp_knobs::EnvLookup;
use crate::js::JsValue;
use crate::js::value::serialize_number;
use crate::registry::{Callback, Port};

const MCP_DEFAULT_MAX_OUTPUT_TOKENS: f64 = 25_000.0;
/// "Claude Code displays a warning when any MCP tool output exceeds 10,000 tokens."
const MCP_OUTPUT_WARNING_TOKENS: f64 = 10_000.0;
/// "Saves a successful result with no image content to a file once it's longer than 50,000 characters."
const MCP_TEXT_PERSIST_CHARS: f64 = 50_000.0;

const MIB: f64 = 1024.0 * 1024.0;
/// "Passes 16 MiB" (HTTP and SSE: one response body or one event) and "one message over 16 MiB" (WebSocket).
const MCP_MESSAGE_BYTES: f64 = 16.0 * MIB;

/// "Error text longer than about 11,000 characters keeps only its first 5,000 and last 5,000 characters."
const ERROR_TEXT_LIMIT: usize = 11_000;
const ERROR_TEXT_KEPT: usize = 5_000;

/// The limits for one tool.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpOutputLimits {
    /// `MAX_MCP_OUTPUT_TOKENS`, or 25,000.
    #[serde(serialize_with = "serialize_number")]
    pub max_tokens: f64,
    /// The token count past which a result warns.
    #[serde(serialize_with = "serialize_number")]
    pub warn_tokens: f64,
    /// The character threshold for text: the tool's own `anthropic/maxResultSizeChars`, or 50,000.
    #[serde(serialize_with = "serialize_number")]
    pub text_chars: f64,
    /// The tool declares its own limit: "that value instead for text content, regardless of" the variable.
    pub declared: bool,
    /// Counts the tokens of a text: the host's.
    pub count_tokens: Callback,
}

/// An `McpOutputLimits` as it crosses: data, or, as it holds `countTokens` (a function), a port
/// whose members are read.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(untagged)]
pub enum LimitsGiven {
    /// The host's record, holding a function.
    Host(Port),
    /// Plain data.
    Data(McpOutputLimits),
}

impl LimitsGiven {
    /// The limits' members, read from the host's record when it is one.
    async fn read(self) -> McpOutputLimits {
        match self {
            Self::Data(limits) => limits,
            Self::Host(record) => {
                let max_tokens = record.member("maxTokens");
                let warn_tokens = record.member("warnTokens");
                let text_chars = record.member("textChars");
                let declared = record.member("declared");
                let count_tokens = record.member("countTokens");
                McpOutputLimits {
                    max_tokens: max_tokens.await,
                    warn_tokens: warn_tokens.await,
                    text_chars: text_chars.await,
                    declared: declared.await,
                    count_tokens: count_tokens.await,
                }
            }
        }
    }
}

/// `Pick<McpToolHints, 'maxResultSizeChars'>`.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MaxResultSizeHint {
    /// The tool's own `anthropic/maxResultSizeChars`.
    #[serde(default)]
    pub max_result_size_chars: Option<f64>,
}

/// The limits for one tool. "The environment variable applies to tools that don't declare their own
/// limit"; a tool that does uses its value for text, and its images still meet `MAX_MCP_OUTPUT_TOKENS`.
pub async fn mcp_output_limits(
    lookup: &dyn EnvLookup,
    hints: &MaxResultSizeHint,
    count_tokens: Callback,
) -> McpOutputLimits {
    let text = lookup.lookup("MAX_MCP_OUTPUT_TOKENS").await;
    McpOutputLimits {
        max_tokens: positive_whole(text.as_deref()).unwrap_or(MCP_DEFAULT_MAX_OUTPUT_TOKENS),
        warn_tokens: MCP_OUTPUT_WARNING_TOKENS,
        text_chars: hints
            .max_result_size_chars
            .unwrap_or(MCP_TEXT_PERSIST_CHARS),
        declared: hints.max_result_size_chars.is_some(),
        count_tokens,
    }
}

/// What to do with a tool result.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum McpResultVerdict {
    /// The result stays in the conversation.
    #[serde(rename_all = "camelCase")]
    Inline {
        /// The result's tokens.
        #[serde(serialize_with = "serialize_number")]
        tokens: f64,
        /// The output passed the warning threshold.
        warn: bool,
        /// Image content makes the result subject to the token limit and it is over it; the host decides.
        exceeds_token_limit: bool,
    },
    /// The result is saved to a file.
    Persist {
        /// The text to save to a file; the conversation gets `persistedResultMessage(path)` instead.
        text: String,
        /// The result's tokens.
        #[serde(serialize_with = "serialize_number")]
        tokens: f64,
        /// The output passed the warning threshold.
        warn: bool,
    },
    /// An error result, its text bounded.
    Error {
        /// What Claude receives as the tool's error message.
        message: String,
        /// Characters cut out between the kept ends.
        removed: usize,
    },
}

/// A tool result, as its content is read: `isError` and the parts.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpCallToolResult {
    /// The parts of the result.
    pub content: Vec<JsValue>,
    /// Whether the tool reports an error.
    #[serde(default)]
    pub is_error: Option<bool>,
}

/// The text each kind of content part holds; a kind with none holds none. Judgment: an embedded
/// resource's text is text content (the docs speak of "text content"); a binary resource has none.
fn text_of(part: &JsValue) -> Option<&str> {
    match part.get("resource") {
        Some(resource) => resource.get("text").and_then(JsValue::as_str),
        None => part.get("text").and_then(JsValue::as_str),
    }
}

struct ImagePart<'a> {
    data: &'a str,
    mime_type: &'a str,
}

/// Audio parts carry `data` and `mimeType` too: the kind decides, the fields narrow.
fn image_of(part: &JsValue) -> Option<ImagePart<'_>> {
    if part.get("type").and_then(JsValue::as_str) != Some("image") {
        return None;
    }
    Some(ImagePart {
        data: part.get("data")?.as_str()?,
        mime_type: part.get("mimeType")?.as_str()?,
    })
}

fn joined(result: &McpCallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(text_of)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Judgment: the marker's wording; the docs say it "says how many characters were removed".
fn removed_marker(removed: usize) -> String {
    format!("\n[... {removed} characters removed ...]\n")
}

/// The text of an error as Claude receives it: "Error text longer than about 11,000 characters keeps only
/// its first 5,000 and last 5,000 characters". For an error that is not a tool's result (a failed call),
/// so a host bounds both the same way.
#[must_use]
pub fn bounded_mcp_error_text(text: &str) -> String {
    error_verdict(text).0
}

/// Judgment (spec/judgments.md): a cut never leaves half of a character that takes two UTF-16 units.
fn error_verdict(text: &str) -> (String, usize) {
    let units: Vec<u16> = text.encode_utf16().collect();
    if units.len() <= ERROR_TEXT_LIMIT {
        return (text.to_owned(), 0);
    }
    let mut head = units.get(..ERROR_TEXT_KEPT).unwrap_or_default();
    if let Some((last, rest)) = head.split_last()
        && (0xd800..=0xdbff).contains(last)
    {
        head = rest;
    }
    let mut tail = units
        .get(units.len() - ERROR_TEXT_KEPT..)
        .unwrap_or_default();
    if let Some((first, rest)) = tail.split_first()
        && (0xdc00..=0xdfff).contains(first)
    {
        tail = rest;
    }
    let removed = units.len() - head.len() - tail.len();
    let (head, tail) = (
        String::from_utf16_lossy(head),
        String::from_utf16_lossy(tail),
    );
    (format!("{head}{}{tail}", removed_marker(removed)), removed)
}

/// Whether the text is too large to stay in the conversation, for a result with no image content.
fn too_large(text: &str, tokens: f64, limits: &McpOutputLimits) -> bool {
    let length = text.encode_utf16().count() as f64;
    if limits.declared {
        length > limits.text_chars
    } else {
        tokens > limits.max_tokens || length > limits.text_chars
    }
}

/// What to do with a tool result. An error result is exempt from the limits and has its text bounded.
/// A successful one with no image content is saved when it is over the limit; one with image content is
/// never saved ("a successful result with no image content") and reports whether it is over the token
/// limit. The images' data counts towards the tokens (judgment, spec/judgments.md).
pub async fn limit_mcp_result(
    result: &McpCallToolResult,
    limits: &McpOutputLimits,
) -> McpResultVerdict {
    let text = joined(result);
    if result.is_error == Some(true) {
        let (message, removed) = error_verdict(&text);
        return McpResultVerdict::Error { message, removed };
    }
    let images: Vec<ImagePart<'_>> = result.content.iter().filter_map(image_of).collect();
    let mut tokens = limits
        .count_tokens
        .call::<f64>(vec![JsValue::from(text.as_str())])
        .await;
    let mut sum = 0.0;
    for image in &images {
        sum += limits
            .count_tokens
            .call::<f64>(vec![JsValue::from(image.data)])
            .await;
    }
    tokens += sum;
    let warn = tokens > limits.warn_tokens;
    if !images.is_empty() {
        return McpResultVerdict::Inline {
            tokens,
            warn,
            exceeds_token_limit: tokens > limits.max_tokens,
        };
    }
    if too_large(&text, tokens, limits) {
        McpResultVerdict::Persist { text, tokens, warn }
    } else {
        McpResultVerdict::Inline {
            tokens,
            warn,
            exceeds_token_limit: false,
        }
    }
}

/// What replaces a saved result in the conversation: "a message that names the file path" (wording is the kit's).
#[must_use]
pub fn persisted_result_message(path: &str) -> String {
    format!(
        "The result was too large to show here and was saved to {path}. Read that file when you need the content."
    )
}

/// What tells Claude where the original of an image result is saved (wording is the kit's).
#[must_use]
pub fn image_result_note(path: &str) -> String {
    format!(
        "The original image was saved to {path}. Use tools such as Bash to crop, convert or reuse it."
    )
}

/// An image to save beside the inline copy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct McpImageFile {
    /// The image's media type.
    pub mime_type: String,
    /// The file extension of that type.
    pub extension: String,
    /// The base64 data as the server sent it: "the original bytes".
    pub data: String,
}

/// "A PNG, JPEG, GIF, or WebP image": the media types saved, and the extension of each.
const IMAGE_EXTENSIONS: [(&str, &str); 4] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
];

/// The image files to save beside the inline copy. With session persistence off
/// (`--no-session-persistence`, `CLAUDE_CODE_SKIP_PROMPT_HISTORY`) "Claude Code writes no image file".
#[must_use]
pub fn mcp_image_files(result: &McpCallToolResult, persist: bool) -> Vec<McpImageFile> {
    if !persist {
        return Vec::new();
    }
    result
        .content
        .iter()
        .filter_map(image_of)
        .filter_map(|image| {
            let (_, extension) = IMAGE_EXTENSIONS
                .iter()
                .find(|(mime_type, _)| *mime_type == image.mime_type)?;
            Some(McpImageFile {
                mime_type: image.mime_type.to_owned(),
                extension: (*extension).to_owned(),
                data: image.data.to_owned(),
            })
        })
        .collect()
}

/// What a transport does with a message over its bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpOversize {
    /// The request fails.
    FailRequest,
    /// The connection closes.
    CloseConnection,
}

/// A server's transport (`mcp-config.ts`, `McpTransport`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    /// An HTTP server.
    Http,
    /// An SSE server.
    Sse,
    /// A WebSocket server.
    Ws,
    /// A stdio server.
    Stdio,
}

const fn oversize(transport: McpTransport) -> Option<McpOversize> {
    match transport {
        // "Claude Code stops reading it and the request fails."
        McpTransport::Http | McpTransport::Sse => Some(McpOversize::FailRequest),
        // "Claude Code closes the connection, and calls in progress on that server fail."
        McpTransport::Ws => Some(McpOversize::CloseConnection),
        // The docs state no bound for a stdio server's messages.
        McpTransport::Stdio => None,
    }
}

/// What happens to a message of `bytes` (after decompression) from a server of `transport`: `undefined` when it is fine.
#[must_use]
pub fn oversized_mcp_message(transport: McpTransport, bytes: f64) -> Option<McpOversize> {
    if bytes > MCP_MESSAGE_BYTES {
        oversize(transport)
    } else {
        None
    }
}

crate::registry::exports! {
    "format/mcp-output" {
        "mcpOutputLimits" => async ok |lookup: Callback, hints: Option<MaxResultSizeHint>, count_tokens: Callback|
            mcp_output_limits(&lookup, &hints.unwrap_or_default(), count_tokens).await;
        "boundedMcpErrorText" => ok |text: String| bounded_mcp_error_text(&text);
        "limitMcpResult" => async ok |result: McpCallToolResult, limits: LimitsGiven|
            limit_mcp_result(&result, &limits.read().await).await;
        "persistedResultMessage" => ok |path: String| persisted_result_message(&path);
        "imageResultNote" => ok |path: String| image_result_note(&path);
        "mcpImageFiles" => ok |result: McpCallToolResult, persist: bool| mcp_image_files(&result, persist);
        "oversizedMcpMessage" => ok |transport: McpTransport, bytes: f64| oversized_mcp_message(transport, bytes);
    }
}
