# Testing

## Test first

Every behaviour starts as a failing test that cites the section of the docs it comes from, under
`spec(id)` for the requirement it proves (docs/SPEC.md). Then the code, then `bun run compat`.

## Where tests live

`tests/<module>/` mirrors `src/<module>/`; helpers live in `tests/support/` (T2). A test file is
`*.test.ts`. No test is focused or switched off (T1); a test that needs something a machine may
lack uses `skipIf(!available)`.

## Deterministic

Tests touch no network, no real clock, no randomness and no files outside a temporary folder they
make. The kit's ports make this natural: a test passes fakes. What a test needs from the contract
is in `spec/`, synced from Agent Plugins Extended (`spec/docs-tables.json`,
`spec/mods-declarations.json`; docs/SPEC.md, "Syncing the spec"), or written by hand from the
section it cites (`tests/support/tool-table.ts`). The corpus is read only by `scripts/`, never by
`bun test`.

## Fakes of the ports

`src/testing/ports` holds an in-memory fake of every port (docs/API.md, "Testing kit"), published as
`@localix-ai/agent-plugins/testing`; `tests/support/ports.ts` re-exports it, one copy:
`memoryFiles(record)`, `scriptedRunner(...replies)`, `recordingHttp(...replies)`,
`fakeModel(...replies)`, `fakeMcp(options)`, `fixedClock(start)`, `locations(options)`,
`fakeEnvironment(record)` and `memoryScratch()`. The scripted ones answer in order, record every
call and throw when the script is spent, so a test never passes by accident. A stream tests
through these and extends them in a file of its own, never by editing another stream's.

## What a good spec test looks like

- One `spec(...)` block per requirement (or a small set the same behaviour proves together).
- A positive case and the documented failure: a field of the wrong type fails, a path with `..`
  is refused with the validator's message.
- Messages the spec quotes verbatim are asserted verbatim.
- No assertion on behaviour the spec does not state; a judgment is tested outside `spec(...)`.

## Corpus

The spec tests prove the kit against the requirements; the corpus measures it against real plugins
and Claude Code's own verdicts on them. Agent Plugins Extended owns both: `bun run corpus:fetch`
and `bun run corpus:oracle` in its `extend/tools/` fetch every plugin of the official and community
marketplaces and record what `claude plugin validate --json` and `claude --plugin-dir <dir> plugin
details <name>` of the spec's release say about each. This repository never fetches the corpus or
runs Claude Code for those verdicts; it reads them from `CORPUS_DIR`, by default the spec checkout's
`extend/.cache/corpus` beside this repository (`scripts/corpus/location.ts`), for the release
`spec/SOURCE.json` names.

`bun run corpus` runs the kit over every plugin and compares it with the recorded validation
verdict (`scripts/corpus/oracle.ts`); a plugin without one is counted apart. The kit's side is
`validatePlugin` over the plugin's folder (docs/API.md, "Validating"); `loadPlugin`'s own verdict
is printed on a separate line, because loading and validating differ on purpose (the validator
reports advice and checks the loader never makes). The run prints:

- the verdict agreement: an error exactly when Claude Code's validator reports one;
- the share of its findings the kit reports with the same severity and message, and the share also
  at the same file and path (findings are paired as a multiset, a finding at the right place first);
- the most frequent findings the kit misses, reports beyond the validator, or puts elsewhere.

`.cache/corpus/report.json` here holds every plugin's. Nothing of the corpus is committed, and
`bun test` never reads it. A difference the run shows is a lead for the spec, never evidence here
(AGENTS.md, E5a): what the spec then states is tested with inputs of our own.

Where the kit and the oracle still differ, and why: a plugin whose directory does not exist is
reported with the oracle's absolute path; `directory`-profile findings of the validator (checks
that need a network or the marketplace directory service) are out of scope; the oracle reads YAML
with Bun's parser, which has quirks the kit approximates (`tests/format/frontmatter-retry.test.ts`
lists what was observed); and a `.mcp.json` entry with a `url` and no `type` is an error to the
loader, as mcp.md says, while the validator reports nothing for that file.

## Commands

| Task                           | Command                                |
| ------------------------------ | -------------------------------------- |
| Every test                     | `bun test`                             |
| One file                       | `bun test tests/format/manifest-fields.test.ts` |
| Tests and the report           | `bun run compat`                       |
| Everything CI and pre-push run | `bun run check`                        |
| The example agent on a scripted model (no key, no network; part of `check`) | `bun run check:example` |
| Real plugins against the spec's recorded verdicts | `bun run corpus`, `bun run corpus:load` |

## Benchmarks

Two kinds: deterministic counts over generated inputs, which are a gate, and times over the corpus,
which are not.

### Counts (the gate)

The meter (`scripts/bench/meter/`) counts units of work instead of time, so one input gives one
number on every run and machine. Nothing in `src/` counts: the counters wrap what the kit is given
or calls.

