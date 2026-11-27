//! Port of `src/convert/to-core/skills.ts`, switched.
//!
//! A Claude Code plugin's skills in a core client (Agent Plugins Extended §18.4.3 item 2). `skills/`
//! is shared, so nothing is written: a skill converts when it is `skills/<name>/SKILL.md` and
//! conforms to Agent Skills (core §7.1). A skill elsewhere (a manifest `skills` path, a root
//! `SKILL.md`) is reported as not converted; so is one a core client would skip, which the converter
//! must never offer as converted. A skill's Claude Code fields and its §4.8 body features are
//! reported: a core client ignores the fields and shows the text as written. Judgment
//! (spec/judgments.md): skills are not copied under `skills/` and fields are not stripped, since
//! either would rewrite the source form.

use crate::convert::notes::{ConversionNote, not_converted, reported};
use crate::convert::skill_body::expanded_features;
use crate::format::core::skill::check_core_skill;
use crate::loading::readers_text::PluginTextFile;

const SKILLS: &str = "skills/";

fn skill_notes(skill: &PluginTextFile) -> Vec<ConversionNote> {
    let path = skill.path.clone().unwrap_or_else(|| {
        format!(
            "{}/SKILL.md",
            skill.directory.as_deref().unwrap_or(&skill.name)
        )
    });
    let home = format!("{SKILLS}{}", skill.name);
    let at_home = skill.directory.as_deref() == Some(home.as_str())
        && skill.path.as_deref() == Some(format!("{home}/SKILL.md").as_str());
    if skill.alone == Some(true) || !at_home {
        return vec![not_converted(
            path,
            None,
            format!(
                "skill \"{}\" is not under skills/, the only place a core client reads",
                skill.name
            ),
        )];
    }
    let check = check_core_skill(&skill.text, &skill.name);
    if !check.problems.is_empty() {
        return vec![not_converted(
            path,
            None,
            format!(
                "a core client skips skill \"{}\": {}",
                skill.name,
                check.problems.join("; ")
            ),
        )];
    }
    let features = expanded_features(&skill.text);
    let mut notes = Vec::new();
    if !check.unknown_fields.is_empty() {
        let fields = check.unknown_fields.join(", ");
        notes.push(reported(
            path.clone(),
            Some(fields.clone()),
            format!(
                "skill \"{}\": a core client ignores {fields}, outside the six Agent Skills fields",
                skill.name
            ),
        ));
    }
    if !features.is_empty() {
        notes.push(reported(
            path,
            None,
            format!(
                "skill \"{}\" holds {}, which a core client shows as written",
                skill.name,
                features.join("; ")
            ),
        ));
    }
    notes
}

/// The notes of a Claude Code plugin's skills in a core client.
#[must_use]
pub fn core_skill_notes(skills: &[PluginTextFile]) -> Vec<ConversionNote> {
    skills.iter().flat_map(skill_notes).collect()
}

crate::registry::exports! {
    "convert/to-core/skills" {
        "coreSkillNotes" => ok |skills: Vec<PluginTextFile>| core_skill_notes(&skills);
    }
}
