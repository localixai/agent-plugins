# Getting started

A host's first steps with `@localix-ai/agent-plugins`: install it, type-check against it, implement
its ports, open a first session, and test the integration without a disk, a shell or a network. The
whole API is in [`docs/API.md`](API.md); every export has a row under [`docs/api/`](api/root.md).

## 1. Install

```sh
bun add @localix-ai/agent-plugins
bun add -d @anthropic-ai/claude-agent-sdk @modelcontextprotocol/sdk --omit=optional
```

The package is not on npm under this name yet. Until its first release, link a checkout instead
(`"@localix-ai/agent-plugins": "file:../agent-plugins"`, or a workspace) and read "Linking a
checkout" below.

The library runs on Bun, which reads its `src`, and on Node.js 22 or later, which reads its `dist`
(`engines`). It has no runtime dependency on either SDK: importing it starts neither Claude Code
nor an MCP client.

### The type peers

The two SDKs supply the official types the library's declarations name: hook inputs and outputs,
tools, agents, settings, and MCP tools and prompts. They are optional peers
(`peerDependenciesMeta`), so a package manager does not install them for you.

- **Install them without optional dependencies.** The Agent SDK ships the Claude Code binary as an
  optional dependency, which the library never runs. `--omit=optional` holds for that one command
  only: a later `bun install` (a clean clone, CI) installs the platform's binary again, and
  omitting optional dependencies for every install drops those of your other packages too.
- **Keep them wherever the declarations are type-checked.** Without them `skipLibCheck: true`
  turns every type the library takes from the SDKs into `any`, with no error.
- **Type-check with `skipLibCheck: true`.** The Agent SDK's own declarations do not compile
  against the zod the library pins (`ZodPreprocess` arity errors in `sdk.d.ts`); that is the
  SDK's to fix.
- **On Node**, supply the runtime's `@types/node` and a library set that includes
  `esnext.disposable`, which the Agent SDK's declarations use.

### Linking a checkout

A host that links the library by path and type-checks its `src` rather than a build adds the
package's own condition, which every subpath maps to its `src/**/*.ts` entry:

```jsonc
// tsconfig.json of the host
{
  "compilerOptions": {
    "moduleResolution": "bundler",
    "customConditions": ["agent-plugins-source"],
    "allowImportingTsExtensions": true,
    // The library's source is checked with your options: it compiles under these.
    "strict": true,
    "exactOptionalPropertyTypes": true,
    "noUncheckedIndexedAccess": true,
    "skipLibCheck": true,
  },
}
```

Bun and Node ignore the condition. Without it, TypeScript reads the built `dist/**/*.d.ts`, so a
linked checkout needs `bun run build` first. A host that installs from the registry reads the
declarations and needs no condition.

## 2. Implement the ports

The library never touches the machine. Every effect is a call on a port, an interface your host
implements over its own disk, processes and network: a local shell, a container, a cloud sandbox.

| Port             | Given to                     | What it does                                                                          |
| ---------------- | ---------------------------- | ------------------------------------------------------------------------------------- |
| `PluginFiles`    | `loadPlugin({ files })`      | reads one plugin folder: `readText`, `readBytes`, `list`, `stat`, `exists`            |
| `CommandRunner`  | `ports.commands` (required)  | runs one process: command hooks, a skill's `` !`cmd` ``, status lines                 |
| `Locations`      | `ports.locations` (required) | the home and project folders, each plugin's root and data folder, path joining        |
| `Environment`    | `ports.environment` (required) | the variables `${VAR}` reads and a hook inherits: `get`, `snapshot`                 |
| `HttpClient`     | `ports.http`                 | `http` hooks                                                                          |
| `McpCaller`      | `ports.mcp`                  | `mcp_tool` hooks, over your MCP client                                                |
| `ModelRunner`    | `ports.models`               | `prompt` and `agent` hooks: `evaluate`, `runAgent`                                    |
| `ScratchFiles`   | `ports.scratch`              | where hook output over the 10,000-character cap is saved; without it, it is cut      |
| `Clock`          | `ports.clock`                | time budgets and timers                                                               |
| `FolderScan`     | `ports.folders`              | the scope folders outside plugins: `~/.claude/skills`, the project's `.claude/` and more |

