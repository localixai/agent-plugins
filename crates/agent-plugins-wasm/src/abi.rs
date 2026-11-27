//! The module's exports. The host asks for two buffers, writes a function's name
//! (`format/path-text#isWithin`) into one and its arguments' JSON array into the other, and
//! calls `agent_plugins_call`, which takes both back and returns a new buffer, the answer: four
//! bytes of its length (little-endian), then its UTF-8 JSON (`{"result"}`, `{"throws"}` or
//! `{"fault"}`, `agent_plugins::registry`). The host reads it and gives it back with
//! `agent_plugins_free`. Every buffer is a boxed slice, freed with the length it was made with.
//!
//! In JavaScript (the npm package's `src/wasm/bridge.ts`):
//!
//! ```js
//! const name = put(encoder.encode('format/path-text#isWithin'))
//! const args = put(encoder.encode('["/a", "/a/b"]'))
//! const answer = agent_plugins_call(name.at, name.length, args.at, args.length)
//! const length = new DataView(memory.buffer).getUint32(answer, true)
//! const json = decoder.decode(new Uint8Array(memory.buffer, answer + 4, length))
//! agent_plugins_free(answer, length + 4)
//! ```
//!
//! An export that waits on the host (docs/RUST.md, "The boundary") is driven with the same
//! buffers: `agent_plugins_start` takes a name and arguments as `agent_plugins_call` does and
//! returns a step's buffer (`{"done"}`, `{"task", "needs"}` or `{"rethrow"}`,
//! `agent_plugins::registry::Step`); `agent_plugins_resume` takes a task's number and a buffer
//! of the host's answers (`{"answers": [...], "aborted": [...]}`) and returns the next step's;
//! `agent_plugins_release` forgets a handle the host no longer holds, and `agent_plugins_cancel`
//! drops a task it will not resume (a core promise it can no longer drive).
#![expect(
    unsafe_code,
    reason = "an export keeps its name (`no_mangle`), and a buffer the host hands back is \
              rebuilt from its pointer and length: the module's whole ABI, and only here"
)]

use std::ptr;

/// A buffer of `len` zeroed bytes for the host to write into. It is the module's again once
/// handed to `agent_plugins_call` or given to `agent_plugins_free`.
#[unsafe(no_mangle)]
pub extern "C" fn agent_plugins_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0_u8; len].into_boxed_slice()).cast::<u8>()
}

/// Frees a buffer: an answer (its length is the four bytes' value plus four) or an unused one
/// from `agent_plugins_alloc`.
///
/// # Safety
///
/// `ptr` and `len` are a buffer this module made and has not freed or taken back.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_plugins_free(ptr: *mut u8, len: usize) {
    drop(unsafe { take(ptr, len) });
}

/// Calls the export named in one buffer with the arguments in the other (`registry::call`) and
/// returns the answer's buffer. Text that is not UTF-8 is a fault, answered as one.
///
/// # Safety
///
/// As `agent_plugins_free`, for both buffers: they are taken back and freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_plugins_call(
    name_ptr: *mut u8,
    name_len: usize,
    args_ptr: *mut u8,
    args_len: usize,
) -> *mut u8 {
    let name = unsafe { take(name_ptr, name_len) };
    let args = unsafe { take(args_ptr, args_len) };
    give(&answer(&name, &args))
}

/// Starts the export named in one buffer with the arguments in the other
/// (`registry::start_json`) and returns the step's buffer.
///
/// # Safety
///
/// As `agent_plugins_call`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_plugins_start(
    name_ptr: *mut u8,
    name_len: usize,
    args_ptr: *mut u8,
    args_len: usize,
) -> *mut u8 {
    let name = unsafe { take(name_ptr, name_len) };
    let args = unsafe { take(args_ptr, args_len) };
    give(&step(&name, &args))
}

/// Gives task `task` the host's answers in the buffer (`registry::resume_json`) and returns the
/// next step's buffer.
///
/// # Safety
///
/// As `agent_plugins_free`, for the buffer: it is taken back and freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn agent_plugins_resume(
    task: u32,
    answers_ptr: *mut u8,
    answers_len: usize,
) -> *mut u8 {
    let answers = unsafe { take(answers_ptr, answers_len) };
    give(&resumed(task, &answers))
}

/// Forgets the handle `handle` (`registry::release`); a handle already released is no fault.
#[unsafe(no_mangle)]
pub extern "C" fn agent_plugins_release(handle: u32) {
    agent_plugins::registry::release(handle);
}

/// Drops the task `task` (`registry::cancel`): a core promise the host can no longer drive; a
/// task already ended is no fault.
#[unsafe(no_mangle)]
pub extern "C" fn agent_plugins_cancel(task: u32) {
    agent_plugins::registry::cancel(task);
}

/// The step's text for a name and arguments as bytes.
#[must_use]
pub fn step(name: &[u8], args: &[u8]) -> String {
    match (std::str::from_utf8(name), std::str::from_utf8(args)) {
        (Ok(name), Ok(args)) => agent_plugins::registry::start_json(name, args),
        (Err(_), _) | (_, Err(_)) => {
            r#"{"done":{"fault":"the name or the arguments are not UTF-8"}}"#.to_owned()
        }
    }
}

/// The next step's text for a task and the host's answers as bytes.
#[must_use]
pub fn resumed(task: u32, answers: &[u8]) -> String {
    match std::str::from_utf8(answers) {
        Ok(answers) => agent_plugins::registry::resume_json(task, answers),
        Err(_) => {
            agent_plugins::registry::cancel(task);
            r#"{"done":{"fault":"the answers are not UTF-8"}}"#.to_owned()
        }
    }
}

/// The answer's text for a name and arguments as bytes.
#[must_use]
pub fn answer(name: &[u8], args: &[u8]) -> String {
    match (std::str::from_utf8(name), std::str::from_utf8(args)) {
        (Ok(name), Ok(args)) => agent_plugins::registry::call(name, args),
        (Err(_), _) | (_, Err(_)) => {
            r#"{"fault":"the name or the arguments are not UTF-8"}"#.to_owned()
        }
    }
}

/// A buffer this module made, taken back.
///
/// # Safety
///
/// As `agent_plugins_free`.
unsafe fn take(ptr: *mut u8, len: usize) -> Box<[u8]> {
    unsafe { Box::from_raw(ptr::slice_from_raw_parts_mut(ptr, len)) }
}

/// An answer's buffer: its length in four bytes, then its text. A wasm32 memory holds at most
/// 4 GiB, so the length fits; an answer that does not is answered as a fault instead.
fn give(text: &str) -> *mut u8 {
    let fault = r#"{"fault":"the answer is larger than 4 GiB"}"#;
    let (length, body) = match u32::try_from(text.len()) {
        Ok(length) => (length, text.as_bytes()),
        Err(_) => (u32::try_from(fault.len()).unwrap_or(0), fault.as_bytes()),
    };
    let mut buffer = Vec::with_capacity(body.len() + 4);
    buffer.extend_from_slice(&length.to_le_bytes());
    buffer.extend_from_slice(body);
    Box::into_raw(buffer.into_boxed_slice()).cast::<u8>()
}
