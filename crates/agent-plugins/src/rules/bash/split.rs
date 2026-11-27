//! Port of `src/rules/bash/split.ts`, switched.

use std::sync::LazyLock;

use serde::Serialize;

use crate::js::trim;
use crate::rules::bash::dialect::{BASH_DIALECT, POWERSHELL_DIALECT, ShellDialect};
use crate::rules::bash::scanner::{Span, new_scan, read_list};
use crate::rules::bash::words::{ShellCommand, read_command};
use crate::rules::memo::{Memo, memoize};

/// The subcommands a command line runs, and whether the kit could read it all.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SplitCommand {
    /// In reading order, a command before the ones nested in it; best effort when not parseable.
    pub subcommands: Vec<String>,
    /// False for a command with nothing after `&&` or `||`, or an unclosed quote or bracket.
    pub parseable: bool,
}

/// Everything the kit reads from one command line, read once per line (`scan_command`) and
/// shared by the rule matchers, the read-only and `acceptEdits` checks, the redirect checks, the
/// critical-path check, the file targets and the hook `if` filter.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommandScan {
    /// In reading order, a command before the ones nested in it; best effort when not parseable.
    pub subcommands: Vec<String>,
    /// False for a command with nothing after `&&` or `||`, or an unclosed quote or bracket.
    pub parseable: bool,
    /// Each subcommand read into its words and redirections, in the order of `subcommands`.
    pub commands: Vec<ShellCommand>,
    /// The whole line read as one command: what a deny or ask rule tries when the line is
    /// unparseable.
    pub whole: ShellCommand,
    /// `$(`, a backtick (Bash), `${` or `$NAME` outside single quotes and not escaped: the
    /// command may run something its text does not show.
    pub substitutes: bool,
    /// A subshell, a command or process substitution, or an arithmetic expansion.
    pub nests: bool,
    /// The functions the line defines (`f() { ... }`, `function f`): a later `f` runs their body.
    pub functions: Vec<String>,
    /// Where the comments are (cut from the subcommands).
    pub comments: Vec<Span>,
    /// Where the here-document bodies are (text, not commands).
    pub heredocs: Vec<Span>,
}

fn scan_text(src: &str, dialect: &ShellDialect) -> CommandScan {
    let mut scan = new_scan(src, dialect, 0);
    read_list(&mut scan, None);
    let commands: Vec<ShellCommand> = scan
        .subs
        .iter()
        .map(|sub| read_command(sub, Some(dialect)))
        .collect();
    // `echo >` with nothing to redirect to is a syntax error, so the line is not read.
    let complete = commands.iter().all(|command| {
        command
            .redirects
            .iter()
            .all(|redirect| redirect.target.is_some())
    });
    CommandScan {
        subcommands: scan.subs,
        parseable: scan.ok && complete,
        commands,
        whole: read_command(trim(src), Some(dialect)),
        substitutes: scan.substitutes,
        nests: scan.nests,
        functions: scan.functions,
        comments: scan.comments,
        heredocs: scan.bodies,
    }
}

static BASH_SCANS: LazyLock<Memo<CommandScan>> =
    LazyLock::new(|| memoize(|command| scan_text(command, &BASH_DIALECT), 64));
static POWERSHELL_SCANS: LazyLock<Memo<CommandScan>> =
    LazyLock::new(|| memoize(|command| scan_text(command, &POWERSHELL_DIALECT), 64));

/// The memo of a dialect's scans (`SCANS`): the TypeScript finds the two by identity, the core by
/// value, which reads the same.
fn scans(dialect: &ShellDialect) -> Option<&'static Memo<CommandScan>> {
    if *dialect == BASH_DIALECT {
        Some(&BASH_SCANS)
    } else if *dialect == POWERSHELL_DIALECT {
        Some(&POWERSHELL_SCANS)
    } else {
        None
    }
}

/// The reading of a command line (`CommandScan`), remembered for the last lines read so every
/// check of one call shares it.
#[must_use]
pub fn scan_command(command: &str, dialect: Option<&ShellDialect>) -> CommandScan {
    let bash = BASH_DIALECT;
    let dialect = dialect.unwrap_or(&bash);
    scans(dialect).map_or_else(|| scan_text(command, dialect), |memo| memo.get(command))
}

/// Splits a command line into the commands it runs (permissions.md, "Compound commands"): at
/// every separator outside quotes and comments, and again inside `$()`, backticks, subshells,
/// function bodies, unquoted here-document bodies and control-flow bodies, so a rule sees a
/// command nested in another. The text of each subcommand is as written, comments cut. A command
/// the kit cannot read to the end is `parseable: false`, with the subcommands found so far.
#[must_use]
pub fn split_command(command: &str, dialect: Option<&ShellDialect>) -> SplitCommand {
    let CommandScan {
        subcommands,
        parseable,
        ..
    } = scan_command(command, dialect);
    SplitCommand {
        subcommands,
        parseable,
    }
}

crate::registry::exports! {
    "rules/bash/split" {
        "splitCommand" => ok |command: String, dialect: Option<ShellDialect>| split_command(&command, dialect.as_ref());
    }
}
