//! Port of `src/format/token-estimate.ts`, switched.
//!
//! Judgment (spec/judgments.md): the docs count tokens (the skill listing budget, re-attached
//! skill content, MCP output limits) and name no tokenizer, so wherever the crate counts tokens
//! without one it takes four characters to a token, a character being a code point, as the skill
//! listing counts characters. A host with a tokenizer passes its own counter where one is taken.

/// Characters to a token.
pub const CHARS_PER_TOKEN: usize = 4;

/// The tokens of `text` by that estimate, rounded up.
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.chars().count().div_ceil(CHARS_PER_TOKEN)
}

crate::registry::exports! {
    "format/token-estimate" {
        "CHARS_PER_TOKEN" => ok || CHARS_PER_TOKEN;
        "estimateTokens" => ok |text: String| estimate_tokens(&text);
    }
}
