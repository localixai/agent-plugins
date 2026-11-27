//! Port of `src/rules/tools/semantics/read-limits.ts`, switched.
//!
//! What a Read call returns at most, and the files it refuses before it reads a byte
//! (tools-reference.md, "Read tool behavior", "Large files").

use serde::{Deserialize, Serialize};

use super::env_number::positive_integer;
use crate::js::number_to_string;
use crate::ports::environment::Environment;
use crate::registry::Port;

/// tools-reference.md, "Large files": "at most 25,000 tokens" and "refuses a whole file over 256
/// KB".
const DEFAULT_TOKEN_LIMIT: f64 = 25_000.0;
const WHOLE_FILE_BYTES: f64 = 256.0 * 1024.0;
const MEGABYTE: f64 = 1024.0 * 1024.0;

/// "Images: Read refuses an image file larger than 128 MB"; "a notebook file over 100 MB".
const IMAGE_BYTES: f64 = 128.0 * MEGABYTE;
const NOTEBOOK_BYTES: f64 = 100.0 * MEGABYTE;

/// What one Read call returns at most, in tokens: 25,000, or
/// `CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS` (env-vars.md: "Override the default token limit for
/// file reads"). A value that is not a positive integer counts as unset (spec/judgments.md).
pub async fn read_token_limit(env: &dyn Environment) -> f64 {
    positive_integer(env, "CLAUDE_CODE_FILE_READ_MAX_OUTPUT_TOKENS")
        .await
        .unwrap_or(DEFAULT_TOKEN_LIMIT)
}

/// A file Read is asked for.
#[derive(Clone, Debug, Deserialize)]
pub struct ReadTarget {
    /// Its path.
    pub path: String,
    /// Its size.
    pub bytes: f64,
}

/// Why Read refuses a file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RefusalReason {
    /// `binary-extension`.
    BinaryExtension,
    /// `image-too-large`.
    ImageTooLarge,
    /// `notebook-too-large`.
    NotebookTooLarge,
}

/// A refusal and its message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ReadRefusal {
    /// Why.
    pub reason: RefusalReason,
    /// What Claude is told.
    pub message: &'static str,
}

/// The extensions the docs name as binary ("such as `.xlsx`, `.parquet`, `.heic`, or `.zip`").
const BINARY_EXTENSIONS: [&str; 4] = ["xlsx", "parquet", "heic", "zip"];
/// The image types Read returns as visual content (tools-reference.md, "Read tool behavior").
const IMAGE_EXTENSIONS: [&str; 5] = ["png", "jpg", "jpeg", "gif", "webp"];

fn extension_of(path: &str) -> String {
    let start = path.rfind(['/', '\\']).map_or(0, |at| at + 1);
    let name = path.get(start..).unwrap_or_default();
    name.rfind('.').map_or_else(String::new, |at| {
        name.get(at + 1..).unwrap_or_default().to_lowercase()
    })
}

/// A refusal and the files it applies to.
struct RefusalRule {
    refusal: ReadRefusal,
    applies: fn(&str, f64) -> bool,
}

const REFUSALS: [RefusalRule; 3] = [
    RefusalRule {
        refusal: ReadRefusal {
            reason: RefusalReason::BinaryExtension,
            message: "Read cannot read a file of this binary type: use a skill for the format, or a shell command or script that reads it",
        },
        applies: |extension, _| BINARY_EXTENSIONS.contains(&extension),
    },
    RefusalRule {
        refusal: ReadRefusal {
            reason: RefusalReason::ImageTooLarge,
            message: "The image is over 128 MB: resize it or export a smaller copy, then read that copy",
        },
        applies: |extension, bytes| IMAGE_EXTENSIONS.contains(&extension) && bytes > IMAGE_BYTES,
    },
    RefusalRule {
        refusal: ReadRefusal {
            reason: RefusalReason::NotebookTooLarge,
            message: "The notebook is over 100 MB: read a portion of it, such as a slice of cells, with a shell command",
        },
        applies: |extension, bytes| extension == "ipynb" && bytes > NOTEBOOK_BYTES,
    },
];

