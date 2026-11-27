//! Port of `src/rules/bash/exec-wrappers.ts`, switched.
//!
//! permissions.md ("Exec wrappers and `find` actions"): "A prefix rule like `Bash(watch *)` or
//! `Bash(find *)` can't auto-approve" the exec wrappers `watch`, `setsid`, `ionice` and `flock`,
//! nor `find` "with an action that runs commands, deletes files, or writes files, such as `-exec`,
//! `-delete`, `-rm`, or `-fprint`, or with `-files0-from`".

use std::collections::HashSet;

use super::words::{ShellCommand, read_command};
use super::wrappers::stripped_words;
use crate::rules::types::PermissionBehavior;

const EXEC_WRAPPERS: [&str; 4] = ["watch", "setsid", "ionice", "flock"];

/// The four actions the docs name, then the others of the same classes ("runs commands, deletes
/// files, or writes files") that find has: `-execdir`, `-ok`, `-okdir`, `-fprint0`, `-fprintf` and
/// `-fls` (spec/judgments.md).
const FIND_ACTIONS: [&str; 11] = [
    "-exec",
    "-delete",
    "-rm",
    "-fprint",
    "-files0-from",
    "-execdir",
    "-ok",
    "-okdir",
    "-fprint0",
    "-fprintf",
    "-fls",
];

/// Whether a prefix (wildcard) allow rule must not approve this command: an exec wrapper, or a
/// `find` with an action that runs, deletes or writes. Wrappers and assignments in front are seen
/// through, whichever rule is asked; quoting a name or an action changes neither (`"watch"` is
/// `watch`, crates/agent-plugins/src/rules/bash/unquote.rs).
#[must_use]
pub fn prompts_despite_prefix_rule(command: &ShellCommand) -> bool {
    let words = stripped_words(command, PermissionBehavior::Deny, &HashSet::new());
    let Some((program, rest)) = words.split_first() else {
        return false;
    };
    if EXEC_WRAPPERS.contains(&program.value.as_str()) {
        return true;
    }
    program.value == "find"
        && rest
            .iter()
            .any(|word| FIND_ACTIONS.contains(&word.value.as_str()))
}

/// `promptsDespitePrefixRule` for a subcommand's text.
#[must_use]
pub fn requires_prompt_despite_prefix_rule(subcommand: &str) -> bool {
    prompts_despite_prefix_rule(&read_command(subcommand, None))
}

crate::registry::exports! {
    "rules/bash/exec-wrappers" {
        "requiresPromptDespitePrefixRule" => ok |subcommand: String| requires_prompt_despite_prefix_rule(&subcommand);
    }
}
