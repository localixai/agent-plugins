# Spec and evidence

agent-plugins follows Agent Plugins Extended (https://github.com/localixai/agent-plugins-extended),
which mines Claude Code; this library syncs from it. The kit claims compatibility with Claude Code
only where a passing test proves a requirement the spec lists. This doc is the process;
`docs/COMPATIBILITY.md` is its output.

## Sources of truth

| Source | What it decides | Where it lives here |
| --- | --- | --- |
| Agent Plugins Extended, at the commit `spec/SOURCE.json` pins | the contract: requirements, judgments (`J<n>.<m>`), deviations from the docs, scope | `spec/` (synced, never edited by hand) |
| `@anthropic-ai/claude-agent-sdk` type declarations, of the release the spec targets | shapes: hook inputs and outputs, tools, agents, MCP, settings | `node_modules/` (pinned) |

Nothing else is evidence: not memory, not another host's behaviour (our own hosts' included), not a
plausible guess, and not a run of Claude Code made here. The spec owns the evidence it is written
against: the Claude Code docs, the mods declarations Claude Code writes, its validator and loader
observed as a black box, and the corpus of real plugins. This repository never pulls or reads any
of them to decide a behaviour (AGENTS.md, E1a).

A behaviour the spec does not state is not implemented on a guess. It goes to the spec first: a
requirement, a judgment or a deviation in agent-plugins-extended. Once the spec states it,
`bun run spec:sync <ref>` brings the revision here and the behaviour is implemented and tested.

## What `spec/` holds

| File | What it is | Owner |
| --- | --- | --- |
| `spec/requirements.json` | every requirement id of the contract, with its category and the docs page and section it cites | the spec |
| `spec/mods-declarations.json` | what the mods declarations of the release declare (E2a) | the spec |
| `spec/docs-tables.json` | the tables of the mods reference page as data, for the mods tests | the spec |
| `spec/out-of-scope.json`, `spec/out-of-scope-pages.json` | the sections and pages the contract leaves out, with reasons | the spec |
| `spec/scope-audit.md` | the owner-approved plugin-host boundary | the spec |
| `spec/SOURCE.json` | the repository, the commit, the Claude Code release and the SHA-256 of each file above | `bun run spec:sync` |
| `spec/judgments.md` | the decisions the spec leaves to an implementation (E4) | this repository |

A requirement id reads `<category path>.<key>`: `manifest.field.homepage`,
`hooks.handler.common.timeout`, `hooks.input.PreToolUse.tool_input`,
`permissions.rule.Bash(git push *)`. The id is the same in the spec and here.

## Syncing the spec

`bun run spec:sync [ref] [--remote]` (`scripts/spec/sync.ts`) copies the files above from
`extend/conformance/` of the spec at a git ref into `spec/` and rewrites `spec/SOURCE.json`.

- The ref defaults to the commit `SOURCE.json` pins, so a plain run restores the pinned files. A
  branch (`main`) or a tag moves the pin to the commit it names.
- The files are read from the checkout beside this repository (`../agent-plugins-extended`) when it
  has the ref (its local refs: fetch there first for a new `main`), else from GitHub: `git` over
  HTTPS, then `gh`. `--remote` skips the checkout.
- The requirements and the declarations must name one Claude Code release, which `SOURCE.json`
  records as `release`; a ref whose snapshots disagree is refused and nothing is written.

`check:spec` (`scripts/spec/check.ts`) runs offline on every `bun run check`:

- **SP1** `spec/requirements.json` parses, and its ids are unique and sorted.
- **SP2** every id a test names in `spec(...)` is a requirement.
- **SP3** the installed Agent SDK is the release `SOURCE.json` names (`0.3.N` ↔ `2.1.N`).
- **SP4** `SOURCE.json` lists exactly the synced files, each is byte for byte what it recorded, and
  the snapshots name its release: a hand edit or a stale copy fails, and `bun run spec:sync` is the
  only fix.
- **SP5** the snapshots the tests read parse (`docs-tables.json`, `mods-declarations.json`).

## Proof

A test proves requirements by running under `spec(ids, body)` from `tests/support/spec.ts`, a
`describe` named `⟦id⟧`. `spec` refuses an id that is not in `spec/requirements.json`, and so does
`check:spec` (SP2) statically. A test states the behaviour the spec states for the id, with the
section it proves named in the file's header comment (the spec's chapter, or the docs page and
section the requirement cites). The static gate checks single-quoted, double-quoted and static
template-literal ids, including arrays; the other quote character inside an id is part of the id,
not a delimiter.

`bun run compat` runs every test with a JUnit report and writes `docs/COMPATIBILITY.md`:

- a requirement is **proven** when at least one test under its block ran and all of them passed;
- **failing** when one of them failed; **unproven** when none ran;
- the share proven, overall and per category, over every requirement. Nothing is excluded from
  the denominator, so the number never flatters.

`bun run check` ends with `compat --check`: a failing test or a stale report fails the check.

## Judgments and deviations

