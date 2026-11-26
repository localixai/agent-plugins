# AGENTS.md

The operating guide for people and coding agents in `@localix-ai/agent-plugins`: a library that
adds Claude Code plugin support to any agent. Read this file first; it is short on purpose.

## The one rule: the spec, not memory

The kit implements **Agent Plugins Extended** (https://github.com/localixai/agent-plugins-extended)
at the revision `spec/SOURCE.json` pins, and nothing it remembers or guesses. The spec mines Claude
Code (its docs, its mods declarations, its validator and loader run as a black box, the corpus of
real plugins); this repository never does, and syncs what the spec publishes.

- **E1** The contract is the spec at the pinned revision. Requirement ids, judgments (`J<n>.<m>`)
  and the snapshots the tests read (`spec/requirements.json`, `spec/mods-declarations.json`,
  `spec/docs-tables.json`, `spec/tools-table.json`, the scope files, the oracle outcomes in
  `spec/oracle/`) come from it through `bun run spec:sync`, never by
  hand; `check:spec` (SP4) fails on an edited or stale copy. Cite the section a test proves in the
  test file's header (the spec's chapter, or the docs page and section the requirement names).
- **E1a** This repository never mines Claude Code. A change of behaviour the spec does not state
  goes to the spec first (a requirement, a judgment or a deviation in agent-plugins-extended);
  sync the revision that states it, then implement here.
- **E2** Shapes come from the official types (`@anthropic-ai/claude-agent-sdk`) of the release the
  spec targets, re-exported in `src/protocol/official.ts`. Never declare a type for something the
  SDK already types.
- **E2a** The mods surface (events and their fields, `$` and its members, `next`, the elements and
  their props, the state helpers, `claude-code/testing`, the built-in tools' inputs) is the spec's
  `spec/mods-declarations.json`, read by the tests and never by hand; cite
  `claude-code/index.d.ts:<line>` of the release. A behaviour only the declarations give is tested
  outside `spec(...)`: compatibility measures the requirements.
- **E3** Every behaviour the spec requires is proven by a test under `spec(id)` for a requirement in
  `spec/requirements.json`. Never invent an id.
- **E4** `spec/judgments.md` records only the decisions the spec leaves to an implementation (how
  the kit shapes an API, orders work, words an issue the spec does not word), each linked to the
  spec's J-id where one exists. A judgment is tested outside `spec(...)` and never counts as
  compatibility.
- **E5** Report compatibility only from `docs/COMPATIBILITY.md` (`bun run compat`): the share
  proven overall and per category. Never state a percentage or "supported" from your own reading.
- **E5a** What Claude Code does where its docs are silent or disagree (a validator's wording, a
  loading outcome) is what the spec's judgments and deviations state, with the oracle runs they
  rest on; the kit's tests of it run outside `spec(...)`. No script here starts Claude Code: the
  spec runs it and records the outcomes (`spec/oracle/`, synced; the corpus ones read from the
  spec's `extend/.cache/oracle/`), and the differential checks here (`corpus*`, `oracle:*`,
  `bench:validate`, the oracle tests; docs/TESTING.md, "Oracle runs") compare the kit with them.
  They measure the kit, never the contract: a difference they show that the spec does not settle
  goes to the spec (E1a).
- **E6** Another host's behaviour (Localix, a blog, an issue) is a lead for the spec, never
  evidence. Port code from Localix only after the spec states the behaviour.
- **E7** The kit follows the spec's latest revision, not just a pinned one. `bun run spec:behind`
  fails while the pin is behind the spec's `main` (CI runs it every week, `spec-behind.yml`, and
  a red run is work to do, not noise). Check it at the start of every session that changes
  behaviour. Catching up is one change in order: `bun run spec:sync main`, the TypeScript until
  `bun run check` passes, then the Rust crate until `crates/agent-plugins/scripts/check` passes
  (docs/RUST.md, "When the spec moves"). Both halves move together: neither is left on an older
  revision, and the crate's checks fail until it has caught up.

## Docs

| Doc                       | Read it before you touch                                                                                       |
| ------------------------- | -------------------------------------------------------------------------------------------------------------- |
| `docs/API.md`             | anything public: the API, its principles, what is done and what is planned; `docs/api/` has one row per export |
| `docs/GETTING-STARTED.md` | a host's first steps: install, the type peers, type-checking, wiring the ports                                 |
| `docs/ARCHITECTURE.md`    | where code goes: layers, invariants, folders                                                                   |
| `docs/SPEC.md`            | the spec this kit follows, syncing it, proof, official types, moving to a new spec revision                    |
| `docs/TESTING.md`         | tests: test first, layout, determinism, what a spec test asserts                                               |
| `docs/SECURITY.md`        | where the kit is stricter than Claude Code, and the risks it leaves to the host                                |
| `docs/COMPATIBILITY.md`   | generated by `bun run compat`; never edit                                                                      |
| `docs/RUST.md`            | the Rust crate in `crates/agent-plugins`: how it is held to the package, how it follows a spec change, RS1–RS9 |
| `spec/judgments.md`       | where the kit decides what the spec leaves to an implementation (E4)                                           |
| `spec/SOURCE.json`        | the spec revision `spec/` is synced from, and each file's hash; written by `bun run spec:sync`                 |
| `CHANGELOG.md`            | every `feat`, `fix`, `perf` commit (G2)                                                                        |