- Port calls, by `<port>.<method>`: `files.*` (`PluginFiles`), `commands.*`, `http.*`,
  `environment.*`, `locations.*`, `models.*`, `folders.*` (`FolderScan`). `files.bytes` adds the
  bytes read; `files.reread` and `files.relist` count a path read or listed again within one
  measurement.
- CPU proxies, by wrapping globals and libraries while the meter runs: `cpu.JSON.parse` (documents),
  `cpu.RegExp` (patterns built at run time, never a literal), `cpu.RegExp.exec` (matches run; a
  `split` by a pattern tries it at each position), `cpu.yaml.parse` and `cpu.acorn.parse`
  (documents). What a fake port does to answer is not counted.
- Not counted: the work of a module switched to the Rust core (docs/RUST.md), which runs inside
  the WebAssembly module, and the boundary's own JSON (`src/wasm/wire.ts` takes `JSON`'s functions
  before the meter wraps them). A count that falls when a module is switched is that work moving
  out of sight; the core has no counts of its own yet.

The inputs are generated in memory (`scripts/bench/inputs.ts`): no corpus, no disk, no clock. Each
scenario (`scripts/bench/scenarios/`) measures one entry point at three sizes, N, 2N and 4N, and
names the law each measurement keeps (`scripts/bench/laws.ts`): `linear` (the step from 2N to 4N
is exactly twice the step from N to 2N, for every counter), `constant`, `no-io` (no port call) or
`nothing` (no counter at all). A measurement listed in `repeats` (the same call again, warm) builds
no pattern, and no measurement reads or lists a path twice. The core keeps module-level memos
(`crates/agent-plugins/src/rules/memo.rs`), so the counts run in a fresh process, each scenario is
warmed once at its smallest size, and every measurement gets text of its own (a salt in each name),
so a memo is never warm from another measurement.

`tests/bench/counts.test.ts` runs `scripts/bench/counts.ts --json` twice and fails when the two runs
differ, when a law is broken, or when any count differs from the committed baseline,
`tests/fixtures/bench/counts.json`, whether it rose or fell. After a change that changes the kit's
work on purpose, run `bun run bench:update` and commit the new baseline with the change: the diff
of that file is the review of the work the change adds or saves. A count that rose is a cost to
explain; one that fell is a saving to write in the changelog.

### Times over the corpus

`bun run bench` times what a host repeats, over the real plugins of the corpus (the spec's,
"Corpus" above), with effects stubbed so the numbers are the kit's own cost:

- `loadPlugin` per plugin, from disk (the corpus disk adapter) and from memory (kit CPU only);
- `createPluginHost`, `openSession`, `session.skills()` and `dispatch('PreToolUse')` for 20 plugins
  and for every loaded plugin at once (a stress case no host meets).

It then counts the same work with the meter (`scripts/bench/corpus-counts.ts`), prints both tables
and writes `.cache/bench.json`. Timings depend on the machine, so they are not a gate; the counts
are the same on every run over one corpus checkout.

