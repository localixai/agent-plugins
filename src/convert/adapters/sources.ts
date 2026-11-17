import { CLAUDE_CODE_RUNNER_SOURCE } from './claude-code-runner-source.ts';
import { CORE_PROFILES_SOURCE } from './core-profiles-source.ts';
import { CORE_RUNNER_SOURCE } from './core-runner-source.ts';

/**
 * The two hook adapters a conversion adds (Agent Plugins Extended §18.4.4), as the text of Node.js
 * ES modules: self-contained, since they run where the kit is not installed, and the same bytes for
 * every conversion (§18.4.1 item 5). The runtime they need is `node` on `PATH`.
 */

export const ADAPTER_RUNTIME = 'node';

const header = (purpose: string): string =>
  [
    '#!/usr/bin/env node',
    `// ${purpose}`,
    '// Written by @localix-ai/agent-plugins (Agent Plugins Extended §18.4.4): convert the package',
    '// again rather than editing it.',
    '',
  ].join('\n');

const imports = (lines: readonly string[]): string => `${lines.join('\n')}\n\n`;

/** Runs a core hook handler under Claude Code: `run-core-handler.mjs <event> -- <command> [args...]`. */
export const RUN_CORE_HANDLER_SOURCE = [
  header('Runs a core Agent Plugins hook handler under Claude Code.'),
  imports([
    "const { spawnSync } = await import('node:child_process');",
    "const { mkdirSync, readFileSync } = await import('node:fs');",
    "const { isAbsolute, resolve } = await import('node:path');",
    "const { default: proc } = await import('node:process');",
    '',
    'const [event, separator, command, ...args] = proc.argv.slice(2);',
  ]),
  CORE_PROFILES_SOURCE,
  '\n',
  CORE_RUNNER_SOURCE,
].join('');

/**
 * Runs a Claude Code `command` hook under a core client: `run-claude-code-handler.mjs --event <event>
 * [--profile <name>] [--timeout <seconds>] [--shell bash|powershell] [--exec] -- <command> [args...]`.
 */
export const RUN_CLAUDE_CODE_HANDLER_SOURCE = [
  header('Runs a Claude Code command hook under a core Agent Plugins client.'),
  imports([
    "const { spawnSync } = await import('node:child_process');",
    "const { readFileSync } = await import('node:fs');",
    "const { default: proc } = await import('node:process');",
  ]),
  CLAUDE_CODE_RUNNER_SOURCE,
].join('');
