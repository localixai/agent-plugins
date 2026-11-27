//! The core's counterpart of `src/rules/tools/semantics/bash-output.ts` at sha256:69c03702699d, which stays TypeScript:
//! its work is a pass over a command's output, of any size, which crossing the boundary would
//! copy there and back (docs/RUST.md, "What stays TypeScript").
//! The windows of a command's output (tools-reference.md, "Output limits").

use super::env_number::positive_integer;
use crate::js::{code_unit_length, math_max, math_min, slice_code_units};
use crate::ports::environment::Environment;

/// tools-reference.md, "Output limits": characters.
const INLINE_VALID: usize = 30_000;
const INLINE_FAILURE: usize = 10_000;
const PREVIEW: usize = 2_000;
const READ_BACK_DEFAULT: f64 = 30_000.0;
const READ_BACK_CEILING: f64 = 150_000.0;
/// settings-reference.md, `bashOutputMaxChars`: "Claude Code clamps the value into the range 4000
/// to 128000".
const SETTING_RANGE: (f64, f64) = (4_000.0, 128_000.0);

/// The windows of a command's output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BashOutputLimits {
    /// How much of a valid result reaches Claude inline.
    pub inline: usize,
    /// How much of the output Claude Code reads back from its working file.
    pub read_back: usize,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a whole number of characters, clamped to the ranges above"
)]
const fn characters(value: f64) -> usize {
    value as usize
}

/// The windows of a command's output. `BASH_MAX_OUTPUT_LENGTH` sets the read-back window (30,000
/// by default, 150,000 at most) and "doesn't raise the inline ceilings"; the `bashOutputMaxChars`
/// setting sizes the inline ceiling and the window together, clamped to 4000 to 128000, and then
/// Claude Code ignores the variable. A window below the inline ceiling lowers it too: nothing
/// larger than what was read back can arrive inline (spec/judgments.md).
pub async fn bash_output_limits(
    env: &dyn Environment,
    bash_output_max_chars: Option<f64>,
) -> BashOutputLimits {
    if let Some(chars) = bash_output_max_chars.filter(|chars| chars.fract() == 0.0 && *chars > 0.0)
    {
        let size = characters(math_min(math_max(chars, SETTING_RANGE.0), SETTING_RANGE.1));
        return BashOutputLimits {
            inline: size,
            read_back: size,
        };
    }
    let read_back = math_min(
        positive_integer(env, "BASH_MAX_OUTPUT_LENGTH")
            .await
            .unwrap_or(READ_BACK_DEFAULT),
        READ_BACK_CEILING,
    );
    let read_back = characters(read_back);
    BashOutputLimits {
        inline: INLINE_VALID.min(read_back),
        read_back,
    }
}

/// What Claude receives of a command's output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BashOutputWindow {
    /// What Claude receives inline: the output, a preview, or a head-and-tail excerpt.
    pub text: String,
    /// True when the output went to a file in the session directory and `text` is only its
    /// preview.
    pub saved_to_file: bool,
}

/// What Claude receives of a command's output (tools-reference.md, "Output limits"). A valid
/// result arrives inline up to the ceiling; past that it is saved to a file and Claude gets a
/// preview of the first 2,000 characters. A failure arrives inline up to 10,000 characters; past
/// that, a head-and-tail excerpt of that size cut from the read-back window, with no file path.
/// The split between head and tail is half and half (spec/judgments.md). Writing the file is the
/// host's.
#[must_use]
pub fn window_bash_output(output: &str, valid: bool, limits: BashOutputLimits) -> BashOutputWindow {
    if valid {
        return if code_unit_length(output) <= limits.inline {
            BashOutputWindow {
                text: output.to_owned(),
                saved_to_file: false,
            }
        } else {
            BashOutputWindow {
                text: slice_code_units(output, 0, Some(PREVIEW)),
                saved_to_file: true,
            }
        };
    }
    let window = slice_code_units(output, 0, Some(limits.read_back));
    let length = code_unit_length(&window);
    if length <= INLINE_FAILURE {
        return BashOutputWindow {
            text: window,
            saved_to_file: false,
        };
    }
    let half = INLINE_FAILURE / 2;
    let head = slice_code_units(&window, 0, Some(half));
    let tail = slice_code_units(&window, length - half, None);
    BashOutputWindow {
        text: head + &tail,
        saved_to_file: false,
    }
}
