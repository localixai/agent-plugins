//! Port of `src/rules/tools/semantics/encoding.ts`, switched.

use serde::{Deserialize, Serialize};

use crate::format::issues::{PluginIssue, error};
use crate::registry::Bytes;

/// errors.md, "File is not valid UTF-8": the message the tool result carries, verbatim.
pub const NOT_UTF8_MESSAGE: &str = "File is not valid UTF-8. It may use a legacy encoding such as Windows-1252, Shift-JIS or GBK, or be binary. This tool saves the whole file as UTF-8, which would replace every byte it cannot decode with U+FFFD. Nothing was written. Make the change with a shell command that reads and writes the file in its own encoding, or ask the user whether to convert the file to UTF-8 first.";

/// A tool that edits a file in place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InPlaceEditTool {
    /// `Edit`.
    Edit,
    /// `NotebookEdit`.
    NotebookEdit,
}

/// Whether a tool that edits in place accepts a file that starts with a little-endian UTF-16
/// byte-order mark (tools-reference.md, "Non-UTF-8 files": "Claude can't use NotebookEdit on a file
/// that isn't valid UTF-8. The same goes for Edit, unless the file starts with a little-endian
/// UTF-16 byte-order mark.").
const fn accepts_utf16le(tool: InPlaceEditTool) -> bool {
    match tool {
        InPlaceEditTool::Edit => true,
        InPlaceEditTool::NotebookEdit => false,
    }
}

/// `new TextDecoder('utf-8', { fatal: true }).decode(bytes)` does not throw.
fn is_valid_utf8(bytes: &[u8]) -> bool {
    std::str::from_utf8(bytes).is_ok()
}

fn starts_with_utf16_le_mark(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xfe])
}

fn refusal(path: &str, message: &str) -> PluginIssue {
    error(path, message)
}

/// The refusal of an Edit or NotebookEdit on a file whose bytes do not decode as UTF-8, or
/// `undefined` when the tool may edit it. The check covers the bytes as a whole: one invalid
/// sequence is enough (errors.md).
#[must_use]
pub fn edit_encoding_issue(bytes: &[u8], tool: InPlaceEditTool) -> Option<PluginIssue> {
    if is_valid_utf8(bytes) {
        return None;
    }
    if accepts_utf16le(tool) && starts_with_utf16_le_mark(bytes) {
        return None;
    }
    Some(refusal("tool_input.file_path", NOT_UTF8_MESSAGE))
}

const REPLACEMENT_CHARACTER: char = '\u{FFFD}';

/// The refusal of a Write that would replace such a file with text holding `U+FFFD`
/// (tools-reference.md, "Non-UTF-8 files": "Claude can still replace such a file with Write, unless
/// the new content contains `U+FFFD`, the character Read shows Claude in place of bytes it can't
/// decode. That guard stops Claude from writing back the garbled text it read."). `existing` is the
/// file's bytes, `undefined` for a file that is not there. "Such a file" is one `editEncodingIssue`
/// refuses for Edit; the wording of this refusal is a judgment (spec/judgments.md).
///
/// A lone surrogate in `content` reads as U+FFFD in the core (docs/RUST.md, "Where the crate
/// answers differently"), so such text is refused here where the TypeScript would write it: the safe side.
#[must_use]
pub fn write_encoding_issue(existing: Option<&[u8]>, content: &str) -> Option<PluginIssue> {
    let existing = existing?;
    edit_encoding_issue(existing, InPlaceEditTool::Edit)?;
    content.contains(REPLACEMENT_CHARACTER).then(|| {
        refusal(
            "tool_input.content",
            "content contains the replacement character U+FFFD, which Read shows in place of bytes it cannot decode; the existing file is not valid UTF-8, so writing that text back would replace it with garbled text",
        )
    })
}

crate::registry::exports! {
    "rules/tools/semantics/encoding" {
        "NOT_UTF8_MESSAGE" => ok || NOT_UTF8_MESSAGE;
        "editEncodingIssue" => ok |bytes: Bytes, tool: InPlaceEditTool| edit_encoding_issue(&bytes.0, tool);
        "writeEncodingIssue" => ok |existing: Option<Bytes>, content: String| write_encoding_issue(existing.as_ref().map(|bytes| bytes.0.as_slice()), &content);
    }
}
