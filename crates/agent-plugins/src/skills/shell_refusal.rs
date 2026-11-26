//! Port of `src/skills/shell-refusal.ts`, switched.
//!
//! The commands the shell tools refuse to run (errors.md, "No shell command in a skill or command
//! was run"). Judgment (spec/judgments.md): a file with CRLF line endings is not refused for its
//! carriage returns, and the check comes before the permission check.

use crate::ports::command_runner::ShellKind;

const fn is_control(code: u32) -> bool {
    code <= 0x08 || (code >= 0x0b && code <= 0x1f) || (code >= 0x7f && code <= 0x9f)
}

/// "PowerShell tool: also U+FEFF or U+180E".
const fn is_power_shell_only(code: u32) -> bool {
    code == 0xfeff || code == 0x180e
}

/// The characters each tool refuses (errors.md): "a raw control character other than a tab or
/// newline" for either tool, and for PowerShell also U+FEFF and U+180E.
const fn refused(shell: ShellKind, code: u32) -> bool {
    match shell {
        ShellKind::Bash => is_control(code),
        ShellKind::Powershell => is_control(code) || is_power_shell_only(code),
    }
}

const fn tool_label(shell: ShellKind) -> &'static str {
    match shell {
        ShellKind::Bash => "Bash",
        ShellKind::Powershell => "PowerShell",
    }
}

/// Each refused character shown as the replacement mark.
fn shown(command: &str, shell: ShellKind) -> String {
    command
        .chars()
        .map(|character| {
            if refused(shell, u32::from(character)) {
                '\u{FFFD}'
            } else {
                character
            }
        })
        .collect()
}

/// The message of errors.md for a command the tool does not accept, or `None` when it does.
#[must_use]
pub fn refusal_of(command: &str, name: &str, shell: ShellKind) -> Option<String> {
    let marked = shown(command, shell);
    if marked == command {
        return None;
    }
    Some(format!(
        "No shell command in {name} was run, because the {} tool does not accept one of them: command contains control characters that would be hidden in the approval dialog. Unprintable characters are shown as \u{FFFD} below: remove them from the skill or command file, or from the arguments or options substituted into it.\n{marked}",
        tool_label(shell)
    ))
}

crate::registry::exports! {
    "skills/shell-refusal" {
        "refusalOf" => ok |command: String, name: String, shell: ShellKind| refusal_of(&command, &name, shell);
    }
}
