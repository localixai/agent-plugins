//! Port of `src/cli/validate-advice.ts`, switched.
//!
//! The advice on a README (plugins/cli-reference.md, "Advice on the README"): a plugin's README with
//! no line that installs the plugin from a marketplace gets the line to add. It is neither an error
//! nor a warning and never changes the verdict.

use serde::Deserialize;

use super::validate_run::ValidationAdvice;
use crate::js::regexp::RegExp;
use crate::js::{code_unit_length, is_whitespace, trim};

const PLACEHOLDER: &str = "<owner>/<repo>";

/// `plugin install` or `plugin i`, in the `/plugin` and `claude plugin` spellings, with what follows on the line.
const INSTALL: &str = r"\bplugin\s+(?:install|i)\s+(.*)$";

/// Whether a line is an install line that counts: "`<plugin>@<marketplace>`" after the command, or
/// "the plugin's name and `--marketplace <source>`". A source still holding `<owner>/<repo>` does not
/// count (§17.13.5 item 2).
fn counts_as_install(rest: &str, plugin: &str) -> bool {
    let words: Vec<&str> = rest
        .split(is_whitespace)
        .filter(|word| !word.is_empty())
        .collect();
    let prefix = format!("{plugin}@");
    if words.iter().any(|word| {
        word.starts_with(&prefix) && code_unit_length(word) > code_unit_length(plugin) + 1
    }) {
        return true;
    }
    let source = words
        .iter()
        .position(|word| *word == "--marketplace")
        .and_then(|flag| words.get(flag + 1));
    words.contains(&plugin) && source.is_some_and(|source| !source.contains(PLACEHOLDER))
}

/// Whether the README already holds a line that installs `plugin` from a marketplace.
#[must_use]
pub fn readme_installs_plugin(readme: &str, plugin: &str) -> bool {
    let Ok(mut install) = RegExp::new(INSTALL, "") else {
        return false;
    };
    readme.split('\n').any(|line| {
        let rest: Option<String> = install
            .exec(line)
            .and_then(|found| found.captures.get(1).cloned().flatten());
        rest.is_some_and(|rest: String| counts_as_install(&rest, plugin))
    })
}

/// What `readmeAdvice` is given.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct ReadmeAdviceInput {
    /// The README's path.
    pub file: String,
    /// The plugin's name.
    pub plugin: String,
    /// The README's text.
    pub readme: String,
    /// The marketplace's source (`owner/repo`).
    #[serde(default)]
    pub source: Option<String>,
}

/// The advice for a README that lacks the line, or none. `source` is the marketplace's source
/// (`owner/repo`); without one the line holds `<owner>/<repo>`, which "pasting it unchanged doesn't
/// clear".
#[must_use]
pub fn readme_advice(input: &ReadmeAdviceInput) -> Option<ValidationAdvice> {
    if readme_installs_plugin(&input.readme, &input.plugin) {
        return None;
    }
    Some(ValidationAdvice {
        file: input.file.clone(),
        message: format!(
            "This README has no line that installs \"{}\" from a marketplace. Add the install line, which works in one step on Claude Code 2.1.275 or later.",
            input.plugin
        ),
        install_line: format!(
            "/plugin install {} --marketplace {}",
            input.plugin,
            input.source.as_deref().unwrap_or(PLACEHOLDER)
        ),
    })
}

/// Whether an advice line still holds the placeholder, so pasting it does not clear the advice.
#[must_use]
pub fn advice_needs_source(advice: &ValidationAdvice) -> bool {
    advice.install_line.contains(PLACEHOLDER)
}

/// The `Advice` block as the page prints it, or nothing when there is none.
#[must_use]
pub fn render_advice(advice: &[ValidationAdvice]) -> Vec<String> {
    if advice.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![
        "ℹ Advice (does not change the verdict):".to_owned(),
        String::new(),
    ];
    for each in advice {
        lines.push(format!("  ❯ {}: {}", each.file, each.message));
        lines.push(format!("      {}", each.install_line));
    }
    lines
}

const GITHUB: &str = r"^(?:https:\/\/github\.com\/|git@github\.com:|ssh:\/\/git@github\.com\/)([^/\s]+\/[^/\s]+?)(?:\.git)?\/?$";

/// The marketplace source Claude Code fills in from "the `origin` remote of the `.git` directory at the
/// top of" the validated directory; `None` when the remote is not one it can use (judgment,
/// spec/judgments.md: a GitHub remote is the one the install line's `owner/repo` form names).
#[must_use]
pub fn source_from_origin(remote: Option<&str>) -> Option<String> {
    let remote = remote?;
    let mut github = RegExp::new(GITHUB, "").ok()?;
    github
        .exec(trim(remote))
        .and_then(|found| found.captures.get(1).cloned().flatten())
}

crate::registry::exports! {
    "cli/validate-advice" {
        "readmeInstallsPlugin" => ok |readme: String, plugin: String| readme_installs_plugin(&readme, &plugin);
        "readmeAdvice" => ok |input: ReadmeAdviceInput| readme_advice(&input);
        "adviceNeedsSource" => ok |advice: ValidationAdvice| advice_needs_source(&advice);
        "renderAdvice" => ok |advice: Vec<ValidationAdvice>| render_advice(&advice);
        "sourceFromOrigin" => ok |remote: Option<String>| source_from_origin(remote.as_deref());
    }
}
