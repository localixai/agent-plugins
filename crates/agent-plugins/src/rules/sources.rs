//! Ported from `src/rules/sources.ts`; only the core uses it now, so it has no wrapper.

/// Whether a source is a settings file or object, as against the command line or the session.
/// Claude Code skips an `mcp__` rule with parentheses only "when it loads a settings file"; a deny
/// rule passed with `--disallowedTools` is how to match a parameter of an MCP tool (permissions.md,
/// "Match by input parameter"). The policy an embedding host passes is settings content, so it is
/// read like a file (spec/judgments.md). A registry typed over the source kinds: a kind added to
/// `RuleSource` fails the typecheck here.
pub const IS_SETTINGS_SOURCE: [(&str, bool); 8] = [
    ("managed", true),
    ("embedder-managed", true),
    ("project", true),
    ("local", true),
    ("user", true),
    ("flag-settings", true),
    ("cli", false),
    ("session", false),
];

/// Whether a source of this kind is a settings file or object.
#[must_use]
pub fn is_settings_source(kind: &str) -> bool {
    IS_SETTINGS_SOURCE
        .iter()
        .any(|(name, settings)| *name == kind && *settings)
}
