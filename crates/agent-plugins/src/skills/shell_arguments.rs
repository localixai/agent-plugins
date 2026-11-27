//! Ported from `src/skills/shell-arguments.ts`; only the core uses it now, so it has no wrapper.
//!
//! How a `!` command receives the invocation's arguments (skills.md, "Available string
//! substitutions"; docs/SECURITY.md, "`$ARGUMENTS` in `!` blocks"). Judgment (spec/judgments.md,
//! `shellArguments`): `pasted`, the default, writes each value into the command as Claude Code does;
//! `environment-reference` gives a bash command each placeholder as a reference to a variable that
//! holds the value, so the shell never parses a value as code, whatever quoting the author put
//! around the placeholder. The permission check reads the command with each value quoted where it
//! lands. PowerShell keeps the values pasted.

use std::collections::HashMap;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::arguments::parse_arguments;
use super::substitute::{ArgumentInput, Insert, Placeholder, rewrite_arguments};
use crate::format::skill_types::SkillShell;
use crate::js::number_to_string;

/// How a `!` command gets the arguments.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShellArguments {
    /// Each value written into the command.
    Pasted,
    /// Each placeholder a reference to a variable holding the value.
    EnvironmentReference,
}

/// A `!` command with its arguments bound: what is checked, what the shell runs, and its variables.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoundArguments {
    /// What the shell runs.
    pub shell: String,
    /// What the permission check reads.
    pub checked: String,
    /// The variables the references read.
    pub environment: IndexMap<String, String>,
}

/// Whether the host's choice binds the arguments of a command for this shell.
const fn binds(choice: ShellArguments, shell: SkillShell) -> bool {
    match (choice, shell) {
        (ShellArguments::Pasted, SkillShell::Bash | SkillShell::Powershell)
        | (ShellArguments::EnvironmentReference, SkillShell::Powershell) => false,
        (ShellArguments::EnvironmentReference, SkillShell::Bash) => true,
    }
}

/// `BINDS[choice ?? 'pasted'][shell]`.
#[must_use]
pub fn binds_arguments(choice: Option<ShellArguments>, shell: SkillShell) -> bool {
    binds(choice.unwrap_or(ShellArguments::Pasted), shell)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Quoting {
    None,
    Single,
    Double,
}

/// The quoting after a character: a quote opens or closes its own kind, and only outside the other.
const fn next(quoting: Quoting, character: char) -> Quoting {
    match (quoting, character) {
        (Quoting::None, '\'') => Quoting::Single,
        (Quoting::None, '"') => Quoting::Double,
        (Quoting::Single, '\'') | (Quoting::Double, '"') => Quoting::None,
        (other, _) => other,
    }
}

/// Whether a backslash escapes the next character: everywhere but in single quotes.
const fn escapes(quoting: Quoting) -> bool {
    !matches!(quoting, Quoting::Single)
}

/// The quoting each `$` of a POSIX command sits in, by its byte offset.
fn quoting_of_dollars(command: &str) -> HashMap<usize, Quoting> {
    let mut found = HashMap::new();
    let mut quoting = Quoting::None;
    let mut characters = command.char_indices();
    while let Some((at, character)) = characters.next() {
        if character == '$' {
            found.insert(at, quoting);
        }
        if character == '\\' && escapes(quoting) {
            characters.next();
        } else {
            quoting = next(quoting, character);
        }
    }
    found
}

/// A reference that reads as one value in the quoting it lands in.
fn reference(quoting: Quoting, name: &str) -> String {
    match quoting {
        Quoting::None => format!("\"${{{name}}}\""),
        Quoting::Double => format!("${{{name}}}"),
        // Close the author's single quote, expand in double quotes, reopen it.
        Quoting::Single => format!("'\"${{{name}}}\"'"),
    }
}

/// `/^[A-Za-z0-9_@%+=:,./-]+$/`.
fn is_bare(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_@%+=:,./-".contains(&byte))
}

fn quote_word(value: &str) -> String {
    if is_bare(value) {
        value.to_owned()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

/// The value as the shell would read it in the quoting it lands in: what a rule checks.
fn literal(quoting: Quoting, value: &str) -> String {
    match quoting {
        Quoting::None => quote_word(value),
        Quoting::Double => value.chars().fold(String::new(), |mut out, character| {
            if matches!(character, '\\' | '"' | '$' | '`') {
                out.push('\\');
            }
            out.push(character);
            out
        }),
        Quoting::Single => value.replace('\'', "'\\''"),
    }
}

const WHOLE: &str = "SKILL_ARGUMENTS";

fn argument_variable(index: f64) -> String {
    format!("SKILL_ARGUMENT_{}", number_to_string(index))
}

/// The `reference` insert: each placeholder a variable, kept in `environment`.
struct References<'a> {
    quoting: &'a HashMap<usize, Quoting>,
    words: &'a [String],
    environment: IndexMap<String, String>,
}

impl References<'_> {
    fn name(&mut self, variable: String, value: &str) -> String {
        self.environment.insert(variable.clone(), value.to_owned());
        variable
    }
}

impl Insert for References<'_> {
    fn insert(&mut self, placeholder: &Placeholder, value: &str, at: usize) -> String {
        let quoting = self.quoting.get(&at).copied().unwrap_or(Quoting::None);
        if let Some(index) = placeholder.index {
            let name = self.name(argument_variable(index), value);
            return reference(quoting, &name);
        }
        if quoting != Quoting::None {
            let name = self.name(WHOLE.to_owned(), value);
            return reference(quoting, &name);
        }
        let words = self.words;
        #[expect(clippy::cast_precision_loss, reason = "a list's position")]
        let named: Vec<String> = words
            .iter()
            .enumerate()
            .map(|(index, word)| {
                let name = self.name(argument_variable(index as f64), word);
                reference(Quoting::None, &name)
            })
            .collect();
        named.join(" ")
    }
}

/// The `literal` insert: each value quoted where it lands.
struct Literals<'a> {
    quoting: &'a HashMap<usize, Quoting>,
    words: &'a [String],
}

impl Insert for Literals<'_> {
    fn insert(&mut self, placeholder: &Placeholder, value: &str, at: usize) -> String {
        let quoting = self.quoting.get(&at).copied().unwrap_or(Quoting::None);
        let whole = placeholder.index.is_none() && quoting == Quoting::None;
        if whole {
            self.words
                .iter()
                .map(|word| quote_word(word))
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            literal(quoting, value)
        }
    }
}

/// `command`'s argument placeholders bound: an unquoted `$ARGUMENTS` is each argument as its own
/// word (the typed quoting groups words), in quotes the text as typed; `$N`, `$ARGUMENTS[N]` and a
/// declared `$name` are one argument. Escapes and unmatched placeholders are as `substituteArguments`
/// leaves them.
#[must_use]
pub fn bind_shell_arguments(command: &str, input: &ArgumentInput) -> BoundArguments {
    let quoting = quoting_of_dollars(command);
    let words = parse_arguments(&input.raw_arguments);
    // A control character never reaches here: the checked text holding it is refused (`shell-run.ts`).
    let mut references = References {
        quoting: &quoting,
        words: &words,
        environment: IndexMap::new(),
    };
    let shell = rewrite_arguments(command, input, &mut references);
    let checked = rewrite_arguments(
        command,
        input,
        &mut Literals {
            quoting: &quoting,
            words: &words,
        },
    );
    BoundArguments {
        shell,
        checked,
        environment: references.environment,
    }
}
