<p align="center">
  <img src="docs/assets/logo.svg" width="72" height="72" alt="agent-plugins">
</p>

<h1 align="center">agent-plugins</h1>

<p align="center">
  <strong>Claude Code plugin support for any agent</strong><br>
  A Localix Lab library: skills, commands, agents, hooks, MCP, mods and marketplaces, run through
  ports your host implements
</p>

<p align="center">
  <a href="docs/GETTING-STARTED.md">Get started</a> ·
  <a href="docs/API.md">API</a> ·
  <a href="docs/COMPATIBILITY.md">Compatibility</a> ·
  <a href="docs/SECURITY.md">Security</a> ·
  <a href="examples/ai-sdk-agent/README.md">Example agent</a>
</p>

`@localix-ai/agent-plugins` reads Claude Code plugins and runs what they hold inside your own
agent: on a laptop, in a container or in a cloud sandbox. It never touches a disk, a shell, the
network or a clock itself. Every effect is a port your host implements, so the same plugin behaves
the same wherever your agent runs, and a test can run it with no machine at all.

It is not an agent, a model client or a sandbox. Your host keeps its model, its tool loop, its MCP
client and its interface; the library decides what a plugin means and what each hook, skill and
setting does, as Claude Code does.

## Agent Plugins Extended

agent-plugins follows Agent Plugins Extended (https://github.com/localixai/agent-plugins-extended),
which mines Claude Code; this library syncs from it. The specification grows the vendor-neutral
Agent Plugins format to the whole Claude Code plugin contract and defines it; this library
implements it. It is a working draft (`1.1.0+claude-code.2.1.321`).

- **The contract is the specification at a pinned commit.** `spec/` holds its requirement list
  (2,730 ids for Claude Code 2.1.321), the mods declarations and the scope, synced by `bun run
  spec:sync` and recorded with their hashes in `spec/SOURCE.json`; the tests prove those ids.
- **The library reads both forms of a plugin.** The Claude Code form (`.claude-plugin/plugin.json`
  and its components, or a folder with components in their default places) as before, and, through
  `@localix-ai/agent-plugins/core`, the core Agent Plugins form: a root `plugin.json` with the core
  `$schema`, `skills/`, `mcp.json` and `hooks.json` (core §4–§10). `loadPluginPackage` decides the
  form first; a dual package loads as a Claude Code plugin only. A session runs core packages beside
  Claude Code plugins (`corePackages`, Agent Plugins Extended §18.1–§18.2): their skills, MCP servers
  and hook handlers share one namespace, one permission system and one hook occurrence with the
  plugins'. Chapter 18 is implemented in full: §18.1–§18.2 in sessions, §18.3's dual-package
  manifest check (`checkDualPackage`), and §18.4's conversion both ways (`convertPluginPackage`,
  `coreToClaudeCode`, `claudeCodeToCore`): the files to write beside the source as data, a note
  for everything not carried over, and hooks through a generated Node.js adapter.
- **What the specification does not state is not implemented here on a guess.** It goes to the
  specification first; [docs/SPEC.md](docs/SPEC.md) has the process and how the conventions map.

## Install

```sh
bun add @localix-ai/agent-plugins
bun add -d @anthropic-ai/claude-agent-sdk @modelcontextprotocol/sdk --omit=optional
```

The two SDKs are optional peers that supply the official types; nothing loads them at runtime.
The package runs on Bun and on Node.js 22 or later. It is not on npm under this name yet: until
the first release, link a checkout (`file:` or a workspace). Type-checking, the peers and linking
by path are in [Getting started](docs/GETTING-STARTED.md).

## A first session

```ts
import { createPluginHost, loadPlugin } from '@localix-ai/agent-plugins';

// `files` is your PluginFiles port over the plugin's folder: a directory, an archive, a sandbox.
const { plugin, issues } = await loadPlugin({ files });
if (!plugin) throw new Error(issues.map((issue) => issue.message).join('\n'));

// The ports are yours: how a command runs, where plugins live, what the environment holds.
const host = createPluginHost({
  plugins: [{ id: 'formatter@acme-tools', plugin }],
  ports: { commands, locations, environment },
});
const session = host.openSession({ sessionId, cwd, permissionMode: 'default', transcriptPath });

const { model, menu } = session.skills(); // what the model is told, and the `/` menu
const skill = await session.expandSkill('formatter:review', { arguments: 'src/', signal });
if (skill.ok) console.log(skill.text); // the text the model receives

// Hooks, typed per event: the input is Claude Code's, the outcome merged over every handler.
const outcome = await session.dispatch(
  'PreToolUse',
  { tool_name: 'Bash', tool_use_id: 'call-1', tool_input: { command: 'rm -rf build' } },
  { signal },
);
if (!outcome.proceed) console.log(outcome.block?.reason);
```

`http`, `mcp` and `models` are further ports, needed only by the hook handler types that use
them. [Getting started](docs/GETTING-STARTED.md) wires every port, and runs this same session in a
test on the fakes of `@localix-ai/agent-plugins/testing`.

## What it covers