`bun run bench:validate` compares `claude plugin validate <dir>` with `validatePlugin` on the first
30 corpus plugins by `marketplace/name`, the median of five runs each, and an empty folder as the
floor of both programs. The `claude` column is the spec's timing on this machine (`oracle:record
validate-timings`, `<oracle cache>/<release>/validate-timings.json`, "Over the corpus" below); this
script never runs it. Starting the
`claude` process is almost all of its time, so the comparison says what one CLI call costs a
host, not how fast either reads a plugin.

## Mutation testing

`bun run mutation <module>` mutates `src/<module>` with StrykerJS and runs only `tests/<module>`
against each mutant (command runner, `bun test --bail`). A surviving mutant is a change of behaviour
the module's own tests do not notice: a weak or missing assertion. The report goes to
`.cache/mutation/<module>.json`; the score is printed. It is heavy (minutes to an hour per module),
so it runs by hand, never in hooks or CI. Run it on a module after changing its tests or code, and
kill survivors by asserting the documented behaviour they change, never by testing the mutation.

## Loading comparison

`bun run corpus:load` compares what the kit LOADS with what Claude Code loads, plugin by plugin:
whether the plugin loads, and the names of its skills (commands included, as multisets), agents,
hook events, MCP servers and LSP servers. Claude Code's side is the inventory the spec recorded
from `claude --plugin-dir <dir> plugin details <name>` (`scripts/corpus/load-oracle.ts` reads
`<corpus>/oracle-load/<release>/`); a plugin without one is counted apart. Every difference is
written to `.cache/corpus/load-report.json`. `validatePlugin` agreeing with `claude plugin
validate` says nothing about loading; this does.

What the comparison reads: the kit's side is `loadPlugin` itself (`scripts/corpus/load-kit.ts`), not
a session, because `plugin details` lists what a plugin declares: a skill by the name it has inside
its plugin, the servers a file names before their entries are read, the hook events whose value is a
list. A `--plugin-dir` folder is classified first (`classifyPluginFolder`); a folder of plugins
answers for its child with the plugin's name.

What stays different, and why (spec/judgments.md, the last row): `plugin details` lists only the
top-level `.md` files of a `commands/` or `agents/` folder, never nested ones, never a manifest
path to a file or a `commands` object map (its debug log reports `ENOTDIR`); the docs say the
opposite (plugins/components.md, "Commands" and "Agents"), and the kit keeps the docs until a
session shows otherwise. `bun run corpus:load` prints the agreement per field; that residual is
all of the remaining `skills` and `agents` differences.

## Oracle runs

This repository never runs Claude Code. Agent Plugins Extended runs it as a black box on fixtures
written for the purpose and records what it did (`bun run oracle:record` in its `extend/tools/`; its
`extend/conformance/README.md`, "Oracle runs", says how each run is isolated). The fixtures and the
outcomes are small, so the spec commits them in `extend/conformance/oracle/`, and `bun run
spec:sync` copies them into `spec/oracle/` with the other snapshots (docs/SPEC.md). The kit's side
stays here and reads them through `scripts/oracle/recorded.ts`; `bun test` compares, with no
`claude` installed. These checks measure this implementation, never the contract (AGENTS.md, E5a):
the behaviour they check is the spec's, and a difference the spec does not settle goes to the spec
first (E1a). A new release's outcomes reach this repository only as a new spec revision.

| Outcome (`spec/oracle/`) | What Claude Code was asked | The kit's side |
| --- | --- | --- |
| `runtime/<case>/expected.json` | what a session registers from the case's plugin (`init` of `claude -p --plugin-dir`), ids only | `tests/host/runtime-oracle.test.ts`, `bun run oracle:runtime` |
| `settings.json` | the Agent SDK's `resolveSettings` per key, and the mode `claude -p` starts in | `bun run oracle:settings` |
| `subprocess-scrub/expected.json` | what a hook receives under `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` | `tests/hooks/handlers/command/scrub-judgments.test.ts` |
| `remote-mcp-env/expected.json` | which `${VAR}` names a remote MCP `url` and `headers` read as empty | `tests/loading/remote-mcp-credentials.test.ts` |
| `headers-helper-env/expected.json` | which variables a supplied `headersHelper` runs without | `tests/loading/helper-credentials.test.ts` |
| `model-content/<case>/expected.json` | the user text blocks the model gets when a skill or command is invoked | `tests/skills/model-content-oracle.test.ts` (`scripts/oracle/model-content.ts`) |

### Runtime

Per case `oracle:runtime` compares, filtered to the plugin's own names: `loaded` (the plugin is among
the session's plugins), `agents`, `slashCommands` (`slash_commands`), `skills` (`init.skills`; its
rule is in `spec/judgments.md`), `mcpServers` (`mcp_servers`, named `plugin:<plugin>:<server>`) and
`outputStyle` (the setting, echoed). The kit's side is `scripts/oracle/kit-registry.ts`:
`loadPlugin` over the folder, `createPluginHost`, then `session.agents()`, `skills()` (the menu),
`mcpServers()` and `outputStyles()`; a case's `oracle-settings.json` is its user layer. A field the
kit gives differently on purpose is in `tests/fixtures/runtime-judgments.json` (the value the kit
gives and the row of `spec/judgments.md`); any other difference fails.

Where the oracle cannot see: the winner of two agents with one name (names only), the model's skill
listing, output styles (`init.output_style` echoes the setting; a forced plugin style is not
reflected), and statuses of servers that need a network.

### Settings

`oracle:settings` hands the kit's merge (`loadSettingsLayers`, `createPolicyView`, `startingMode`)
each case's files and compares each key with the SDK resolver's recorded value, and the starting
mode also with `claude`'s. Lists compare as sets, since the order carries no behaviour. The
recording machine's managed settings are not recorded: a key they set is not compared, and where
there were any the kit gets a managed file that delivers an unrelated key, so a parent layer meets a
managed policy as it did there. A case where the kit is known to differ carries the reason in
`scripts/oracle/settings-known.ts`; the run exits non-zero on an unknown difference, on a known one
that no longer differs, and on any difference from `claude`.

### Over the corpus

`bun run corpus:runtime` is the runtime check over the corpus plugins: the spec records a session
per plugin that runs nothing at start (`oracle:record corpus-runtime`), too large to commit, in its
`extend/.cache/oracle/<release>/corpus-runtime/` (`ORACLE_CACHE_DIR` moves it; `--release` picks
the release, the pinned one by default). The kit's registry of each plugin is compared field by
field, and the differences go to `.cache/corpus/runtime-report.json`.
