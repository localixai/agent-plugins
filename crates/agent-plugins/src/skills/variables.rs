//! Port of `src/skills/variables.ts`, switched.
//!
//! The `${CLAUDE_*}` variables of a skill's text (skills.md, "Available string substitutions").
//! The mechanism is `loading/resolve` (the one home of variable substitution, docs/ARCHITECTURE.md);
//! this file decides only what a skill adds to it: its own three variables (`CLAUDE_SESSION_ID`,
//! `CLAUDE_EFFORT`, `CLAUDE_SKILL_DIR`) and which of the six an origin or a session leaves as
//! written.

use std::sync::LazyLock;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::expand_policy::{SkillOrigin, SkillOriginKind};
use crate::format::issues::PluginIssue;
use crate::format::skill_types::{OriginTraits, origin_traits};
use crate::js::regexp::RegExp;
use crate::js::{promise_all, trim_start};
use crate::loading::resolve::substitute::{
    FieldPolicy, ShellPaths, Substituted, SubstitutionScope, TokenReference,
};
use crate::loading::resolve::substituter::{Substituter, substituter};
use crate::loading::resolve::surfaces::{SubstitutionSurface, surface_policy_of};
use crate::loading::resolve::user_config::ResolvedUserConfig;
use crate::ports::environment::Environment;
use crate::ports::locations::Locations;
use crate::ports::{BoxFuture, Signal};
use crate::registry::Port;
use crate::rules::memo::{LIMIT, Memo, memoize};

/// The ports a skill's variables are read through.
#[derive(Clone, Debug, Deserialize)]
pub struct SkillVariablePorts<L = Port, E = Port> {
    /// Where the paths come from.
    pub locations: L,
    /// The process environment.
    pub environment: E,
}

/// What the session tells a skill's variables.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillSession {
    /// `${CLAUDE_SESSION_ID}`.
    pub session_id: String,
    /// `${CLAUDE_EFFORT}`: the effort level in force while the skill runs. A skill that sets its own
    /// `effort` runs at that level, so pass the one `resolveTurnOverrides` chose. Absent, the
    /// placeholder stays as written.
    #[serde(default)]
    pub effort: Option<String>,
    /// The id `Locations` knows the plugin by; default the plugin name of the skill's origin.
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// The saved `userConfig` values of the plugin, for `${user_config.KEY}` in a plugin skill.
    #[serde(default)]
    pub user_config: Option<ResolvedUserConfig>,
    /// A Windows host: paths are written with forward slashes (docs/API.md, "Variable substitution").
    #[serde(default)]
    pub windows: Option<bool>,
    /// The session runs in an isolated container, a cloud or Cowork session, where a synced skill
    /// "keeps the behavior a local skill has" (skills.md, "How Claude Code handles the body of a
    /// synced skill"). False is a session on the owner's machine.
    #[serde(default)]
    pub isolated_session: Option<bool>,
    /// The absolute folder `skill.dir` is relative to, when it is not the origin's usual root (a
    /// project skill found in a nested `.claude/skills` or in an added directory).
    #[serde(default)]
    pub skill_root: Option<String>,
    /// How a `!` command writes the path variables for the shell (`SubstitutionScope.shellPaths`, the
    /// host's `shellPaths`): default the reference, with the value in the command's environment.
    #[serde(default)]
    pub shell_paths: Option<ShellPaths>,
    /// Cancels the work.
    #[serde(default)]
    pub signal: Option<Signal>,
}

/// What `substituteVariables` needs: the shared scope and the names this skill leaves as written.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVariables<L = Port, E = Port> {
    /// What the references resolve against.
    pub scope: SubstitutionScope<L, E>,
    /// Variable names (without `${}`) that stay as written for this origin and session.
    pub literal: Vec<String>,
    /// Only a plugin skill is told about an unresolved `${user_config.KEY}`.
    pub reports_issues: bool,
}

/// `${CLAUDE_PLUGIN_ROOT}` and `${CLAUDE_PLUGIN_DATA}`: "Substituted only in plugin skills".
const PLUGIN_ONLY: [&str; 2] = ["CLAUDE_PLUGIN_ROOT", "CLAUDE_PLUGIN_DATA"];
/// The two a synced skill does not get in a session on the owner's machine.
const SESSION_BOUND: [&str; 2] = ["CLAUDE_PROJECT_DIR", "CLAUDE_SESSION_ID"];

