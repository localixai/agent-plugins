//! Port of `src/marketplace/plugin-content.ts`, switched.
//!
//! What counts as plugin content at the top level of a directory a source delivers: the command
//! source's printed directory and an archive's root (plugins/marketplace-reference.md, "Output that
//! fails the install or update").

/// "Plugin content at its top level, such as a `.claude-plugin/` directory or a `skills/`, `commands/`, `agents/`, or `hooks/` directory."
pub const PLUGIN_CONTENT_DIRECTORIES: [&str; 5] =
    [".claude-plugin", "skills", "commands", "agents", "hooks"];

crate::registry::exports! {
    "marketplace/plugin-content" {
        "PLUGIN_CONTENT_DIRECTORIES" => ok || PLUGIN_CONTENT_DIRECTORIES;
    }
}
