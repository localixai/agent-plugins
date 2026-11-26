//! The differential tests: every ported function answers each call the npm package's own tests
//! made of its namesake, as `scripts/rust/record.ts` recorded them in `tests/vectors/` (with the
//! requirement ids those tests prove). One module here per module of `src/`.

#![expect(
    clippy::expect_used,
    clippy::disallowed_methods,
    clippy::disallowed_types,
    reason = "test support: it reads the vectors, JSON text the recorder wrote, from disk with \
              serde_json and says a line that is no JSON is a bug"
)]

mod support;

mod js;
mod registry;
