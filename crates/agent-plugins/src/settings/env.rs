//! Layer 4: the `env` key of the settings files (`src/settings/env/` in the npm package): the
//! variables Claude Code ignores, and what the files come to for a session and its subprocesses.

pub mod ignored_variables;
pub mod resolve_env;
pub mod settings_env;