/// Where a skill's relative `dir` starts, per origin. Judgment: the docs name the folders of a
/// skill (`~/.claude/skills/`, `.claude/skills/`, the plugin folder), not how a host stores the
/// relative path; the kit reads it from the folder that holds the skills of that origin, a synced
/// skill from the home folder like a personal one, and a bundled one (whose `dir` the host gives
/// absolute) from the project folder.
fn roots<'a, L: Locations>(
    kind: SkillOriginKind,
    locations: &'a L,
    plugin_id: &'a str,
) -> BoxFuture<'a, String> {
    match kind {
        SkillOriginKind::Plugin => locations.plugin_root(plugin_id),
        SkillOriginKind::Managed | SkillOriginKind::Project | SkillOriginKind::Bundled => {
            locations.project_dir()
        }
        SkillOriginKind::User | SkillOriginKind::Synced => locations.home_dir(),
    }
}

/// `/^(?:[/\\]|[A-Za-z]:[/\\])/`.
fn is_absolute(path: &str) -> bool {
    let bytes = path.as_bytes();
    let separator = |byte: Option<&u8>| matches!(byte, Some(b'/' | b'\\'));
    separator(bytes.first())
        || (bytes.first().is_some_and(u8::is_ascii_alphabetic)
            && bytes.get(1) == Some(&b':')
            && separator(bytes.get(2)))
}

/// A skill's path as the host writes it: `path` as is when it is absolute, `root` for `.`, else
/// `path` below `root`. The one rule for `${CLAUDE_SKILL_DIR}` and for where a session's skill lives
/// (`SessionSkill.file`, `SessionSkill.directory`).
pub async fn skill_path_in(root: &str, path: &str, locations: &impl Locations) -> String {
    if is_absolute(path) {
        return path.to_owned();
    }
    if path == "." {
        root.to_owned()
    } else {
        locations.join_path(root, path).await
    }
}

/// `Pick<Skill | Command, 'origin'> & { dir? }`: anything with an origin; a `Command` has no folder
/// of its own.
#[derive(Clone, Debug, Deserialize)]
pub struct VariableSource {
    /// Where the skill came from.
    pub origin: SkillOrigin,
    /// The skill's folder.
    #[serde(default)]
    pub dir: Option<String>,
}

/// `ORIGIN_TRAITS[kind]`.
#[must_use]
pub fn traits_of(kind: SkillOriginKind) -> Option<OriginTraits> {
    let name = match kind {
        SkillOriginKind::Plugin => "plugin",
        SkillOriginKind::Managed => "managed",
        SkillOriginKind::User => "user",
        SkillOriginKind::Project => "project",
        SkillOriginKind::Synced => "synced",
        SkillOriginKind::Bundled => "bundled",
    };
    origin_traits().get(name).copied()
}

/// `${CLAUDE_SKILL_DIR}`: "the directory containing the skill's SKILL.md ... For plugin skills,
/// this is the skill's subdirectory within the plugin, not the plugin root". A command file has no
/// `SKILL.md` and the docs name no folder for it, so its placeholder stays as written (judgment).
async fn skill_directory<L: Locations, E>(
    source: &VariableSource,
    session: &SkillSession,
    ports: &SkillVariablePorts<L, E>,
    plugin_id: &str,
) -> Option<String> {
    let dir = source.dir.as_deref()?;
    let locations = &ports.locations;
    if is_absolute(dir) {
        return Some(dir.to_owned());
    }
    let root = match &session.skill_root {
        Some(root) => root.clone(),
        None => roots(source.origin.kind, locations, plugin_id).await,
    };
    Some(skill_path_in(&root, dir, locations).await)
}

fn forward_slashes(path: &str) -> String {
    path.replace('\\', "/")
}

/// The variables a skill adds whose value is a path (`SubstitutionScope.extraPaths`).
const SKILL_PATHS: [&str; 1] = ["CLAUDE_SKILL_DIR"];

