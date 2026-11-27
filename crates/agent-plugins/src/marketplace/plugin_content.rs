//! Ported from `src/marketplace/plugin-content.ts`; only the core uses it now, so it has no wrapper.
//!
//! What counts as plugin content at the top level of a directory a source delivers: the command
//! source's printed directory and an archive's root (plugins/marketplace-reference.md, "Output that
//! fails the install or update").

/// "Plugin content at its top level, such as a `.claude-plugin/` directory or a `skills/`, `commands/`, `agents/`, or `hooks/` directory."
pub const PLUGIN_CONTENT_DIRECTORIES: [&str; 5] =
    [".claude-plugin", "skills", "commands", "agents", "hooks"];