/// Whether Read refuses a file before it reads a byte (tools-reference.md, "Read tool behavior"):
/// an extension it treats as binary, an image over 128 MB, a notebook over 100 MB. Megabytes are
/// 1024 x 1024 bytes and only the four extensions the docs name are modelled (spec/judgments.md);
/// the wording of the refusals is a judgment too.
#[must_use]
pub fn classify_read_target(target: &ReadTarget) -> Option<ReadRefusal> {
    let extension = extension_of(&target.path);
    REFUSALS
        .iter()
        .find(|rule| (rule.applies)(&extension, target.bytes))
        .map(|rule| rule.refusal)
}

/// How a text read is sized.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadSizing {
    /// The size of the text file on disk.
    pub bytes: f64,
    /// What the text to return comes to, in tokens: the whole file, or the `offset` and `limit`
    /// window.
    pub tokens: f64,
    /// The call passes `offset` or `limit`.
    pub windowed: bool,
    /// The call passes `allow_large: true`.
    pub allow_large: bool,
    /// The room left in the context window, in tokens; `allow_large` is sized against it.
    #[serde(default)]
    pub context_room_tokens: Option<f64>,
}

/// `whole`: all of it; `partial`: the first page and a `PARTIAL view` notice; `refused`: an error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ReadOutcome {
    /// `whole`.
    Whole,
    /// `partial`.
    Partial,
    /// `refused`.
    Refused,
}

/// What a text read returns (tools-reference.md, "Large files"). Over the default limits: a whole
/// file over 256 KB is refused, a whole file over the token limit gives its first page with a
/// `PARTIAL view` notice, and an `offset` or `limit` read over the token limit is refused. With
/// `allow_large` the read "is sized against the room left in the session's context window instead
/// of the default limits"; over that room it is refused (spec/judgments.md: the docs do not say
/// what a read over the room gets). Without a measured room `allow_large` has no effect.
pub async fn classify_read_size(sizing: ReadSizing, env: &dyn Environment) -> ReadOutcome {
    if let (true, Some(room)) = (sizing.allow_large, sizing.context_room_tokens) {
        return if sizing.tokens <= room {
            ReadOutcome::Whole
        } else {
            ReadOutcome::Refused
        };
    }
    if !sizing.windowed && sizing.bytes > WHOLE_FILE_BYTES {
        return ReadOutcome::Refused;
    }
    if sizing.tokens <= read_token_limit(env).await {
        return ReadOutcome::Whole;
    }
    if sizing.windowed {
        ReadOutcome::Refused
    } else {
        ReadOutcome::Partial
    }
}

/// The notice of a partial view: "how much of the file it received and how to read more with
/// `offset` and `limit`". It always starts with `PARTIAL view`; the rest is a judgment.
#[must_use]
pub fn partial_view_notice(shown_lines: f64, total_lines: f64) -> String {
    format!(
        "PARTIAL view: showing the first {} of {} lines; read more with offset and limit.",
        number_to_string(shown_lines),
        number_to_string(total_lines)
    )
}

/// The error of a read that is too large: a portion with `offset` and `limit`, or Grep (wording
/// is a judgment).
pub const READ_TOO_LARGE_MESSAGE: &str = "The file is too large to read in one call: read a portion with offset and limit, or search for specific content with Grep instead.";

crate::registry::exports! {
    "rules/tools/semantics/read-limits" {
        "readTokenLimit" => async ok |env: Port| read_token_limit(&env).await;
        "classifyReadTarget" => ok |target: ReadTarget| classify_read_target(&target);
        "classifyReadSize" => async ok |sizing: ReadSizing, env: Port| classify_read_size(sizing, &env).await;
        "partialViewNotice" => ok |shown_lines: f64, total_lines: f64| partial_view_notice(shown_lines, total_lines);
        "READ_TOO_LARGE_MESSAGE" => ok || READ_TOO_LARGE_MESSAGE;
    }
}

#[cfg(test)]
mod tests {
    use super::extension_of;

    #[test]
    fn the_extension_is_after_the_last_dot_of_the_name() {
        assert_eq!(extension_of("a/b.c/D.ZIP"), "zip");
        assert_eq!(extension_of("a.b\\c"), "");
        assert_eq!(extension_of("x."), "");
    }
}