| Area               | What the library does                                                            |
| ------------------ | -------------------------------------------------------------------------------- |
| Plugins            | reads the manifest and layout, validates in `claude plugin validate`'s words     |
| Skills, commands   | lists them to the model and the `/` menu, expands arguments, `!` commands, files |
| Agents             | subagent definitions, tool lists, model routing, frontmatter hooks               |
| Hooks              | every event and handler type, matched, run through your ports, merged            |
| MCP, LSP, monitors | servers a plugin declares, with expansion, policy and trust gates                |
| Settings, policy   | the settings layers, managed policy, permission rules, folder trust              |
| Marketplaces       | catalogs, installs, updates and dependencies over your fetch ports               |
| Mods               | the mod surface and its hook chain, for a UI process or an isolate               |

## How it is measured

- **Requirements from the documentation.** Claude Code's documentation of one release is turned
  into requirements, one per documented field, event, rule or section, and a test proves each:
  for Claude Code 2.1.321, 2,730 of 2,730, 100.0% overall and in every category
  ([compatibility report](docs/COMPATIBILITY.md), written by `bun run compat`).
- **Judgments, apart.** Where the documentation is silent the library decides, says so in the
  code and in `spec/judgments.md`, and a judgment never counts toward the figure.
- **The official shapes.** Hook inputs and outputs, tools, agents, MCP and settings are the types
  of `@anthropic-ai/claude-agent-sdk`, so a release that changes one fails the build.
- **Real plugins.** `bun run corpus` compares `validatePlugin` with `claude plugin validate` of the
  same release on every plugin of the official and community marketplaces, verdict and findings
  ([testing](docs/TESTING.md), "Corpus").
- **Safe by default.** Where Claude Code's behaviour is a security hole, the library is stricter,
  says so, and gives the option back where one exists ([security](docs/SECURITY.md)).

How requirements are extracted, scoped and proven is in [docs/SPEC.md](docs/SPEC.md).

## Entry points

The root holds what a host loop calls: `createPluginHost` and its sessions, `loadPlugin`,
`validatePlugin`, `decide`, the port types and Claude Code's types. Each domain has its whole
surface at a subpath:

| Import                                    | What it holds                                          |
| ----------------------------------------- | ------------------------------------------------------ |
| `@localix-ai/agent-plugins/loading`       | loading a plugin, workspace trust, MCP, LSP, monitors  |
| `@localix-ai/agent-plugins/format`        | the reader of every plugin file                        |
| `@localix-ai/agent-plugins/skills`        | skills and commands                                    |
| `@localix-ai/agent-plugins/hooks`         | the hook engine                                        |
| `@localix-ai/agent-plugins/rules`         | permission rules and modes                             |
| `@localix-ai/agent-plugins/settings`      | settings layers, merge and every policy key            |
| `@localix-ai/agent-plugins/agents`        | subagents, their catalog and workflows                 |
| `@localix-ai/agent-plugins/marketplace`   | marketplaces, installs and updates                     |
| `@localix-ai/agent-plugins/mods`          | mods and their test kit                                |
| `@localix-ai/agent-plugins/mods/elements` | the mods element table, for a UI process               |
| `@localix-ai/agent-plugins/mods/isolate`  | what code inside an isolate needs                      |
| `@localix-ai/agent-plugins/cli`           | the `claude plugin` commands as plans                  |
| `@localix-ai/agent-plugins/sdk`           | a session's hooks as a Claude Agent SDK `hooks` option |
| `@localix-ai/agent-plugins/elicitation`   | MCP elicitation shapes, without the engine             |
| `@localix-ai/agent-plugins/layout`        | the standard plugin layout, without the engine         |
| `@localix-ai/agent-plugins/testing`       | in-memory fakes of every port                          |

Every export has a row under [`docs/api/`](docs/api/root.md); [docs/API.md](docs/API.md) is the
whole API with its principles.

[`examples/ai-sdk-agent`](examples/ai-sdk-agent/README.md) is a small agent on the AI SDK that runs
real plugins through the library: hooks of every handler type, skills, commands, subagents and MCP
servers.

## Development

| Path        | What it holds                                                         |
| ----------- | --------------------------------------------------------------------- |
| `src/`      | the library, in layers ([architecture](docs/ARCHITECTURE.md))         |
| `tests/`    | one folder per module; a `spec(id)` test proves a requirement         |
| `spec/`     | the requirements extracted from the docs, and the library's judgments |
| `docs/`     | the API, the process, and the generated compatibility report          |
| `examples/` | an agent on the AI SDK that the check runs                            |

```sh
bun install         # also installs the git hooks
bun test            # every test
bun run compat      # the tests and the compatibility report
bun run check       # everything CI runs: format, lint, layers, spec, docs, types, tests
bun run build       # dist, as published
```

A tag `v<version>` publishes that version to npm
([release workflow](.github/workflows/release.yml)). How to work on it, and what a change has to
keep: [CONTRIBUTING.md](CONTRIBUTING.md) and [AGENTS.md](AGENTS.md).

## Licence

MIT ([LICENSE](LICENSE)). Part of Localix Lab, [github.com/localixai](https://github.com/localixai).

An independent project: not affiliated with, endorsed by or supported by Anthropic. Claude and
Claude Code are trademarks of Anthropic, PBC.
