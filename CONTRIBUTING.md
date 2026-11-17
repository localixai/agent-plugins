# Contributing

How to work on `@localix-ai/agent-plugins`, a Localix Lab library.

```sh
bun install          # also installs the git hooks
bun test             # every test, or one file: bun test tests/format/…
bun run check        # everything CI and the pre-push hook run
```

[AGENTS.md](AGENTS.md) is the operating guide, for people and coding agents alike. Read it first.

## What a change has to keep

- **The spec, not memory.** A behaviour comes from Agent Plugins Extended at the pinned commit
  (`spec/SOURCE.json`), a shape from the official types. Read the section of the spec you implement
  and cite it in the test's header. This repository never mines Claude Code: a behaviour the spec
  does not state goes to the spec first. Another host's behaviour, a blog or an issue is a lead for
  the spec, never evidence.
- **A test per requirement.** Every behaviour is proven by a test under `spec(id)` for a requirement
  in `spec/requirements.json`. Write the failing test first ([docs/TESTING.md](docs/TESTING.md)).
- **A judgment only where the spec leaves the choice to an implementation.** Say so in a comment
  and add a row to [spec/judgments.md](spec/judgments.md), linked to the spec's J-id where one
  exists. A judgment never counts as compatibility.
- **Every effect is a port.** `src/` uses no `node:*`, `Bun`, `process`, `Date`, `fetch` or timer
  ([docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)).
- **Never copy a hole.** Where Claude Code's behaviour is unsafe, the kit is safe by default and
  [docs/SECURITY.md](docs/SECURITY.md) records the deviation.

## The specification

agent-plugins follows [Agent Plugins Extended](https://github.com/localixai/agent-plugins-extended),
which mines Claude Code; this library syncs from it. `spec/` is a copy of the spec's files at one
commit: never edit it, run `bun run spec:sync [ref]` (default: the pinned commit), and `check:spec`
fails on anything else. [docs/SPEC.md](docs/SPEC.md) has the process.

## Figures in the documentation

No compatibility figure is typed by hand: each comes from
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md), which `bun run compat` writes.

## Releases

A `feat`, `fix` or `perf` commit adds its entry to [CHANGELOG.md](CHANGELOG.md) under
`[Unreleased]`; the commit hook checks it. A tag `v<version>` publishes that version
([release workflow](.github/workflows/release.yml)).
