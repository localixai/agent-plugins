//! Shell commands as the rules read them: dialects, keywords, aliases, assignments
//! (`src/rules/bash/` in the npm package).

#[expect(
    clippy::module_inception,
    reason = "the port of `src/rules/bash/bash.ts` sits at its source's path"
)]
pub mod bash;
pub mod dialect;
pub mod exec_wrappers;
pub mod heredoc;
pub mod keywords;
pub mod powershell;
pub mod powershell_aliases;
pub mod safe_env;
pub mod scanner;
pub mod shell;
pub mod split;
pub mod text_pattern;
pub mod unquote;
pub mod wildcard;
pub mod wildcard_warning;
pub mod words;
pub mod wrappers;