/// The variables a skill adds to the shared scope (`scope.extra`).
fn extra_variables(session: &SkillSession, skill_dir: Option<String>) -> IndexMap<String, String> {
    let windows = session.windows == Some(true);
    let mut extra = IndexMap::from([("CLAUDE_SESSION_ID".to_owned(), session.session_id.clone())]);
    if let Some(effort) = &session.effort {
        extra.insert("CLAUDE_EFFORT".to_owned(), effort.clone());
    }
    if let Some(dir) = skill_dir {
        let dir = if windows { forward_slashes(&dir) } else { dir };
        extra.insert("CLAUDE_SKILL_DIR".to_owned(), dir);
    }
    extra
}

/// The names this origin and session leave as written.
fn literal_names(origin: &SkillOrigin, session: &SkillSession) -> Vec<String> {
    let traits = traits_of(origin.kind);
    let plugin_variables = traits.is_some_and(|traits| traits.substitutes_plugin_variables);
    let session_bound = traits.is_some_and(|traits| traits.substitutes_session_variables)
        || session.isolated_session == Some(true);
    let plugin_only = if plugin_variables {
        &[][..]
    } else {
        &PLUGIN_ONLY[..]
    };
    let bound = if session_bound {
        &[][..]
    } else {
        &SESSION_BOUND[..]
    };
    plugin_only
        .iter()
        .chain(bound)
        .map(|&name| name.to_owned())
        .collect()
}

/// The scope `loading/resolve` substitutes in, and the names this skill leaves as written.
pub async fn build_variables<L: Locations + Clone, E: Clone>(
    source: &VariableSource,
    session: &SkillSession,
    ports: &SkillVariablePorts<L, E>,
) -> SkillVariables<L, E> {
    let plugin_id = session
        .plugin_id
        .clone()
        .or_else(|| source.origin.plugin.clone())
        .unwrap_or_default();
    let skill_dir = skill_directory(source, session, ports, &plugin_id).await;
    SkillVariables {
        scope: SubstitutionScope {
            plugin_id,
            locations: ports.locations.clone(),
            environment: ports.environment.clone(),
            user_config: session.user_config.clone().unwrap_or_default(),
            windows: session.windows == Some(true),
            extra: Some(extra_variables(session, skill_dir)),
            extra_paths: Some(SKILL_PATHS.iter().map(|&name| name.to_owned()).collect()),
            shell_paths: session.shell_paths,
            signal: session.signal.clone(),
        },
        literal: literal_names(&source.origin, session),
        reports_issues: traits_of(source.origin.kind)
            .is_some_and(|traits| traits.substitutes_plugin_variables),
    }
}

/// What `substituteVariables` gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct VariablesSubstituted {
    /// The text.
    pub text: String,
    /// Warnings from the shared substitution, for a plugin skill only.
    pub issues: Vec<PluginIssue>,
}

/// A reference to one of the names, with the optional `:-default` the shared grammar reads; built once per name list.
static REFERENCE_PATTERN: LazyLock<Memo<Option<RegExp>>> = LazyLock::new(|| {
    memoize(
        |alternation| {
            RegExp::new(
                &format!("(\\$\\{{(?:{alternation})(?::-[^{{}}]*)?\\}})"),
                "",
            )
            .ok()
        },
        LIMIT,
    )
});

fn reference_to(names: &[String]) -> Option<RegExp> {
    REFERENCE_PATTERN.get(&names.join("|"))
}

/// One pass: the text and issues, what was referenced, and the substituter whose `variable` the
/// shell's environment reads.
pub struct Pass<'a, L, E> {
    /// The text.
    pub text: String,
    /// Warnings, for a plugin skill only.
    pub issues: Vec<PluginIssue>,
    /// What the pieces referenced.
    pub referenced: Vec<TokenReference>,
    /// The substituter of the pass (`variable` is its method).
    pub variable: Substituter<'a, L, E>,
}

