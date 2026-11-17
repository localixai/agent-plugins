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

The inputs are generated in memory (`scripts/bench/inputs.ts`): no corpus, no disk, no clock. Each
scenario (`scripts/bench/scenarios/`) measures one entry point at three sizes, N, 2N and 4N, and
names the law each measurement keeps (`scripts/bench/laws.ts`): `linear` (the step from 2N to 4N
is exactly twice the step from N to 2N, for every counter), `constant`, `no-io` (no port call) or
`nothing` (no counter at all). A measurement listed in `repeats` (the same call again, warm) builds
no pattern, and no measurement reads or lists a path twice. The kit keeps module-level memos
(`src/rules/memo.ts`), so the counts run in a fresh process, each scenario is warmed once at its
smallest size, and every measurement gets text of its own (a salt in each name), so a memo is never
warm from another measurement.

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

`bun run bench:validate` compares `claude plugin validate <dir>` (the program `CLAUDE_ORACLE_BIN`
names, else `claude`) with `validatePlugin` on the first 30 corpus plugins by `marketplace/name`,
the median of five runs each, and an empty folder as the floor of both programs. Starting the
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

## Differential checks on fixtures

The checks below run Claude Code as a black box on small fixtures of our own and compare the kit
with what it does; the result is committed beside the fixture, and `bun test` compares the kit with
it without running Claude Code. They measure this implementation, never the contract (AGENTS.md,
E5a): the behaviour they check is the spec's, and a difference the spec does not settle goes to the
spec first (E1a). `bun run corpus:runtime` is the same check of a session over the corpus plugins
that run nothing at start.

## Remote MCP credential oracle

`bun run oracle:remote-mcp-env` observes which `${VAR}` names Claude Code reads as empty in a remote
MCP server's `url` and `headers` (mcp.md lists them "such as"). For the names of
`tests/fixtures/remote-mcp-env/candidates.txt`, a project `.mcp.json` sends one header per name to a
local HTTP server through `claude mcp get`; a covered name arrives empty whether or not it is set,
so the run sets no variable and gets a bare environment. The result is committed as `expected.json`
beside the candidates; `tests/loading/remote-mcp-credentials.test.ts` checks the kit's rule against it
without running Claude Code, and `--check` fails when a new release empties other names.

## headersHelper environment oracle

`bun run oracle:headers-helper-env` observes which variables Claude Code removes from a supplied
`headersHelper` (mcp.md, "Which variables a helper can read", gives the rule by example). Every name
of `tests/fixtures/headers-helper-env/candidates.txt` is set to a dummy value in a bare environment
(a proxy to an address `NO_PROXY` bypasses), a trusted private project's helper prints the names it
received to a local HTTP server through `claude mcp get`, and the result is committed as
`expected.json`; `tests/loading/helper-credentials.test.ts` checks the kit's rule against it.

`bun run oracle:subprocess-scrub` observes what `CLAUDE_CODE_SUBPROCESS_ENV_SCRUB=1` leaves of each
variable of `tests/fixtures/subprocess-scrub/candidates.json` (env-vars.md, "What the subprocess
environment scrub removes", gives the rule by example): a bare environment, a private config folder
whose SessionStart hook prints its environment, an API address nobody listens on, and the run stopped
once the hook wrote. The result is committed as `expected.json`;
`tests/hooks/handlers/command/scrub-judgments.test.ts` checks the kit's one scrub against it.

## Model-request oracle

`bun run oracle:model-content` reads what Claude Code sends the model when a plugin's skill or
command is invoked. `scripts/oracle/model-request.ts` runs `claude -p "/<plugin>:<skill> <args>"`
with `ANTHROPIC_BASE_URL` pointing at a local fake Messages API that records each request and
streams back one word, so no model is called; the config and working folders are private and the
environment is bare. Each case of `tests/fixtures/model-content/<case>/` is a tiny plugin of its own
and a `case.json`; the user message blocks Claude Code sent for the skill (its own reminders left out), with the
run's paths and session id normalized, are committed as `expected.json`. `tests/skills/model-content-oracle.test.ts` compares the
kit's `expandSkill` with the block after `<command-message>` without running Claude Code. The run
executes the plugin: point it only at fixtures written for it.

## Runtime oracle

`bun run oracle:runtime` compares what the kit's SESSION registers with what a Claude Code session
registers, plugin by plugin, for the fixtures of `tests/fixtures/runtime/<case>/` (each folder is a
tiny plugin of its own, named after the case). Claude Code's side is the `init` event of
`claude -p --plugin-dir <fixture> --output-format stream-json --verbose --max-turns 1 hi`, run as a
black box (`scripts/oracle/runtime.ts`) in a private home, config folder and working folder, with no
inherited variable but `PATH`, an invalid API key and an API address nothing listens on; the run is
ended at `init`, before the model is asked, so nothing is spent. It runs the fixture's hooks and MCP
servers (a stdio server that does not exist is enough: its name is registered, its status `failed`),
so point it only at fixtures written for it, never at a third-party or corpus plugin. The event is
cached per Claude Code version and content hash of the fixture in `.cache/oracle-runtime/<version>/`;
`oracle-settings.json` in a fixture is copied to the config folder as `settings.json`.

Per case it compares, filtered to the plugin's own names: `loaded` (the plugin is among the session's
plugins), `agents`, `slashCommands` (`slash_commands`), `skills` (`init.skills`; its rule is in
`spec/judgments.md`), `mcpServers` (`mcp_servers`, named `plugin:<plugin>:<server>`) and `outputStyle`
(the setting, echoed). The kit's side is `scripts/oracle/kit-registry.ts`: `loadPlugin` over the
folder, `createPluginHost`, then `session.agents()`, `skills()` (the menu), `mcpServers()` and
`outputStyles()`.

`expected.json` beside each fixture is the oracle's result, ids only, and is committed:
`bun test` (`tests/host/runtime-oracle.test.ts`) checks the kit against it without running `claude`.
`oracle:runtime` rewrites a stale `expected.json` when `claude` is installed (`--check` fails
instead) and exits non-zero on a difference not recorded in `tests/fixtures/runtime/judgments.json`
(the value the kit gives and the reason). `.cache` stays uncommitted.

Where the oracle cannot see: the winner of two agents with one name (names only), the model's skill
listing, output styles (`init.output_style` echoes the setting; a forced plugin style is not
reflected), and statuses of servers that need a network.

## Settings oracle

`bun run oracle:settings` compares the kit's settings merge (`loadSettingsLayers`, `createPolicyView`,
`startingMode`) with the Agent SDK's own resolver, `resolveSettings` and `filterEscalatingDefaultMode`
(pure JS, `@alpha`), for the cases of `scripts/oracle/settings-cases.ts`; for the mode a session
starts in it also asks `claude` itself, isolated as the runtime oracle is and ended at `init`. HOME
and the config folder are private; the machine's managed settings file cannot be moved, so the kit is
handed the same file (an MDM profile is not seen). Lists compare as sets, since the order carries no
behaviour. A case where the kit is known to differ carries the reason (`known`); the run exits
non-zero on an unknown difference, on a known one that no longer differs, and on any difference from
`claude`. Nothing is cached or committed but the cases.
