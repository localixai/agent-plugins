//! zod 4 (the version in `package.json`, 4.4.3) for the core: the validator the package's
//! modules check their input with, answering as zod answers. Its issues reach the
//! `PluginIssue`s users read (`found.message`, `found.path`), so a schema here gives zod's
//! output (defaults applied, transforms run, unknown keys dropped, kept or refused as each
//! object kind does) and zod's issues, in zod's order, with zod's `code`, `path`, `message` and
//! the members each code adds, its English messages word for word. `tests/differential/js/zod.rs`
//! holds it to zod itself, run by `scripts/rust/js-oracle/zod.ts`.
//!
//! A TypeScript schema maps onto Rust line by line: a builder or a method keeps its name in
//! snake case (`z.looseObject` is `zod::loose_object`, `z.enum` is `zod::r#enum`), a shape is
//! `zod::fields! { "key" => schema, ..spread }`, a builder's params are `.error(params)` and a
//! check's the second argument of its `_with` twin (`.min(1, wrongText)` is
//! `.min_with(1.0, WRONG_TEXT)`), a refinement's its second argument (`()` for none). Params are a
//! message (`{ error: '…' }`, `{ message: '…' }`), [`error_fn`] (`{ error: (issue) => … }`), with
//! [`Params::path`] and [`Params::abort`]. A schema a module builds once is a `LazyLock<Schema>`
//! (or a function that builds it); a schema is shared, cloned in O(1).
//!
//! ```ignore
//! // const verdict = z.object({ ok: z.boolean(), reason: z.string().optional() });
//! static VERDICT: LazyLock<Schema> = LazyLock::new(|| {
//!     zod::object(zod::fields! { "ok" => zod::boolean(), "reason" => zod::string().optional() })
//! });
//! // verdict.safeParse(value).data
//! VERDICT.safe_parse(&value).ok()
//! ```
//!
//! What the package uses of zod, counted over the 47 modules of `src/` that import it outside
//! `src/mods/` (the 40 there stay TypeScript), which is what this module provides:
//!
//! - builders: `z.string` 164, `z.object` 102, `z.literal` 44, `z.array` 33, `z.enum` 31,
//!   `z.boolean` 25, `z.record` 20, `z.number` 15, `z.looseObject` 15, `z.unknown` 13,
//!   `z.union` 10, `z.url` 2, `z.strictObject` 2, `z.int` 2, `z.discriminatedUnion` 2,
//!   `z.custom` 2, `z.json` 1, and `z.NEVER` 1 ([`NEVER`]);
//! - methods: `.optional` 80, `.exactOptional` 54, `.safeParse` 30, `.refine` 14, `.min` 11,
//!   `.max` 10, `.nullable` 4, `.int` 4, `.regex` 3, `.positive` 3, `.parse` 3, `.array` 2,
//!   `.partial` 2, `.check` 2, `.shape` 2, `.transform` 1, `.superRefine` 1, `.required` 1,
//!   `.nonnegative` 1, `.finite` 1 (nothing in zod 4);
//! - in `src/mods/` only, provided too: `.default` 11, `.catch` 4, `.extend` 2, `z.instanceof` 1;
//! - error options: `{ error: '…' }`, `{ message: '…' }`, `{ error: (issue) => … }` reading
//!   `issue.input`; a refinement's `path`;
//! - issues read: `error.issues`, each one's `path` (joined, or mapped with `String`),
//!   `message` and `code`; `context.issues.push({ code, keys | message, input, continue })` in a
//!   `.check`, `context.addIssue({ code, path?, message })` in a `.superRefine` and a
//!   `.transform`; `z.ZodError`, `z.core.$ZodIssue` and `z.ZodSafeParseResult` as types.
//!
//! Where it answers differently, each a decision: a member and a key are own ones (`object.rs`);
//! an object's `length` that is an array or an object reads as `NaN` (`checks.rs`); a
//! discriminator value two options take picks the first, where zod refuses the schema when it is
//! first used (`union.rs`); a `.regex` pattern is read by Rust's `regex`, so it is written as
//! JavaScript means it (RS6); `.partial()` and `.extend()` of an object with checks keep them,
//! where zod throws.

mod build;
mod checks;
mod context;
mod issue;
mod locale;
mod methods;
mod object;
mod params;
mod run;
mod schema;
mod union;

pub use build::{
    array, boolean, custom, discriminated_union, r#enum, instanceof, int, json, lazy, literal,
    loose_object, never, null, number, object, record, strict_object, string, union, unknown, url,
};
pub use context::{CatchContext, Context};
pub use issue::{Issue, RawIssue, ZodError, issue};
pub use params::{ErrorMap, Params, Shape, error_fn};
pub use schema::Schema;

/// `zod::fields! { "key" => schema, ..spread }`: a shape, as an object literal writes one.
pub use crate::__zod_fields as fields;

use crate::js::JsValue;

/// `z.NEVER`: what a `.transform` returns after it added an issue; the output is not read.
pub const NEVER: JsValue = JsValue::Undefined;

#[cfg(test)]
mod tests;
