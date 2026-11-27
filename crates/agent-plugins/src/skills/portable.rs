//! Port of `src/skills/portable.ts`, switched.
//!
//! Which frontmatter fields each distribution path takes (skills.md, "Using skill frontmatter outside
//! Claude Code"): "Claude Code accepts every field in the table above. Outside Claude Code, you can use
//! only the fields in the Agent Skills spec", and a field the spec doesn't allow makes packaging or
//! upload fail "with a hard error instead of ignoring the field". The kit reads the file; the host (a
//! packaging step, an uploader) shows the error.

use serde::Deserialize;

use crate::format::frontmatter::parse_frontmatter;
use crate::format::issues::{PluginIssue, error};
use crate::format::skill_fields::SKILL_FIELDS;
use crate::js::{compare_code_units, in_property_order};

/// The six fields of the Agent Skills spec: "name, description, license, compatibility, metadata, allowed-tools".
pub const AGENT_SKILLS_FIELDS: [&str; 6] = [
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// The ways a skill is distributed.
pub const DISTRIBUTION_PATHS: [&str; 4] = [
    "claude-code",
    "claude-ai-upload",
    "skills-api",
    "package-skill",
];

/// One distribution path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DistributionPath {
    /// Claude Code itself.
    ClaudeCode,
    /// An upload to claude.ai.
    ClaudeAiUpload,
    /// The Skills API.
    SkillsApi,
    /// The packaging script.
    PackageSkill,
}

impl DistributionPath {
    /// The fields the path takes, one row of the docs' table each.
    const fn accepted(self) -> &'static [&'static str] {
        match self {
            Self::ClaudeCode => &SKILL_FIELDS,
            Self::ClaudeAiUpload | Self::SkillsApi | Self::PackageSkill => &AGENT_SKILLS_FIELDS,
        }
    }

    /// Whether the path stops at a field it does not take; Claude Code ignores one it does not recognize.
    const fn refuses_unknown(self) -> bool {
        match self {
            Self::ClaudeCode => false,
            Self::ClaudeAiUpload | Self::SkillsApi | Self::PackageSkill => true,
        }
    }
}

/// The hard error of an upload or packaging for a `SKILL.md`, or none. The message is the docs':
/// `Unexpected key(s) in SKILL.md frontmatter: argument-hint. Allowed properties are: allowed-tools,
/// compatibility, description, license, metadata, name`. Judgment (spec/judgments.md): several
/// unexpected keys are listed in the order they appear, comma separated.
#[must_use]
pub fn frontmatter_issues(text: &str, path: DistributionPath) -> Vec<PluginIssue> {
    if !path.refuses_unknown() {
        return Vec::new();
    }
    let allowed = path.accepted();
    let fields = in_property_order(parse_frontmatter(text).fields);
    let unexpected: Vec<&str> = fields
        .keys()
        .map(String::as_str)
        .filter(|key| !allowed.contains(key))
        .collect();
    if unexpected.is_empty() {
        return Vec::new();
    }
    let mut sorted = allowed.to_vec();
    sorted.sort_by(|left, right| compare_code_units(left, right));
    let message = format!(
        "Unexpected key(s) in SKILL.md frontmatter: {}. Allowed properties are: {}",
        unexpected.join(", "),
        sorted.join(", ")
    );
    vec![error("SKILL.md", &message)]
}

crate::registry::exports! {
    "skills/portable" {
        "AGENT_SKILLS_FIELDS" => ok || AGENT_SKILLS_FIELDS;
        "DISTRIBUTION_PATHS" => ok || DISTRIBUTION_PATHS;
        "frontmatterIssues" => ok |text: String, path: DistributionPath| frontmatter_issues(&text, path);
    }
}
