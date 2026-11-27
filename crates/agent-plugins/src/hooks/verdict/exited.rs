//! Ported from `src/hooks/verdict/exited.ts`; only the core uses it now, so it has no wrapper.
//!
//! A run that ended with an exit code (hooks.md, "Exit code output"): the table of stdout against
//! exit code, with the events' own rules on top.

use serde::Deserialize;

use super::debug_lines::{Subject, plain_text_note, unrecognized_keys_note, with_notes};
use super::exit_codes::{Cell, ExitClass, MessageFrom, ResultKind, exit_class_of, exit_table};
use super::exit2::{Exit2Rule, StderrChannel, exit2_rule, ignored};
use super::notices::{
    NO_STDERR, NON_BLOCKING_STATUS, first_line, from_plugin, handler_label, hook_error, hook_name,
};
use super::on_failure::{
    Failure, FailureCause, failure_verdict, on_failure_blocks, value_of, with_block,
};
use super::plain_stdout::plain_stdout;
use super::route::{BlockText, routed_block};
use super::stdout_kind::{StdoutReading, read_stdout};
use super::suspect_capture::suspect_capture;
use super::types::{BlockVia, Feedback, HookNotice, ParseResult};
use crate::hooks::handlers::context::ExitedRun;
use crate::hooks::source_kind::{LabelledEntry, is_plugin_source, plugin_label};
use crate::js::{JsObject, JsValue, trim};

/// Everything the three exit-code readers share.
struct Read<'a> {
    subject: &'a Subject,
    run: &'a ExitedRun,
    reading: StdoutReading,
    output: Option<JsObject>,
    problem: Option<String>,
    cell: Cell,
    rule: Exit2Rule,
}

fn base(subject: &Subject) -> JsObject {
    JsObject::from_iter([
        ("handler".to_owned(), subject.entry.clone()),
        ("issues".to_owned(), JsValue::Array(vec![])),
    ])
}

/// "WorktreeCreate: a command hook can't return JSON, because Claude Code reads its stdout as the
/// worktree path."
const PATH_EVENTS: [&str; 1] = ["WorktreeCreate"];

fn is_command(subject: &Subject) -> bool {
    subject.handler().contains_key("command")
}

fn reads_path(subject: &Subject) -> bool {
    PATH_EVENTS.contains(&subject.event.as_str()) && is_command(subject)
}

/// `/No such file or directory/i`: without `u`, `i` folds ASCII letters only.
fn missing_file(stderr: &str) -> bool {
    stderr
        .to_ascii_lowercase()
        .contains("no such file or directory")
}

fn line_for(subject: &Subject, text: &str) -> String {
    hook_error(
        &hook_name(&subject.event, subject.tool_name.as_deref()),
        &handler_label(&subject.handler()),
        text,
    )
}

/// A failure of the output: it cannot be parsed or fails validation.
fn invalid_output(read: &Read<'_>) -> Failure {
    let text = read
        .problem
        .clone()
        .unwrap_or_else(|| "the output is not valid".to_owned());
    Failure {
        cause: FailureCause::InvalidOutput,
        detail: text.clone(),
        text,
    }
}

fn other_exit(read: &Read<'_>) -> Failure {
    let line = first_line(&read.run.stderr);
    let shown = if line.is_empty() { NO_STDERR } else { &line };
    Failure {
        cause: FailureCause::OtherExit,
        text: format!("{NON_BLOCKING_STATUS} {shown}"),
        detail: read.run.stderr.clone(),
    }
}

const HOOK_SPECIFIC_OUTPUT: &str = "hookSpecificOutput";

/// "Claude Code applies your `hookSpecificOutput` when your hook exits 0, and ignores it on any
/// other exit code": only that field is dropped, `systemMessage` and `continue` still count.
fn without_specific(output: &JsObject) -> Option<JsObject> {
    let rest: JsObject = output
        .iter()
        .filter(|(key, _)| *key != HOOK_SPECIFIC_OUTPUT)
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    (!rest.is_empty()).then_some(rest)
}

/// The output counts only when the table says its fields do, and the event reads it on this
/// code.
fn applied_output(read: &Read<'_>, exit_class: ExitClass) -> Option<JsObject> {
    let output = read.output.as_ref()?;
    if read.cell.fields == super::exit_codes::Fields::None {
        return None;
    }
    if read.rule.output_only_on_exit0 && exit_class != ExitClass::Zero {
        without_specific(output)
    } else {
        Some(output.clone())
    }
}

