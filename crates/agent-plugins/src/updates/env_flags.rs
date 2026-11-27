//! Ported from `src/updates/env-flags.ts`; only the core uses it now, so it has no wrapper.
//!
//! The variables that stop updates and traffic, read through the `Environment` port (docs/
//! ARCHITECTURE.md, L3). env-vars.md: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is "any non-empty
//! value"; the others are "Set to `1`".

use crate::format::env_flag::is_truthy_env;
use crate::ports::environment::Environment;

/// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC`.
pub const DISABLE_NONESSENTIAL_TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";

/// Whether the variable holds any non-empty value.
pub async fn is_set(env: &dyn Environment, name: &str) -> bool {
    env.get(name).await.unwrap_or_default() != ""
}

/// Whether a "set to `1`" variable is on. The docs say `1`; its table reads `DEBUG` as on for `1`,
/// `true`, `yes` and `on`, and the kit reads these four spellings, in any case, the same way
/// (spec/judgments.md). Anything else, `0` and the empty string included, is off.
pub async fn is_on(env: &dyn Environment, name: &str) -> bool {
    is_truthy_env(env.get(name).await.as_deref())
}
