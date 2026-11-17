/**
 * Fixtures of the tests of core packages in a session (Agent Plugins Extended §18.1, §18.2): a core
 * package written for this suite and loaded through the real core loader, a Claude Code plugin
 * beside it, and a host over the fakes whose runner answers each command by its name.
 */
import { createPluginHost, type HostPlugin, type PluginHostOptions } from '../../src/host/index.ts';
import { loadCorePackage, type CorePackage } from '../../src/entry/core.ts';
import type { CommandRun } from '../../src/ports/index.ts';
import type { PolicyView } from '../../src/settings/index.ts';
import type { RunnerReply } from '../../src/testing/ports/runner.ts';
import { loaded, SESSION } from './host.ts';
import { fakeEnvironment, locations, memoryFiles, scriptedRunner } from './ports.ts';

const json = (value: unknown): string => JSON.stringify(value);
const schema = (document: string) =>
  `https://agent-plugins.org/schemas/1.1.0/${document}.schema.json`;

export const CORE_ID = 'acme.tools@acme';
export const CLAUDE_ID = 'guard@acme';

/** `acme.tools`: a skill, a stdio and a remote server, and a handler on each of the five events. */
export function coreFiles(hooks?: Record<string, unknown>): Record<string, string> {
  return {
    'plugin.json': json({ $schema: schema('plugin'), name: 'acme.tools', version: '2.0.0' }),
    'skills/release/SKILL.md': [
      '---',
      'name: release',
      'description: Cut a release.',
      'allowed-tools: Bash(git:*)',
      'disable-model-invocation: true',
      '---',
      'Release $ARGUMENTS from ${CLAUDE_PLUGIN_ROOT} with !`git tag`.',
    ].join('\n'),
    'mcp.json': json({
      $schema: schema('mcp'),
      mcpServers: {
        notes: {
          type: 'stdio',
          command: './bin/notes',
          args: ['--data', '${PLUGIN_DATA}/notes', '${CLAUDE_PLUGIN_ROOT}'],
          env: { NOTES_HOME: '${PLUGIN_ROOT}/share' },
        },
        remote: { type: 'streamable-http', url: 'https://notes.example/mcp' },
      },
    }),
    'bin/notes': '#!/bin/sh',
    'hooks/check': '#!/bin/sh',
    'hooks.json': json({
      $schema: schema('hooks'),
      hooks: hooks ?? {
        SessionStart: [{ command: './hooks/check', args: ['start'] }],
        UserPromptSubmit: [{ command: './hooks/check', args: ['prompt'] }],
        PreToolUse: [{ command: './hooks/check', args: ['pre', '${PLUGIN_ROOT}/policy.json'] }],
        PostToolUse: [{ command: './hooks/check', args: ['post'] }],
        Stop: [{ command: './hooks/check', args: ['stop'] }],
      },
    }),
  };
}

export async function corePackage(files = coreFiles()): Promise<CorePackage> {
  const { package: loadedPackage, issues } = await loadCorePackage({ files: memoryFiles(files) });
  if (loadedPackage === undefined) throw new Error(`the package does not load: ${json(issues)}`);
  return loadedPackage;
}

/** `guard`: a Claude Code plugin with a command hook on the same five events. */
async function claudePlugin(): Promise<HostPlugin> {
  const hook = (name: string) => [{ hooks: [{ type: 'command', command: `cc-${name}` }] }];
  const plugin = await loaded({
    '.claude-plugin/plugin.json': json({ name: 'guard', version: '1.0.0' }),
    'hooks/hooks.json': json({
      hooks: {
        SessionStart: hook('start'),
        UserPromptSubmit: hook('prompt'),
        PreToolUse: hook('pre'),
        PostToolUse: hook('post'),
        PostToolUseFailure: hook('failure'),
        Stop: hook('stop'),
      },
    }),
  });
  return { id: CLAUDE_ID, plugin };
}

/** What a command answers: by the first argument of a core handler, or a Claude Code hook's command. */
export type Answers = Readonly<Record<string, RunnerReply>>;

const keyOf = (run: CommandRun): string =>
  run.spec.form === 'exec' ? `core-${run.spec.args[0] ?? ''}` : run.spec.command;

export interface CoreHostFixture {
  readonly answers?: Answers;
  readonly claude?: boolean;
  readonly core?: CorePackage;
  readonly settings?: PolicyView;
  readonly env?: Record<string, string>;
  readonly options?: Partial<PluginHostOptions>;
}

/** A host with the core package (and, unless `claude: false`, the Claude Code plugin before it). */
export async function coreHostOf(fixture: CoreHostFixture = {}) {
  const answers = fixture.answers ?? {};
  const reply: RunnerReply = (run) => {
    const answer = answers[keyOf(run)] ?? {};
    return typeof answer === 'function' ? answer(run) : answer;
  };
  const runner = scriptedRunner(...Array.from({ length: 64 }, () => reply));
  const host = createPluginHost({
    plugins: fixture.claude === false ? [] : [await claudePlugin()],
    corePackages: [{ id: CORE_ID, package: fixture.core ?? (await corePackage()) }],
    ports: {
      commands: runner,
      locations: {
        ...locations(),
        existingDirectory: (candidates) => Promise.resolve(candidates[0]),
      },
      environment: fakeEnvironment(fixture.env),
    },
    ...(fixture.settings === undefined ? {} : { settings: fixture.settings }),
    ...fixture.options,
  });
  return { host, runner };
}

/** The runs of the core handlers, in order. */
export const coreRuns = (runs: readonly CommandRun[]): CommandRun[] =>
  runs.filter((run) => run.spec.form === 'exec');

/** The JSON a run was given on stdin. */
export const stdinOf = (run: CommandRun | undefined): unknown => JSON.parse(run?.stdin ?? 'null');

export const answer = (value: unknown): RunnerReply => ({ stdout: json(value) });

/** A Bash call, as Claude Code's `PreToolUse` facts carry it. */
export const BASH_CALL = {
  tool_name: 'Bash',
  tool_input: { command: 'npm publish' },
  tool_use_id: 't1',
};

/** A session of `coreHostOf` in `SESSION`, with the runner's answers. */
export async function coreSessionOf(answers: Answers = {}, more: CoreHostFixture = {}) {
  const { host, runner } = await coreHostOf({ answers, ...more });
  return { session: host.openSession(SESSION), runner };
}