A port left out only disables what needs it: without `http`, an `http` hook reports a
`PluginIssue` for its component, and the other hooks still run. The conventions every port follows:

- **Absence is an answer.** A file or directory that does not exist is `undefined`, not an error.
- **An effect that did not happen is a value.** A port returns a `PortFailure` (`{ failure,
  message }`) in its result union rather than throwing; a thrown exception is a bug.
- **Cancellation is a signal.** A request that does I/O carries `signal: AbortSignal`; a path-style
  read takes it as a trailing optional argument.
- **The library never builds a process line.** `CommandRunner` is told the form (`exec` with
  arguments, or `shell` with a named shell), the whole environment and the timeout, and enforces
  the timeout itself.
- **Paths are the host's.** `PluginFiles` takes paths relative to the plugin folder, `/`-separated;
  every host path comes from `Locations`, which also joins them.

[`examples/ai-sdk-agent/src/ports.ts`](../examples/ai-sdk-agent/src/ports.ts) implements each of
them on a local machine in under 200 lines: a folder on disk, `bash`, `fetch`, the process
environment and an AI SDK model. The full contract of each port is docs/API.md, "Ports".

## 3. Load a plugin

```ts
import { loadPlugin, validatePlugin } from '@localix-ai/agent-plugins';

const { plugin, issues } = await loadPlugin({ files });
const report = await validatePlugin({ files }); // what `claude plugin validate` reports, in its words
```

`loadPlugin` reads the folder once into a typed `Plugin`; `plugin` is `undefined` when the plugin
does not load (a manifest that fails, no name to load it under), and `issues` says why, in the
shape `claude plugin validate` reports. Keep the `Plugin` (in memory, in a database) and open
sessions over it without reading files again. A plugin installed from a marketplace loads with its
entry (`LoadPluginInput.entry`, from `readMarketplace`); one without a manifest needs a
`fallbackName`.

A folder that may not be a plugin: `classifyPluginFolder` says `plugin` or `folder-of-plugins`; for
a folder of plugins `readPluginDirectory` lists the subfolders that load, so an empty list is a
folder to refuse.

## 4. Open a first session

```ts
import { createPluginHost } from '@localix-ai/agent-plugins';

const host = createPluginHost({
  plugins: [{ id: 'formatter@acme-tools', plugin }],
  ports: { commands, locations, environment, http },
});

const session = host.openSession({
  sessionId: 's-1',
  cwd: '/work/app',
  permissionMode: 'default',
  transcriptPath: '/home/user/.claude/projects/app/s-1.jsonl',
});
const signal = AbortSignal.timeout(60_000);

// Skills and commands: what the model is told, and what the `/` menu offers.
const { model, menu } = session.skills();
const skill = await session.expandSkill('formatter:review', { arguments: 'src/', signal });
if (skill.ok) console.log(skill.text);

// Hooks: the input is Claude Code's, typed per event; the outcome is merged over every handler.
const outcome = await session.dispatch(
  'PreToolUse',
  { tool_name: 'Bash', tool_use_id: 'call-1', tool_input: { command: 'rm -rf build' } },
  { signal },
);
if (!outcome.proceed) console.log(outcome.block?.reason);
```

The plugin's id is `name@marketplace`, the key of `enabledPlugins`; `Locations.pluginRoot(id)` must
return the folder `loadPlugin` read for that plugin: it is `${CLAUDE_PLUGIN_ROOT}` and where its
command hooks run.

`session.checkTool` runs a whole tool call (the `PreToolUse` hooks and the permission rules, then
`PostToolUse`) for a host's tool loop, and `./sdk` gives a session's hooks as the `hooks` option of
a Claude Agent SDK query. Settings, managed policy and saved plugin options are `settings`,
`parentSettings` and `userConfig` of `createPluginHost` (docs/API.md, "Host and sessions").

### Ports that change per session or per turn