/// `${CLAUDE_*}` in `text` under one policy, in one pass (a value is never scanned again). A
/// reference this skill leaves as written splits the text: the pieces between are substituted, the
/// reference is kept.
pub async fn substitute_pass<'a, L: Locations + Clone + 'a, E: Environment>(
    text: &str,
    vars: &'a SkillVariables<L, E>,
    policy: &FieldPolicy,
    path: &str,
) -> Pass<'a, L, E> {
    let pieces: Vec<String> = match reference_to(&vars.literal) {
        Some(pattern) if !vars.literal.is_empty() => pattern
            .split(text)
            .into_iter()
            .map(Option::unwrap_or_default)
            .collect(),
        _ => vec![text.to_owned()],
    };
    let run = substituter(&vars.scope);
    let done = promise_all(
        pieces
            .iter()
            .enumerate()
            .map(|(index, piece)| -> BoxFuture<'_, Substituted> {
                // The odd pieces are the references the split kept.
                if index % 2 == 1 {
                    Box::pin(std::future::ready(Substituted {
                        text: piece.clone(),
                        issues: vec![],
                        referenced: vec![],
                        unresolved: Default::default(),
                    }))
                } else {
                    Box::pin(run.run(piece, policy, path))
                }
            })
            .collect(),
    )
    .await;
    Pass {
        text: done.iter().map(|piece| piece.text.as_str()).collect(),
        issues: if vars.reports_issues {
            done.iter().flat_map(|piece| piece.issues.clone()).collect()
        } else {
            vec![]
        },
        referenced: done
            .into_iter()
            .flat_map(|piece| piece.referenced)
            .collect(),
        variable: run,
    }
}

/// `${CLAUDE_*}` in skill content: the paths written in. Used for the skill's Markdown, the text a
/// `!` command's permission check reads, and the Bash rules of `allowed-tools`
/// (`substituteAllowedTools`).
pub async fn substitute_variables<L: Locations + Clone, E: Environment>(
    text: &str,
    vars: &SkillVariables<L, E>,
    path: &str,
) -> VariablesSubstituted {
    let policy = surface_policy_of(SubstitutionSurface::MarkdownContent);
    let Pass { text, issues, .. } = substitute_pass(text, vars, &policy, path).await;
    VariablesSubstituted { text, issues }
}

/// A Bash rule: the only rules of `allowed-tools` skills.md says are substituted (`/^\s*Bash\s*\(/`).
fn is_bash_rule(rule: &str) -> bool {
    trim_start(rule)
        .strip_prefix("Bash")
        .is_some_and(|rest| trim_start(rest).starts_with('('))
}

/// What `substituteAllowedTools` gives.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AllowedToolsSubstituted {
    /// The rules of `allowed-tools`, in order, the Bash ones with their `${CLAUDE_*}` replaced.
    pub rules: Vec<String>,
    /// What the substitution found.
    pub issues: Vec<PluginIssue>,
}

/// `Pick<Skill, 'allowedTools'>`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllowedTools {
    /// `allowed-tools`, as written.
    pub allowed_tools: Vec<String>,
}

/// The `allowed-tools` of a skill with `${CLAUDE_*}` replaced in its Bash rules (skills.md: the rule
/// "then matches the exact command the skill body tells Claude to run"). Any other rule is kept as
/// written. The host passes the result on to the rule matcher.
pub async fn substitute_allowed_tools<L: Locations + Clone, E: Environment>(
    skill: &AllowedTools,
    vars: &SkillVariables<L, E>,
    path: &str,
) -> AllowedToolsSubstituted {
    let done = promise_all(
        skill
            .allowed_tools
            .iter()
            .map(|rule| -> BoxFuture<'_, VariablesSubstituted> {
                if is_bash_rule(rule) {
                    Box::pin(substitute_variables(rule, vars, path))
                } else {
                    Box::pin(std::future::ready(VariablesSubstituted {
                        text: rule.clone(),
                        issues: vec![],
                    }))
                }
            })
            .collect(),
    )
    .await;
    AllowedToolsSubstituted {
        rules: done.iter().map(|rule| rule.text.clone()).collect(),
        issues: done.into_iter().flat_map(|rule| rule.issues).collect(),
    }
}

crate::registry::exports! {
    "skills/variables" {
        "skillPathIn" => async ok |root: String, path: String, locations: Port| skill_path_in(&root, &path, &locations).await;
        "buildVariables" => async ok |source: VariableSource, session: SkillSession, ports: SkillVariablePorts| build_variables(&source, &session, &ports).await;
        "substituteVariables" => async ok |text: String, vars: SkillVariables, path: Option<String>| substitute_variables(&text, &vars, path.as_deref().unwrap_or("")).await;
        "substituteAllowedTools" => async ok |skill: AllowedTools, vars: SkillVariables, path: Option<String>| substitute_allowed_tools(&skill, &vars, path.as_deref().unwrap_or("allowed-tools")).await;
    }
}
