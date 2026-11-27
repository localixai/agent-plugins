//! Port of `src/updates/env-flags.ts`, switched.
//!
//! The variables that stop updates and traffic, read through the `Environment` port (docs/
//! ARCHITECTURE.md, L3). env-vars.md: `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` is "any non-empty
//! value"; the others are "Set to `1`".

use crate::format::env_flag::is_truthy_env;
use crate::ports::environment::Environment;
use crate::registry::Port;

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

crate::registry::exports! {
    "updates/env-flags" {
        "DISABLE_NONESSENTIAL_TRAFFIC" => ok || DISABLE_NONESSENTIAL_TRAFFIC;
        "isSet" => async ok |env: Port, name: String| is_set(&env, &name).await;
        "isOn" => async ok |env: Port, name: String| is_on(&env, &name).await;
    }
}
