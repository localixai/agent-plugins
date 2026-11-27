//! Port of `src/cli/command-table.ts`, switched.
//!
//! The commands `claude` takes that this module knows (cli-reference.md, "CLI commands";
//! plugins/cli-reference.md, "claude plugin commands" and "claude plugin marketplace commands"): the
//! names, the aliases and who runs each. The kit has no CLI of its own; a `planned` command is one
//! whose outcome the kit models as a plan over ports, a `host-delegated` one is another product
//! feature (agent view, MCP servers and their OAuth) the host runs, recorded here so a host that routes
//! words to commands has one table. A registry typed over the documented names: no `switch`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// `claude plugin <subcommand>`; `claude plugin eval init` is a command of `eval`.
pub const PLUGIN_SUBCOMMANDS: &[&str] = &[
    "init",
    "install",
    "uninstall",
    "enable",
    "disable",
    "update",
    "list",
    "details",
    "configure",
    "prune",
    "eval",
    "tag",
    "test",
    "validate",
    "marketplace",
];

/// `claude plugin marketplace <subcommand>`.
pub const MARKETPLACE_SUBCOMMANDS: &[&str] = &["add", "list", "remove", "update"];

/// `claude mcp login` and `claude mcp logout` are listed beside `claude mcp`.
pub const MCP_SUBCOMMANDS: &[&str] = &["login", "logout"];

/// Who runs a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CommandHandling {
    /// The kit models the outcome as a plan over ports.
    Planned,
    /// Another product feature the host runs.
    HostDelegated,
}

/// What a word that is none of the command's subcommands is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnlistedWord {
    /// An unknown command.
    Unknown,
    /// An argument of the command's own.
    Argument,
}

/// `COMMAND_HANDLINGS`.
pub const COMMAND_HANDLINGS: [CommandHandling; 2] =
    [CommandHandling::Planned, CommandHandling::HostDelegated];

/// One command of the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRow {
    /// Other words that run the same command, at the same position.
    pub aliases: &'static [&'static str],
    /// Who runs it.
    pub handling: CommandHandling,
    /// What a word that is none of the command's subcommands is: `unknown` for `claude plugin` and
    /// `claude plugin marketplace` (`claude plugin add` is "unknown command 'add'"), `argument` for a
    /// command that takes arguments of its own.
    pub unlisted_word: UnlistedWord,
}

const fn planned(aliases: &'static [&'static str]) -> CommandRow {
    CommandRow {
        aliases,
        handling: CommandHandling::Planned,
        unlisted_word: UnlistedWord::Argument,
    }
}
const PARENT: CommandRow = CommandRow {
    aliases: &[],
    handling: CommandHandling::Planned,
    unlisted_word: UnlistedWord::Unknown,
};
const DELEGATED: CommandRow = CommandRow {
    aliases: &[],
    handling: CommandHandling::HostDelegated,
    unlisted_word: UnlistedWord::Argument,
};

/// Every command with its aliases; a name added to a union above fails the typecheck until it has a row.
const COMMAND_TABLE: [(&str, CommandRow); 24] = [
    (
        "plugin",
        CommandRow {
            aliases: &["plugins"],
            ..PARENT
        },
    ),
    ("plugin init", planned(&["new"])),
    ("plugin install", planned(&["i"])),
    ("plugin uninstall", planned(&["remove", "rm"])),
    ("plugin enable", planned(&[])),
    ("plugin disable", planned(&[])),
    ("plugin update", planned(&[])),
    ("plugin list", planned(&[])),
    ("plugin details", planned(&[])),
    ("plugin configure", planned(&[])),
    ("plugin prune", planned(&["autoremove"])),
    ("plugin eval", planned(&[])),
    ("plugin tag", planned(&[])),
    ("plugin test", planned(&[])),
    ("plugin validate", planned(&[])),
    ("plugin marketplace", PARENT),
    ("plugin marketplace add", planned(&[])),
    ("plugin marketplace list", planned(&[])),
    ("plugin marketplace remove", planned(&["rm"])),
    ("plugin marketplace update", planned(&[])),
    ("agents", DELEGATED),
    ("mcp", DELEGATED),
    ("mcp login", DELEGATED),
    ("mcp logout", DELEGATED),
];

/// Every command with its aliases, in the table's order.
#[must_use]
pub fn command_table() -> IndexMap<&'static str, CommandRow> {
    COMMAND_TABLE.iter().copied().collect()
}

/// The command a run of words names, and what is left after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolvedCommand {
    /// The command.
    pub command: &'static str,
    /// Who runs it.
    pub handling: CommandHandling,
    /// The words after the command, for its own parsing.
    pub rest: Vec<String>,
}

fn row_of(name: &str) -> Option<CommandRow> {
    COMMAND_TABLE
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, row)| *row)
}

fn last_word(name: &str) -> &str {
    name.rsplit(' ').next().unwrap_or(name)
}

/// The commands one word below `above` (`''` for the top level).
fn children_of(above: &str) -> Vec<&'static str> {
    let prefix = if above.is_empty() {
        String::new()
    } else {
        format!("{above} ")
    };
    COMMAND_TABLE
        .iter()
        .map(|(name, _)| *name)
        .filter(|name| {
            name.strip_prefix(prefix.as_str())
                .is_some_and(|below| !below.contains(' '))
                && *name != above
        })
        .collect()
}

fn answers_to(name: &str, word: &str) -> bool {
    last_word(name) == word || row_of(name).is_some_and(|row| row.aliases.contains(&word))
}

/// Whether the words left after a command name something its table row calls unknown.
fn strands_unknown_word(found: &str, rest: &[String]) -> bool {
    row_of(found).is_some_and(|row| row.unlisted_word == UnlistedWord::Unknown)
        && rest.first().is_some_and(|word| !word.starts_with('-'))
}

/// The command the words name, by name or alias, at each level. `claude plugin add` is unknown; a
/// word below `claude mcp` or `claude agents` that the table does not list is that command's own
/// argument, so the host keeps it in `rest`.
#[must_use]
pub fn resolve_command(words: &[String]) -> Option<ResolvedCommand> {
    let mut found: Option<&'static str> = None;
    let mut rest = words;
    while let Some(word) = rest.first() {
        let next = children_of(found.unwrap_or(""))
            .into_iter()
            .find(|name| answers_to(name, word));
        let Some(next) = next else { break };
        found = Some(next);
        rest = rest.get(1..).unwrap_or_default();
    }
    let found = found?;
    if strands_unknown_word(found, rest) {
        return None;
    }
    Some(ResolvedCommand {
        command: found,
        handling: row_of(found)?.handling,
        rest: rest.to_vec(),
    })
}

crate::registry::exports! {
    "cli/command-table" {
        "PLUGIN_SUBCOMMANDS" => ok || PLUGIN_SUBCOMMANDS;
        "MARKETPLACE_SUBCOMMANDS" => ok || MARKETPLACE_SUBCOMMANDS;
        "MCP_SUBCOMMANDS" => ok || MCP_SUBCOMMANDS;
        "COMMAND_HANDLINGS" => ok || COMMAND_HANDLINGS;
        "COMMAND_TABLE" => ok command_table;
        "resolveCommand" => ok |words: Vec<String>| resolve_command(&words);
    }
}