fn success(read: &Read<'_>, exit_class: ExitClass) -> JsObject {
    let subject = read.subject;
    let kind = plain_stdout().get(subject.event.as_str()).copied();
    let mut verdict = base(subject);
    verdict.insert("status".to_owned(), JsValue::from("success"));
    verdict.insert("succeeded".to_owned(), JsValue::Bool(true));
    if let Some(applied) = applied_output(read, exit_class) {
        verdict.insert("output".to_owned(), JsValue::Object(applied));
    }
    if let Some(kind) = kind.filter(|_| {
        exit_class == ExitClass::Zero && read.output.is_none() && !read.reading.text.is_empty()
    }) {
        let plain = JsObject::from_iter([
            ("text".to_owned(), JsValue::from(read.reading.text.as_str())),
            ("as".to_owned(), JsValue::from(kind)),
        ]);
        verdict.insert("plainText".to_owned(), JsValue::Object(plain));
    }
    if reads_path(subject) {
        verdict.insert("stdout".to_owned(), JsValue::from(read.run.stdout.as_str()));
    }
    if subject.effects_ignored {
        verdict.insert("effectsIgnored".to_owned(), JsValue::Bool(true));
    }
    verdict
}

fn exit0(read: &Read<'_>) -> JsObject {
    if read.cell.result == ResultKind::NonBlockingError {
        failure_verdict(read.subject, &invalid_output(read))
    } else {
        success(read, ExitClass::Zero)
    }
}

/// Exit 2 on an event that cannot block: the stderr goes where the table's row says.
fn unblockable(read: &Read<'_>) -> JsObject {
    let subject = read.subject;
    let message = line_for(subject, trim(&read.run.stderr));
    let handler_id = subject.entry_id();
    let mut verdict = base(subject);
    verdict.insert("status".to_owned(), JsValue::from("non-blocking-error"));
    verdict.insert("succeeded".to_owned(), JsValue::Bool(false));
    if let Some(output) = applied_output(read, ExitClass::Two) {
        verdict.insert("output".to_owned(), JsValue::Object(output));
    }
    match read.rule.stderr {
        StderrChannel::Claude => {
            let feedback = Feedback {
                handler_id,
                to: "claude".to_owned(),
                text: message,
            };
            verdict.insert("feedback".to_owned(), value_of(&feedback));
        }
        StderrChannel::User => {
            let notice = HookNotice {
                handler_id,
                message,
            };
            verdict.insert("notice".to_owned(), value_of(&notice));
        }
        StderrChannel::UserAndClaude | StderrChannel::Debug | StderrChannel::Discarded => {}
    }
    verdict
}

/// The plugin whose hook this is, when it is one: the error names it, the same way the missing
/// directory notice does (`pluginLabel`).
fn plugin_of(subject: &Subject) -> Option<String> {
    let entry = LabelledEntry::deserialize(subject.entry.clone()).ok()?;
    if is_plugin_source(&entry.source) {
        plugin_label(&entry)
    } else {
        None
    }
}

/// "When a plugin's hook blocks this way and its stderr is the blocking message": a hook that
/// printed nothing to stderr has no message to end, so the sentence is left out.
fn stderr_message(subject: &Subject, reason: &str) -> String {
    if reason.is_empty() {
        line_for(subject, reason)
    } else {
        from_plugin(&line_for(subject, reason), plugin_of(subject).as_deref())
    }
}

fn blocked(read: &Read<'_>) -> JsObject {
    let subject = read.subject;
    let reason = trim(&read.run.stderr).to_owned();
    let block = routed_block(
        &subject.event,
        BlockVia::Exit2,
        BlockText {
            message: stderr_message(subject, &reason),
            reason,
        },
        read.rule
            .blocks_unless_directory_gone
            .then(|| "worktree-directory-remains".to_owned()),
    );
    let mut verdict = base(subject);
    verdict.insert("status".to_owned(), JsValue::from("blocked"));
    verdict.insert("succeeded".to_owned(), JsValue::Bool(false));
    let mut verdict = with_block(verdict, &subject.entry_id(), &block);
    if let Some(output) = applied_output(read, ExitClass::Two) {
        verdict.insert("output".to_owned(), JsValue::Object(output));
    }
    verdict
}

/// "Stop, SubagentStop, TaskCompleted, and a plugin's UserPromptSubmit hook: when your hook exits
/// 2 with nothing on stdout and its stderr says a file is missing, such as `No such file or
/// directory`, Claude Code treats the run as a non-blocking error."
fn is_missing_file(read: &Read<'_>) -> bool {
    read.reading.text.is_empty()
        && missing_file(&read.run.stderr)
        && read.rule.missing_file_is_error.is_some_and(|is_error| {
            read.subject
                .source()
                .is_some_and(|source| is_error(&source))
        })
}

