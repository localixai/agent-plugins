//! Port of `src/hooks/output/classifier-note.ts`, switched.
//!
//! The note a `PostToolUse` hook leaves for the auto mode classifier (hooks.md, "Annotate a result
//! for the auto mode classifier"): which notes reach it, how many characters of them, and how much
//! weight it gives them.

use serde::{Deserialize, Serialize};

use super::fields::text_in;
use super::merge_rules::Read;
use crate::hooks::contracts::HookSourceKind;
use crate::rules::decide::approval_memory::{ToolType, tool_type_of};

/// "Caps the notes for one tool call at 2,000 characters and truncates the rest."
const CLASSIFIER_CAP: usize = 2_000;

/// How much the classifier may trust the note. `unverified`: "application-provided context. The
/// note never establishes user intent". `application`: the note came only from in-process Agent
/// SDK callbacks during the live session, so the classifier "may weigh a user statement relayed in
/// the note as user intent".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClassifierTrust {
    /// Application-provided context.
    Unverified,
    /// Only in-process Agent SDK callbacks.
    Application,
}

/// The classifier's transcript "omits read-only lookups such as file reads and searches", and
/// "Claude Code discards a note attached to one of those calls". The docs name no list: the file
/// reads and searches (Read, Grep, Glob) of the permission docs' read-only tool type are the
/// lookups (spec/judgments.md).
#[must_use]
pub fn is_unrecorded_call(tool_name: &str) -> bool {
    tool_type_of(tool_name) == Some(ToolType::ReadOnly)
}

/// "Hooks configured in Claude Code" (settings files, plugins, skills, agent frontmatter) are
/// unverified; the `session` source kind is the SDK's in-process callbacks (spec/judgments.md).
/// "When hooks from both groups annotate the same call, the classifier treats the combined note
/// as unverified".
const fn trust(kind: HookSourceKind) -> ClassifierTrust {
    match kind {
        HookSourceKind::Settings
        | HookSourceKind::Plugin
        | HookSourceKind::Skill
        | HookSourceKind::Agent => ClassifierTrust::Unverified,
        HookSourceKind::Session => ClassifierTrust::Application,
    }
}

fn trust_of(notes: &[&Read]) -> ClassifierTrust {
    if notes
        .iter()
        .all(|read| trust(read.handler.source.kind) == ClassifierTrust::Application)
    {
        ClassifierTrust::Application
    } else {
        ClassifierTrust::Unverified
    }
}

/// The note for one tool call.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClassifierNote {
    /// The joined notes, within the cap.
    pub text: String,
    /// How much the classifier may trust it.
    pub trust: ClassifierTrust,
}

/// The note for one tool call: the notes that survived (`kept`), joined with a newline and cut to
/// the cap shared "across every hook that responds to that call". None for a call the classifier
/// does not record.
#[must_use]
pub fn classifier_note(tool_name: &str, kept: &[&Read]) -> Option<ClassifierNote> {
    if is_unrecorded_call(tool_name) {
        return None;
    }
    let written: Vec<(&Read, String)> = kept
        .iter()
        .filter_map(|read| {
            text_in(read.specific.as_ref(), "classifierContext").map(|text| (*read, text))
        })
        .collect();
    if written.is_empty() {
        return None;
    }
    let joined = written
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let reads: Vec<&Read> = written.iter().map(|(read, _)| *read).collect();
    Some(ClassifierNote {
        text: joined.chars().take(CLASSIFIER_CAP).collect(),
        trust: trust_of(&reads),
    })
}

/// "After a session resumes, Claude Code treats restored notes as unverified context": a host
/// that stores a note and gives it back to the classifier after a resume passes it through this.
#[must_use]
pub fn restored_note(note: ClassifierNote) -> ClassifierNote {
    ClassifierNote {
        trust: ClassifierTrust::Unverified,
        ..note
    }
}

crate::registry::exports! {
    "hooks/output/classifier-note" {
        "isUnrecordedCall" => ok |tool_name: String| is_unrecorded_call(&tool_name);
        "restoredNote" => ok |note: ClassifierNote| restored_note(note);
    }
}
