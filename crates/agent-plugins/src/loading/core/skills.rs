//! Port of `src/loading/core/skills.ts`, switched.
//!
//! A core package's skills (core §7.1): each immediate child of `skills/` holding a path named
//! exactly `SKILL.md` that resolves to a regular file is one skill; nothing deeper is searched. A
//! `SKILL.md` outside the plugin root is skipped (core §4.1, boundary 3), and so is one that fails
//! the Agent Skills specification; the others load. A skill is named by its directory, offered as
//! `<package>:<directory>` (Agent Plugins Extended §18.1.5 item 5).

use serde::Serialize;

use super::locations::{FixedLocation, Kind, fixed_location};
use crate::format::core::skill::check_core_skill;
use crate::format::issues::{PluginIssue, error, warning};
use crate::format::listing::list_sorted;
use crate::format::locate::{LocateContext, locate};
use crate::js::{JsValue, promise_all};
use crate::loading::readers_text::PluginTextFile;
use crate::ports::BoxFuture;
use crate::ports::files::{DirectoryEntry, TextFileReader};

/// The skills read, and what was wrong.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct CoreSkillsRead {
    /// The skills.
    pub skills: Vec<PluginTextFile>,
    /// What was found.
    pub issues: Vec<PluginIssue>,
}

const DIRECTORY: &str = "skills";

/// One child of `skills/`: its skill, the issues it gives, or nothing when it holds no `SKILL.md`.
async fn child_skill(
    context: &LocateContext,
    entry: &DirectoryEntry,
    plugin_name: &str,
) -> CoreSkillsRead {
    let path = format!("{DIRECTORY}/{}/SKILL.md", entry.name);
    let found = locate(context, DIRECTORY, &path, Some(true)).await;
    if let Some(issue) = found.issue {
        return CoreSkillsRead {
            skills: Vec::new(),
            issues: vec![PluginIssue { path, ..issue }],
        };
    }
    if !found.stat.is_some_and(|stat| stat.is_file) {
        return CoreSkillsRead::default();
    }
    let text = context
        .files
        .read_text(&path, context.signal.as_ref())
        .await
        .unwrap_or_default();
    let check = check_core_skill(&text, &entry.name);
    let extra: Vec<PluginIssue> = check
        .unknown_fields
        .iter()
        .map(|field| {
            warning(
                &path,
                &format!("Field \"{field}\" is not part of the Agent Skills specification"),
            )
        })
        .collect();
    if !check.problems.is_empty() {
        let mut issues = vec![error(
            &path,
            &format!(
                "Skill \"{}\" is skipped: {}",
                entry.name,
                check.problems.join("; ")
            ),
        )];
        issues.extend(extra);
        return CoreSkillsRead {
            skills: Vec::new(),
            issues,
        };
    }
    let skill = PluginTextFile {
        path: Some(path),
        name: entry.name.clone(),
        qualified_name: format!("{plugin_name}:{}", entry.name),
        text,
        directory: Some(format!("{DIRECTORY}/{}", entry.name)),
        alone: None,
        entry: JsValue::Undefined,
    };
    CoreSkillsRead {
        skills: vec![skill],
        issues: extra,
    }
}

/// The skills of `skills/`, in name order.
pub async fn read_core_skills(context: &LocateContext, plugin_name: &str) -> CoreSkillsRead {
    match fixed_location(context, DIRECTORY, Kind::Directory).await {
        FixedLocation::Absent { .. } => return CoreSkillsRead::default(),
        FixedLocation::Issue { issue } => {
            return CoreSkillsRead {
                skills: Vec::new(),
                issues: vec![issue],
            };
        }
        FixedLocation::Present { .. } => {}
    }
    let entries = list_sorted(&context.files, DIRECTORY, context.signal.as_ref())
        .await
        .unwrap_or_default();
    let reads: Vec<BoxFuture<'_, CoreSkillsRead>> = entries
        .iter()
        .filter(|entry| !entry.is_file)
        .map(|entry| -> BoxFuture<'_, CoreSkillsRead> {
            Box::pin(child_skill(context, entry, plugin_name))
        })
        .collect();
    let mut read = CoreSkillsRead::default();
    for one in promise_all(reads).await {
        read.skills.extend(one.skills);
        read.issues.extend(one.issues);
    }
    read
}

crate::registry::exports! {
    "loading/core/skills" {
        "readCoreSkills" => async ok |context: LocateContext, plugin_name: String|
            read_core_skills(&context, &plugin_name).await;
    }
}
