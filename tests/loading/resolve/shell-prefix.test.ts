/**
 * env-vars.md, "Variables — CLAUDE_CODE_SHELL_PREFIX" (Agent Plugins Extended §8.2.1, J8.26): a stdio
 * server starts as `<prefix> '<command line>'`, the line being `command` and each `args` element,
 * each POSIX-quoted. A docs paragraph beyond the requirement snapshot, so outside `spec(...)`.
 */
import { describe, expect, test } from 'bun:test';
import {
  mcpStdioStart,
  posixQuote,
  resolveMcpServer,
  shellPrefixed,
} from '../../../src/entry/loading.ts';
import { scopeOf } from '../../support/hook-scope.ts';
import { fakeEnvironment } from '../../support/ports.ts';

const PREFIX = '/usr/local/bin/wrap.sh';

describe('a stdio server under CLAUDE_CODE_SHELL_PREFIX', () => {
  test('starts as the prefix with the quoted command line as its one argument', () => {
    const env = fakeEnvironment({ CLAUDE_CODE_SHELL_PREFIX: PREFIX });
    expect(mcpStdioStart({ command: 'npx', args: ['-y', 'my server'] }, env)).toEqual({
      form: 'shell',
      command: `${PREFIX} ''\\''npx'\\'' '\\''-y'\\'' '\\''my server'\\'''`,
      shell: 'bash',
    });
  });

  test('without the prefix, or with it blank, it is an exec of command and args', () => {
    for (const value of [undefined, '', '  ']) {
      const env = fakeEnvironment(value === undefined ? {} : { CLAUDE_CODE_SHELL_PREFIX: value });
      expect(mcpStdioStart({ command: 'npx', args: ['-y', 'pkg'] }, env)).toEqual({
        form: 'exec',
        command: 'npx',
        args: ['-y', 'pkg'],
      });
    }
  });

  test('a plugin server resolves with its start, after substitution', async () => {
    const scope = scopeOf({ environment: fakeEnvironment({ CLAUDE_CODE_SHELL_PREFIX: PREFIX }) });
    const { resolved } = await resolveMcpServer(
      'db',
      { command: '${CLAUDE_PLUGIN_ROOT}/db', args: ['--port', '5'] },
      scope,
    );
    const line = ['/plugins/formatter@acme-tools/db', '--port', '5'].map(posixQuote).join(' ');
    expect(resolved?.start).toEqual({
      form: 'shell',
      command: shellPrefixed(PREFIX, line),
      shell: 'bash',
    });
  });
});

describe('POSIX quoting', () => {
  test("wraps in single quotes and writes an embedded quote as '\\''", () => {
    expect(posixQuote("it's")).toBe(`'it'\\''s'`);
    expect(posixQuote('')).toBe(`''`);
  });
});
