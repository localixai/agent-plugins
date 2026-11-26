//! Shell commands as the rules read them: dialects, keywords, aliases, assignments
//! (`src/rules/bash/` in the npm package).

pub mod dialect;
pub mod keywords;
pub mod powershell_aliases;
pub mod safe_env;
pub mod text_pattern;
