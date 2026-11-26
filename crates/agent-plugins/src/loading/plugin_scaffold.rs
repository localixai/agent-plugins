//! Port of `src/loading/plugin-scaffold.ts`, switched.
//!
//! The files `claude plugin init` writes (plugins/create.md, "Scaffold the plugin with `claude plugin
//! init`" and "Share the plugin through a repository"; plugins/cli-reference.md, "plugin init"): a
//! pure plan, the host writes. The command "creates `~/.claude/skills/my-tool/` with a
//! `.claude-plugin/plugin.json` and a root `SKILL.md`"; `--with skills` also scaffolds a skill under
//! `skills/`. To share a plugin through one repository "create the same layout yourself at
//! `<project>/.claude/skills/<name>/`".

use serde::{Deserialize, Serialize};

use crate::format::issues::PluginIssue;
use crate::format::names::name_load_issues;
use crate::js::json_stringify::json_stringify_indented;
use crate::js::{JsObject, JsValue};

/// Where the plugin folder goes: the personal skills directory, or a project's.
pub const SCAFFOLD_SCOPES: [ScaffoldScope; 2] = [ScaffoldScope::User, ScaffoldScope::Project];

/// One of `SCAFFOLD_SCOPES`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScaffoldScope {
    /// `~/.claude/skills`.
    User,
    /// `.claude/skills`.
    Project,
}

impl ScaffoldScope {
    /// The skills directory of the scope.
    const fn skills_directory(self) -> &'static str {
        match self {
            Self::User => "~/.claude/skills",
            Self::Project => ".claude/skills",
        }
    }
}

/// One file of the plan.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ScaffoldFile {
    /// Relative to the plugin folder.
    pub path: String,
    /// What it holds.
    pub content: String,
}

/// What `claude plugin init` writes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaffoldPlan {
    /// The plugin folder: `~/.claude/skills/<name>` or `.claude/skills/<name>` (relative to the project).
    pub directory: String,
    /// The files.
    pub files: Vec<ScaffoldFile>,
    /// `✔ Created plugin "<name>" at <directory>`.
    pub created: String,
    /// `It will auto-load next session as <name>@skills-dir. Run /reload-plugins to load it now.`
    pub loads_as: String,
}

/// What to scaffold.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScaffoldInput {
    /// The plugin's name.
    pub name: String,
    /// Where it goes.
    pub scope: ScaffoldScope,
    /// `--with skills`: also scaffold a skill under `skills/`.
    #[serde(default)]
    pub with_skills: Option<bool>,
}

/// The plan, or why the name cannot be a plugin's.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ScaffoldOutcome {
    /// The plan.
    Plan {
        /// What to write.
        plan: ScaffoldPlan,
    },
    /// The name's issues.
    Issues {
        /// Why the name cannot be a plugin's.
        issues: Vec<PluginIssue>,
    },
}

const EXAMPLE_SKILL: &str = "example";

/// Judgment (spec/judgments.md): the docs name the files, not their text. The manifest holds the
/// name `claude plugin init <name>` gives ("the plugin's `name` in its manifest"); each `SKILL.md` has a
/// `description`, which Claude needs to know when to use a skill, and a line to replace.
fn skill_file(path: String, description: &str) -> ScaffoldFile {
    ScaffoldFile {
        path,
        content: format!(
            "---\ndescription: {description}\n---\n\nReplace this with the instructions Claude should follow.\n"
        ),
    }
}

/// `JSON.stringify({ name }, null, 2)`.
fn manifest_text(name: &str) -> String {
    let mut manifest = JsObject::new();
    manifest.insert("name".to_owned(), JsValue::from(name));
    json_stringify_indented(&JsValue::Object(manifest), 2).unwrap_or_default()
}

/// The plan, or why the name cannot be a plugin's.
#[must_use]
pub fn plan_plugin_scaffold(input: &ScaffoldInput) -> ScaffoldOutcome {
    let issues = name_load_issues(&input.name);
    if !issues.is_empty() {
        return ScaffoldOutcome::Issues { issues };
    }
    let directory = format!("{}/{}", input.scope.skills_directory(), input.name);
    let mut files = vec![
        ScaffoldFile {
            path: ".claude-plugin/plugin.json".to_owned(),
            content: format!("{}\n", manifest_text(&input.name)),
        },
        skill_file(
            "SKILL.md".to_owned(),
            &format!("Use when the user asks for {}", input.name),
        ),
    ];
    if input.with_skills == Some(true) {
        files.push(skill_file(
            format!("skills/{EXAMPLE_SKILL}/SKILL.md"),
            &format!("An example skill of {}", input.name),
        ));
    }
    ScaffoldOutcome::Plan {
        plan: ScaffoldPlan {
            created: format!("✔ Created plugin \"{}\" at {directory}", input.name),
            loads_as: format!(
                "It will auto-load next session as {}@skills-dir. Run /reload-plugins to load it now.",
                input.name
            ),
            directory,
            files,
        },
    }
}

crate::registry::exports! {
    "loading/plugin-scaffold" {
        "SCAFFOLD_SCOPES" => ok || SCAFFOLD_SCOPES;
        "planPluginScaffold" => ok |input: ScaffoldInput| plan_plugin_scaffold(&input);
    }
}
