//! Port of `src/protocol/mods-limits.ts`, switched.
//!
//! Every number of the mods reference's Limits table (plugins/mods/reference.md, "Limits") as a named
//! constant, so no module of the kit writes one of them as a literal. Pure data: what each limit
//! does is the code of the module that owns the limited behaviour.

use serde::Serialize;

const MIB: u32 = 1024 * 1024;

/// The mods reference's Limits table, one member per limit, in the table's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModLimits {
    /// A hook's own execution time for one event, not counting time inside `next` or a mods API call other than `$.clock.sleep`.
    pub hook_ms: u32,
    /// The same limit for a `prompt.edit` hook.
    pub prompt_edit_ms: u32,
    /// A `.catch` handler's own execution time.
    pub catch_ms: u32,
    /// How long a hook may keep running after its `next.signal` aborted before it is reported as lingering (claude-code/index.d.ts:5428, `HookBudget.lingerMs`).
    pub linger_ms: u32,
    /// All `session.end` hooks together, unless the SessionEnd hook budget is changed (hooks.md, "SessionEnd input").
    pub session_end_ms: u32,
    /// `$.process.run` timeout by default, and at most.
    pub run_timeout_ms: u32,
    /// `$.process.run` timeout, at most.
    pub run_timeout_max_ms: u32,
    /// `$.process.run` `stdout` and `stderr`, each.
    pub output_bytes: u32,
    /// `$.model.complete` `maxTokens` by default, and at most.
    pub max_tokens: u32,
    /// `$.model.complete` `maxTokens`, at most.
    pub max_tokens_cap: u32,
    /// `$.fs.read` and `$.fs.write`, one file.
    pub file_bytes: u32,
    /// A `$.http.fetch` request body, in characters; the response body, in bytes.
    pub http_request_chars: u32,
    /// A `$.http.fetch` response body, in bytes.
    pub http_response_bytes: u32,
    /// One `$.http.fetch` call, redirects and body included.
    pub http_ms: u32,
    /// Redirects one `$.http.fetch` call follows.
    pub redirects: u32,
    /// A `drop` reason or a `config.set` `deny` reason, in characters.
    pub reason_chars: u32,
    /// Text in one tree: the first characters drawn.
    pub tree_text: u32,
    /// A `Code`'s `language` or `path`, a `Select` option's `value`, a `Client`'s `module`.
    pub field_chars: u32,
    /// A `Link`'s `href`.
    pub href_chars: u32,
    /// Nesting depth of one tree, below the root element.
    pub tree_depth: u32,
    /// Elements and text children in one tree.
    pub tree_nodes: u32,
    /// `Raster` cells in one tree.
    pub raster_cells: u32,
    /// PNG or RGBA bytes of all the `Image`s in one tree.
    pub image_bytes: u32,
    /// `$.store`: JSON in total.
    pub store_bytes: u32,
    /// `$.session.messages()`: the newest entries.
    pub messages: u32,
    /// How long a toast stays, in milliseconds.
    pub toast_ms: u32,
    /// `$.ui.invalidate('ui.render')` redraws a second, and in the terminal for the visible pane, the expanded band and the hint line.
    pub invalidate_per_second: u32,
    /// The same, in the terminal.
    pub invalidate_per_second_terminal: u32,
    /// A pane opened without the user asking: placed from this many terminal columns, and from the second after they opened it once.
    pub pane_columns: u32,
    /// Terminal columns from which a pane opens, from the second time the user opened it.
    pub pane_columns_after_open: u32,
    /// Command, tool, subagent type and pane names, and the `id` of a placed row or a backdrop.
    pub name_chars: u32,
    /// Rows from `$.ui.place` that one mod has in a conversation at one time.
    pub placed_rows: u32,
    /// Scopes nested inside one another in one file of a hooks module.
    pub scopes: u32,
    /// Nesting depth of a mods API call's arguments, of a changed event passed to `next`, and of a hook's return value.
    pub nesting: u32,
    /// One `claude plugin test` test.
    pub test_ms: u32,
}

/// The mods reference's Limits table.
pub const MOD_LIMITS: ModLimits = ModLimits {
    hook_ms: 10_000,
    prompt_edit_ms: 50,
    catch_ms: 1000,
    linger_ms: 5000,
    session_end_ms: 1500,
    run_timeout_ms: 30_000,
    run_timeout_max_ms: 600_000,
    output_bytes: 4 * MIB,
    max_tokens: 1024,
    max_tokens_cap: 64_000,
    file_bytes: 4 * MIB,
    http_request_chars: 4 * MIB,
    http_response_bytes: 4 * MIB,
    http_ms: 30_000,
    redirects: 5,
    reason_chars: 4096,
    tree_text: 100_000,
    field_chars: 10_000,
    href_chars: 2048,
    tree_depth: 32,
    tree_nodes: 20_000,
    raster_cells: 262_144,
    image_bytes: 2 * MIB,
    store_bytes: 4 * MIB,
    messages: 4096,
    toast_ms: 4000,
    invalidate_per_second: 10,
    invalidate_per_second_terminal: 30,
    pane_columns: 144,
    pane_columns_after_open: 110,
    name_chars: 64,
    placed_rows: 500,
    scopes: 2000,
    nesting: 28_000,
    test_ms: 5000,
};

/// Command, tool, subagent type and pane names: letters, digits, `_` and `-`, up to `nameChars`.
#[must_use]
pub fn mod_name_pattern() -> String {
    format!("^[A-Za-z0-9_-]{{1,{}}}$", MOD_LIMITS.name_chars)
}

crate::registry::exports! {
    "protocol/mods-limits" {
        "MOD_LIMITS" => ok || MOD_LIMITS;
        "MOD_NAME_PATTERN" => ok || crate::registry::RegExp { source: mod_name_pattern(), flags: String::new() };
    }
}
