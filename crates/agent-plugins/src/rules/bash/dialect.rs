//! Port of `src/rules/bash/dialect.ts`, switched.
//!
//! How a shell writes what the splitter has to tell apart. Bash and PowerShell differ in their
//! separators, their escape character and what a backtick is, so each is data here and the one
//! scanner (`split.ts`) reads it.

use serde::Serialize;

/// A shell's spelling of what the splitter tells apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[expect(
    clippy::struct_excessive_bools,
    reason = "the TypeScript interface's own flags, each an independent trait of a shell"
)]
pub struct ShellDialect {
    /// The command separators, longest first (permissions.md, "Compound commands" and "PowerShell").
    pub separators: &'static [&'static str],
    /// The separators after which the command goes on, so one with nothing after it is unparseable.
    pub chains: &'static [&'static str],
    /// The character that makes the next one literal.
    pub escape: &'static str,
    /// A backtick opens a command substitution (Bash) rather than escaping a character (PowerShell).
    pub backtick_substitutes: bool,
    /// `{ ... }` holds commands of its own (a PowerShell script block).
    pub braces_hold_commands: bool,
    /// `do`, `then`, `done`, `for ...` and the like are shell keywords, not commands.
    pub has_keywords: bool,
    /// `<# ... #>` is a block comment (PowerShell); a `#` that starts a word comments out its line in both.
    pub block_comments: bool,
}

/// permissions.md, "Compound commands": `&&`, `||`, `;`, `|`, `|&`, `&` and newlines.
pub const BASH_DIALECT: ShellDialect = ShellDialect {
    separators: &["&&", "||", "|&", ";", "|", "&", "\n"],
    chains: &["&&", "||"],
    escape: "\\",
    backtick_substitutes: true,
    braces_hold_commands: false,
    has_keywords: true,
    block_comments: false,
};

/// permissions.md, "PowerShell": `|`, `;` and on PowerShell 7+ `&&` and `||` split a compound
/// command. `&` is the call operator there, and the backtick is the escape character.
pub const POWERSHELL_DIALECT: ShellDialect = ShellDialect {
    separators: &["&&", "||", ";", "|", "\n"],
    chains: &["&&", "||"],
    escape: "`",
    backtick_substitutes: false,
    braces_hold_commands: true,
    has_keywords: false,
    block_comments: true,
};

crate::registry::exports! {
    "rules/bash/dialect" {
        "BASH_DIALECT" => ok || BASH_DIALECT;
        "POWERSHELL_DIALECT" => ok || POWERSHELL_DIALECT;
    }
}