The ports of `createPluginHost` are fixed for the host's life; `host.update` replaces plugins and
settings, not ports. Bind what changes per run in `SessionOptions.ports` or a turn's
`withTurn({ ports })` (the models, the MCP caller, the observer, the scratch files, the runner a
skill's `!` commands and the hooks use), or give the host a port that forwards to one bound later:

```ts
let models: ModelRunner | undefined; // set when the provider is ready
const bound = (): ModelRunner => {
  if (models === undefined) throw new Error('no model runner yet');
  return models;
};
const lateModels: ModelRunner = {
  evaluate: (request) => bound().evaluate(request),
  runAgent: (request) => bound().runAgent(request),
};
```

## 5. Test the integration with the fakes

`@localix-ai/agent-plugins/testing` has an in-memory fake of every port. The scripted ones answer
in order, record every call and throw when the script is spent, so a test never passes by
accident. The session above, on a plugin held in memory, runs in a test like this:

```ts
import { expect, test } from 'bun:test';
import { createPluginHost, loadPlugin } from '@localix-ai/agent-plugins';
import {
  fakeEnvironment,
  locations,
  memoryFiles,
  scriptedRunner,
} from '@localix-ai/agent-plugins/testing';

const files = memoryFiles({
  '.claude-plugin/plugin.json': JSON.stringify({ name: 'formatter' }),
  'skills/review/SKILL.md': '---\ndescription: Review the files given\n---\nReview $ARGUMENTS.\n',
  'hooks/hooks.json': JSON.stringify({
    hooks: {
      PreToolUse: [
        { matcher: 'Bash', hooks: [{ type: 'command', command: '${CLAUDE_PLUGIN_ROOT}/guard.sh' }] },
      ],
    },
  }),
});

test('the guard hook blocks the call', async () => {
  const { plugin } = await loadPlugin({ files });
  if (!plugin) throw new Error('the plugin does not load');
  const commands = scriptedRunner({ exitCode: 2, stderr: 'rm is not allowed here' });
  const host = createPluginHost({
    plugins: [{ id: 'formatter@acme-tools', plugin }],
    ports: {
      commands,
      // Every plugin root (and the project) must be listed, or a command hook fails with
      // "Plugin directory does not exist".
      locations: locations({ directories: ['/plugins/formatter@acme-tools', '/work/app'] }),
      environment: fakeEnvironment({ HOME: '/home/user' }),
    },
  });
  const session = host.openSession({
    sessionId: 's-1',
    cwd: '/work/app',
    permissionMode: 'default',
    transcriptPath: '/home/user/.claude/s-1.jsonl',
  });
  const signal = new AbortController().signal;

  expect(session.skills().menu.map((entry) => entry.name)).toEqual(['formatter:review']);

  const outcome = await session.dispatch(
    'PreToolUse',
    { tool_name: 'Bash', tool_use_id: 't1', tool_input: { command: 'rm -rf build' } },
    { signal },
  );
  expect(outcome.proceed).toBe(false);
  expect(outcome.block?.reason).toBe('rm is not allowed here');
  expect(commands.calls[0]?.env.CLAUDE_PLUGIN_ROOT).toBe('/plugins/formatter@acme-tools');
});
```

The other fakes are `recordingHttp`, `fakeModel`, `fakeMcp`, `fixedClock`, `memoryScratch` and
`folderScan`; docs/API.md, "Testing kit", says what each answers. The mods test kit is `modsTesting`
in `./mods`.

## 6. Where to go next

- The root holds what a host loop calls; each domain has its whole surface at a subpath:
  `./marketplace` (`readMarketplace`, `planMarketplaceFetch`, `planPluginFetch`: marketplaces,
  installs and updates over your fetch ports), `./mods` (`createModsHost`: mods and their hook
  chain, with `./mods/elements` and `./mods/isolate` for a UI process or an isolate), `./cli`
  (`planInstall` and the rest of `claude plugin`), `./sdk` (a session's hooks as the Agent SDK's
  `hooks` option), and `./loading`, `./format`, `./skills`, `./hooks`, `./rules`, `./settings`,
  `./agents`, `./layout`, `./elicitation` for the parts.
- [`examples/ai-sdk-agent`](../examples/ai-sdk-agent/README.md) is a small agent on the AI SDK that
  runs real plugins through the library: hooks of every handler type, skills, commands, subagents
  and MCP servers. `bun run smoke` there runs one turn on a scripted model, with no key and no
  network, and the repository's `bun run check` runs it; `bun run scenarios` checks real plugins on
  Claude Haiku 5.5.
- [`docs/SECURITY.md`](SECURITY.md) lists where the library is stricter than Claude Code, the
  option that restores Claude Code's behaviour where one exists, and the risks your host owns.
