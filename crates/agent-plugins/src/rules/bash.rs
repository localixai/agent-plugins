//! Shell commands as the rules read them: dialects, keywords, aliases, assignments
//! (`src/rules/bash/` in the npm package).

pub mod dialect;
pub mod heredoc;
pub mod keywords;
pub mod powershell_aliases;
pub mod safe_env;
pub mod scanner;
pub mod split;
pub mod text_pattern;
pub mod unquote;
pub mod wildcard;
pub mod words;
pub mod wrappers;
