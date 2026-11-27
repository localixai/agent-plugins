//! Port of `src/hooks/input/facts-model.ts` at sha256:1afc367d516b.
//!
//! Facts of the model-switch events (hooks.md, "PreModelSwitch input"). `PostModelSwitch` has the
//! same fields, and its `source` also names the switches Claude Code makes on its own.

use std::sync::LazyLock;

use super::facts_shared::COUNT;
use crate::zod::{self, Schema, Shape, fields};

static COST: LazyLock<Shape> = LazyLock::new(|| {
    fields! {
        "from_model" => zod::string(),
        "to_model" => zod::string(),
        // "a string, or `null` when the request was for the default model".
        "requested_model" => zod::string().nullable(),
        "context_tokens" => COUNT.clone(),
        "prompt_cache_warm" => zod::boolean(),
        "cache_ttl" => zod::r#enum(["5m", "1h"]),
        "estimated_cache_write_usd" => zod::number(),
        "pricing" => zod::r#enum(["configured", "catalog", "default"]),
    }
});

/// The facts of `PreModelSwitch`.
pub static PRE_MODEL_SWITCH_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        ..COST.clone(),
        "source" => zod::r#enum(["command", "picker", "sdk"]),
    })
});

/// The facts of `PostModelSwitch`.
pub static POST_MODEL_SWITCH_FACTS: LazyLock<Schema> = LazyLock::new(|| {
    zod::object(fields! {
        ..COST.clone(),
        "source" => zod::r#enum(["command", "picker", "sdk", "auto", "resume"]),
    })
});
