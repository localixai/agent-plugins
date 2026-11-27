//! Ported from `src/convert/to-claude-code/skills.ts`; only the core uses it now, so it has no wrapper.
//!
//! A core package's skills in Claude Code (Agent Plugins Extended §18.4.2 item 2): `skills/` is used
//! as it is, nothing is written; a skill whose body holds text Claude Code expands or runs is
//! reported, since Claude Code will replace or run what a core client shows as written.

use crate::convert::notes::{ConversionNote, reported};
use crate::convert::skill_body::expanded_features;
use crate::loading::readers_text::PluginTextFile;

/// A core package's skills in Claude Code: the notes of the skills whose body Claude Code expands.
#[must_use]
pub fn claude_code_skill_notes(skills: &[PluginTextFile]) -> Vec<ConversionNote> {
    skills
        .iter()
        .flat_map(|skill| {
            let features = expanded_features(&skill.text);
            let path = skill
                .path
                .clone()
                .unwrap_or_else(|| format!("skills/{}/SKILL.md", skill.name));
            if features.is_empty() {
                Vec::new()
            } else {
                vec![reported(
                    path,
                    None,
                    format!(
                        "skill \"{}\" holds {}, which Claude Code expands or runs and a core client shows as written",
                        skill.name,
                        features.join("; ")
                    ),
                )]
            }
        })
        .collect()
}
