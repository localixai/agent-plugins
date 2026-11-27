//! Port of `src/rules/tools/shell-selection.ts`, switched.
//!
//! tools-reference.md, "Shell selection in settings, hooks, and skills": `"defaultShell": "powershell"`
//! "routes interactive `!` commands through PowerShell. Requires the PowerShell tool to be enabled."
//! (A command hook's `shell` and a skill's `shell` are read by `hooks` and `skills`.)

use serde::{Deserialize, Serialize};

/// The shell an interactive `!` command runs in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InteractiveShell {
    /// `bash`.
    Bash,
    /// `powershell`.
    Powershell,
}

/// What `interactiveShell` reads.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShellSettings {
    /// The `defaultShell` setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_shell: Option<String>,
    /// Whether the PowerShell tool is enabled.
    pub power_shell_tool_enabled: bool,
}

/// The shell an interactive `!` command runs in. Judgment (spec/judgments.md): any `defaultShell`
/// other than `powershell`, and `powershell` while the PowerShell tool is off, leaves Bash.
#[must_use]
pub fn interactive_shell(input: &ShellSettings) -> InteractiveShell {
    if input.default_shell.as_deref() == Some("powershell") && input.power_shell_tool_enabled {
        InteractiveShell::Powershell
    } else {
        InteractiveShell::Bash
    }
}

crate::registry::exports! {
    "rules/tools/shell-selection" {
        "interactiveShell" => ok |input: ShellSettings| interactive_shell(&input);
    }
}