The spec decides where the Claude Code docs are silent (its judgments, `J<chapter>.<n>`) and where
Claude Code's own behaviour departs from its docs (its deviations, E4a there), each with the oracle
runs it rests on. The kit follows both; its tests of a judgment or a deviation run outside
`spec(...)`, since compatibility measures the requirements alone.

`spec/judgments.md` here holds only what the spec leaves to an implementation: how the kit shapes
its API, merges several plugins' answers into one, orders work, or words an issue the spec does not
word. Each row names the code that holds it, and the spec's J-id when the spec has a judgment on
the same point. A row whose question the spec should settle is taken to the spec (E1a) and, once
the spec settles it, replaced by a link to its J-id.

Where the spec follows Claude Code's behaviour over its docs on a loading outcome, the kit does too:

| Docs | What the spec states | The kit |
| --- | --- | --- |
| plugins/troubleshooting.md, "`Failed to load hooks from <path>`": a hooks file that fails the schema is a load failure of the hooks | A guard hook (`PreToolUse`, `PermissionRequest`) that cannot be read, or a stray guard or handler list outside `hooks`, keeps the whole plugin from loading | `loadPlugin` returns no plugin |
| mcp.md, "Option 2": Claude Code skips a server whose entry cannot be read | An unreadable entry in the manifest's inline `mcpServers` keeps the whole plugin from loading (the same entry in `.mcp.json` is dropped) | `loadPlugin` returns no plugin |
| plugins/components.md, "Skills": a frontmatter `name` "replaces the last segment" of `/<plugin>:<directory>` | A session names a skill that a scan finds after its folder; `name` is used only for a folder a manifest path names itself and for the plugin-root `SKILL.md` | The catalog gives the folder as the final name (`SkillLayout.fixedName`); `resolveSkillName` alone keeps the docs |
| skills.md, "Resolve skills that share a name": a skill wins over a `.claude/commands/` file | In a plugin, the command's entry is the one in `slash_commands` for a skill and a command of one name | The session lists the command |

## Official types

`src/protocol/official.ts` re-exports Claude Code's types from the SDK with `import type` (L2:
nowhere else, never at runtime). Runtime lists the kit needs (`HOOK_EVENTS`,
`HOOK_HANDLER_TYPES`) are `as const` arrays asserted equal to the official unions with
`Assert<Equal<…>>`; registries are mapped types over those unions. When the SDK adds, removes or
renames an event, a handler type or a field, `bun run typecheck` names every place to change.

## Declarations

`spec/mods-declarations.json` is the spec's reading of the TypeScript declarations Claude Code
writes beside a mod (`claude-code/index.d.ts`, the built-in tools' inputs and the MCP types): the
events and their fields, `$` and its members, the fields of `next`, the elements of each surface
and their props, what `claude-code` and `claude-code/testing` export and the built-in tools'
inputs, each with its line. Tests read it through `scripts/spec/declarations.ts`.
`tests/mods/declarations/surface.test.ts` compares the kit with it and fails on a difference not
judged in `spec/judgments.md` (`**Dn** declarations <release>`, listed in
`tests/support/declared-differences.ts`) and on a judged one that is gone (AGENTS.md, E2a).

## How the conventions map

| Here | In the spec |
| --- | --- |
| `spec/requirements.json`, synced by `bun run spec:sync` | its conformance requirement list, a snapshot never edited by hand (C2) |
| a requirement id, `<category path>.<key>` | the same id; a normative paragraph names it in a `req` marker (C1) |
| proven: a passing test under `spec(id)` (E3) | stated: a normative paragraph carries the id's marker |
| `docs/COMPATIBILITY.md`, the share proven (E5) | its conformance report, the share stated (E5) |
| a row of `spec/judgments.md`: a decision left to an implementation (E4) | a judgment `J<chapter>.<n>` in the table at the end of its chapter (E4) |
| a behaviour the kit follows over the docs (above) | a deviation (E4a): the text follows the observed behaviour, not counted as stated |
| `declarations <release>` (E2a) | official shapes of the same release (E2) |

The rule ids on the left are this repository's (AGENTS.md); those on the right are the spec's own
guide, which numbers its rules independently.

The library reads the Claude Code form of a plugin and, in `./core`, loads the core form of Agent
Plugins (a root `plugin.json` with a core `$schema`, core §4–§10); its tests of the core cite
`core §n` and run outside `spec(...)`, since the requirement ids are Claude Code's. A session does
not run core hooks or servers yet.

## Moving to a new spec revision

The spec moves first: a new Claude Code release is mined there, and its snapshots are committed
there. Here:

1. `bun run spec:sync <ref>` (a commit, a tag or `main`). Read the requirements it added and
   removed (`git diff spec/requirements.json`).
2. When `SOURCE.json`'s release changed, `bun add -d @anthropic-ai/claude-agent-sdk@<matching>` and
   keep the `peerDependencies` range in step; `check:spec` (SP3) refuses a pair that disagrees.
3. `bun run typecheck`: fix every place a changed official type breaks.
4. Write tests for the added requirements and the changed judgments, fix what fails, `bun run
   compat`.
5. Commit `spec/`, the code and the report together.