Update the doc that owns a behaviour in the same commit as the code.

## Commands

| Task                               | Command                                                |
| ---------------------------------- | ------------------------------------------------------ |
| Install                            | `bun install` (also installs the git hooks)            |
| Build the Rust core for the tests  | `bun run build:wasm` (before tests, types, a build)    |
| Sync the spec                      | `bun run spec:sync [ref]` (default: the pinned commit) |
| Is the spec ahead of the pin (E7)  | `bun run spec:behind`                                  |
| Tests                              | `bun test`, or one file: `bun test tests/format/…`     |
| Tests and the compatibility report | `bun run compat`                                       |
| Everything (pre-push and CI)       | `bun run check` (never `--no-verify`)                  |
| Format / lint                      | `bun run format`, `bun run lint` (`lint:fix` to apply) |
| Types                              | `bun run typecheck`                                    |
| Build                              | `bun run build`                                        |
| The Rust crate (docs/RUST.md)      | `crates/agent-plugins/scripts/check`                   |
| Record the Rust crate's vectors    | `bun scripts/rust/record.ts [tests/<file>…]`           |

## How to add a behaviour

1. Find what the spec states for it: the chapter, and the requirement ids in
   `spec/requirements.json`. When the spec does not state it, take it to the spec first (E1a) and
   `bun run spec:sync <ref>` once the spec does.
2. Write the failing test in `tests/<module>/` under `spec(ids)`, citing the section.
3. Implement in the right layer (docs/ARCHITECTURE.md), using the official types.
4. `bun run compat`, then `bun run check`. Commit the code, tests, docs and report together.

## Rules

Each rule is checked by a gate; a gate never gets a baseline or an exception without the owner.

| Id     | Rule                                                                                                                                                                      | Gate                                 |
| ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------ |
| L1     | A `src/` module imports only from its own folder and lower layers                                                                                                         | `check:layers`                       |
| L2     | `@anthropic-ai/*` only in `src/protocol/`, only `import type`                                                                                                             | `check:layers`                       |
| L3     | `src/` uses no `node:*`, `bun`, `Bun`, `process`, `Date`, `fetch` or timer: every effect is a port                                                                        | `check:layers`                       |
| L4     | `src/` never imports `tests/` or `scripts/`                                                                                                                               | `check:layers`                       |
| S1     | No file over 300 lines; split by responsibility                                                                                                                           | `check:size`                         |
| K1     | No `switch`: what differs per member is a strategy in a registry typed over the union                                                                                     | `lint` (quality)                     |
| K2     | Never compare a kind-like field to a string, nor pass a member to a generic guard as one                                                                                  | `lint` (quality)                     |
| Q1     | Complexity ≤ 10, cognitive ≤ 15, depth ≤ 3, ≤ 4 params, ≤ 25 statements, ≤ 80 lines per function                                                                          | `lint` (quality)                     |
| T1     | No focused or switched-off test                                                                                                                                           | `check:tests`                        |
| T2     | Tests only in `tests/<module>/`, support in `tests/support/`                                                                                                              | `check:test-layout`                  |
| SP1–5  | Requirements valid; test ids exist; the SDK is the spec's release; `spec/` is the pinned spec, byte for byte; the snapshots parse, and the tools table copy is the spec's | `check:spec`                         |
| C1     | Every test passes and `docs/COMPATIBILITY.md` is current                                                                                                                  | `compat --check`                     |
| DOC1–3 | Paths named in docs exist; every doc has a row above; every export of every entry has a row in its `docs/api/` table, and every name the docs import exists               | `check:docs`                         |
| D1     | No unused file, export or dependency                                                                                                                                      | `check:deadcode`                     |
| G1–2   | Conventional Commits; `feat`/`fix`/`perf` add a CHANGELOG entry                                                                                                           | `commit-msg` hook                    |
| RS1–9  | The Rust core's rules (docs/RUST.md): a port per module, its exports by name, held to the vectors until its TypeScript is switched to a wrapper over it                   | `crates/agent-plugins/scripts/check` |

Also: no `any`, no non-null `!`, no `as` casts outside a narrow, commented reason, no lint-disable
comment without its reason, no legacy shims, no `git stash`/`reset`/`checkout` of others' work.