fn exit2(read: &Read<'_>) -> JsObject {
    if is_missing_file(read) {
        return failure_verdict(read.subject, &other_exit(read));
    }
    if read
        .rule
        .cannot_block_when
        .is_some_and(|cannot| cannot(&read.subject.facts))
    {
        return unblockable(read);
    }
    if read.rule.can_block {
        blocked(read)
    } else {
        unblockable(read)
    }
}

/// A hook that fails when it exits with another code than 0 or 2 (a failure for `onFailure`,
/// too).
fn exit_other(read: &Read<'_>) -> JsObject {
    let subject = read.subject;
    if read.rule.any_non_zero_blocks {
        return failure_verdict(subject, &other_exit(read));
    }
    if read.cell.result == ResultKind::NonBlockingError {
        let failure = if read.cell.message == MessageFrom::Validation {
            invalid_output(read)
        } else {
            other_exit(read)
        };
        return failure_verdict(subject, &failure);
    }
    // "Success. Claude Code ignores the exit code, and the fields alone decide. For hooks whose
    // failure blocks the action, the run counts as a failure instead."
    if on_failure_blocks(subject) {
        failure_verdict(subject, &other_exit(read))
    } else {
        success(read, ExitClass::Other)
    }
}

fn exit_reader(exit_class: ExitClass) -> fn(&Read<'_>) -> JsObject {
    match exit_class {
        ExitClass::Zero => exit0,
        ExitClass::Two => exit2,
        ExitClass::Other => exit_other,
    }
}

const fn class_name(exit_class: ExitClass) -> &'static str {
    match exit_class {
        ExitClass::Zero => "zero",
        ExitClass::Two => "two",
        ExitClass::Other => "other",
    }
}

/// What the stdout is, against the output schema: the row of the table it falls in.
struct Classified {
    output: Option<JsObject>,
    problem: Option<String>,
    stdout_class: &'static str,
}

fn classify(reading: &StdoutReading, checked: Option<&ParseResult>) -> Classified {
    let output = checked.and_then(|checked| checked.output.clone());
    let unchecked = reading.json.is_some() && output.is_none();
    let problem = reading
        .parse_failure
        .clone()
        .or_else(|| checked.and_then(|checked| checked.failure.clone()))
        .or_else(|| unchecked.then(|| "the output failed validation".to_owned()));
    let stdout_class = match (&problem, &output) {
        (Some(_), _) => "json-invalid",
        (None, Some(_)) => "json-valid",
        (None, None) => "plain",
    };
    Classified {
        output,
        problem,
        stdout_class,
    }
}

/// The debug lines of the reading: stdout of an exit 0 read as plain text (a `WorktreeCreate`
/// command hook's stdout is a path, never JSON, so it gets none), and keys a valid output had that
/// the schema does not name.
fn notes_of(
    subject: &Subject,
    exit_class: ExitClass,
    reading: &StdoutReading,
    unrecognized: Option<&[String]>,
) -> Vec<crate::format::issues::PluginIssue> {
    let mut notes =
        if exit_class == ExitClass::Zero && !reads_path(subject) && reading.json.is_none() {
            plain_text_note(subject, &reading.text)
        } else {
            vec![]
        };
    notes.extend(unrecognized_keys_note(subject, unrecognized));
    notes
}

/// What a run that ended with an exit code comes to.
pub async fn interpret_exited(subject: &Subject, run: &ExitedRun) -> JsObject {
    let exit_class = exit_class_of(f64::from(run.exit_code));
    // Exit 2 blocks on its own; any other run whose capture is suspect is refused.
    if run.output_held_open == Some(true) && exit_class != ExitClass::Two {
        return suspect_capture(subject);
    }
    let reading = if reads_path(subject) {
        StdoutReading {
            text: trim(&run.stdout).to_owned(),
            json: None,
            parse_failure: None,
        }
    } else {
        read_stdout(&run.stdout)
    };
    let checked = match &reading.json {
        Some(json) => Some(subject.parse(&subject.event, json.clone()).await),
        None => None,
    };
    let Classified {
        output,
        problem,
        stdout_class,
    } = classify(&reading, checked.as_ref());
    let table = exit_table();
    let Some(cell) = table
        .get(class_name(exit_class))
        .and_then(|row| row.get(stdout_class))
        .copied()
    else {
        return base(subject);
    };
    let unrecognized = checked.and_then(|checked| checked.unrecognized_keys);
    let read = Read {
        subject,
        run,
        reading,
        output,
        problem,
        cell,
        rule: exit2_rule(&subject.event).unwrap_or(ignored(StderrChannel::Discarded)),
    };
    let verdict = exit_reader(exit_class)(&read);
    let notes = notes_of(subject, exit_class, &read.reading, unrecognized.as_deref());
    with_notes(verdict, &notes)
}
