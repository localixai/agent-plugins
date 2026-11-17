/**
 * The core tool profiles Claude Code calls get (Agent Plugins Extended §18.2.7, Judgment J18.16;
 * core §7.3.6 and §7.3.7, the profile sections) and how a core handler's output is read (core
 * §7.3.3, §7.3.2 item 5; §18.1.2, "Failures"). Shapes on the Claude Code side are those of
 * `sdk-tools.d.ts` 0.3.321. Judgment-style tests outside `spec(...)`: chapter 18 has no ids.
 */
import { describe, expect, test } from 'bun:test';
import { readCoreRun } from '../../src/hooks/core/output.ts';
import { completedProfile } from '../../src/hooks/core/profile-results.ts';
import { pendingProfile } from '../../src/hooks/core/profiles.ts';

const context = { cwd: '/work', facts: {} };
const ok = (response: unknown) => ({ ok: true as const, response });
const failed = (error: string) => ({ ok: false as const, error });

describe('§18.2.7: the pending profile of each tool', () => {
  test('shell, file.read, file.modify, web.fetch, web.search', () => {
    expect(pendingProfile('Bash', { command: 'ls' }, context)).toEqual({
      name: 'shell',
      input: { command: 'ls' },
    });
    expect(pendingProfile('PowerShell', { command: 'dir' }, context)?.name).toBe('shell');
    expect(pendingProfile('Read', { file_path: '/a' }, context)).toEqual({
      name: 'file.read',
      input: { path: '/a' },
    });
    expect(pendingProfile('Write', { file_path: '/a', content: '' }, context)?.input).toEqual({
      operations: [{ kind: 'write', path: '/a' }],
    });
    expect(
      pendingProfile('Edit', { file_path: '/a', old_string: '', new_string: 'x' }, context)?.input,
    ).toEqual({
      operations: [{ kind: 'edit', path: '/a' }],
    });
    expect(pendingProfile('WebFetch', { url: 'https://x.dev/a', prompt: 'p' }, context)).toEqual({
      name: 'web.fetch',
      input: { url: 'https://x.dev/a' },
    });
    expect(pendingProfile('WebSearch', { query: 'q' }, context)?.input).toEqual({ queries: ['q'] });
  });

  test('J18.16: a relative notebook_path is made absolute against the working directory', () => {
    expect(
      pendingProfile('NotebookEdit', { notebook_path: 'n.ipynb', new_source: '' }, context)?.input,
    ).toEqual({
      operations: [{ kind: 'edit', path: '/work/n.ipynb' }],
    });
  });

  test('none for a Monitor with ws, a WebFetch of a non-web URL, and the tools of no operation', () => {
    expect(pendingProfile('Monitor', { ws: 'wss://x' }, context)).toBeUndefined();
    expect(pendingProfile('Monitor', { command: 'tail -f x' }, context)?.name).toBe('shell');
    expect(pendingProfile('WebFetch', { url: 'file:///etc/passwd' }, context)).toBeUndefined();
    for (const tool of ['Glob', 'Grep', 'Agent', 'Skill', 'AskUserQuestion', 'TodoWrite'])
      expect(pendingProfile(tool, { pattern: 'x' }, context)).toBeUndefined();
  });

  test('mcp: the declared tool name and the arguments; the host’s name wins', () => {
    expect(pendingProfile('mcp__plugin_acme_tools_notes__latest_tag', { a: 1 }, context)).toEqual({
      name: 'mcp',
      input: { name: 'latest_tag', arguments: { a: 1 } },
    });
    const named = { cwd: '/work', facts: { mcpToolName: 'x__y' } };
    expect(pendingProfile('mcp__s__x__y', undefined, named)?.input).toEqual({
      name: 'x__y',
      arguments: {},
    });
  });

  test('J18.21: no mcp profile when the full name does not determine the declared name', () => {
    expect(pendingProfile('mcp__s__x__y', {}, context)).toBeUndefined();
    expect(pendingProfile('mcp__s___y', {}, context)).toBeUndefined();
    expect(pendingProfile('mcp__s__', {}, context)).toBeUndefined();
  });
});

describe('§18.2.7: the completed profile of each tool', () => {
  test('shell: output and exit code; none without an exit code (J18.16)', () => {
    const response = { stdout: 'a', stderr: '', interrupted: false };
    expect(completedProfile('Bash', { command: 'x' }, ok(response), context)?.result).toEqual({
      output: 'a',
      exit_code: 0,
    });
    expect(
      completedProfile('Bash', { command: 'x' }, failed('Exit code 3\nbad'), context)?.result,
    ).toEqual({
      output: 'bad',
      exit_code: 3,
    });
    expect(
      completedProfile('Bash', { command: 'x', run_in_background: true }, ok(response), context),
    ).toBeUndefined();
    expect(
      completedProfile(
        'Bash',
        { command: 'x' },
        ok({ ...response, backgroundTaskId: 'b' }),
        context,
      ),
    ).toBeUndefined();
    expect(
      completedProfile('Bash', { command: 'x' }, failed('Shell could not start'), context),
    ).toBeUndefined();
    expect(completedProfile('Monitor', { command: 'x' }, ok({}), context)).toBeUndefined();
  });

  test('J18.21: no exit code 0 is assumed when returnCodeInterpretation is set', () => {
    const response = { stdout: '', stderr: '', interrupted: false, returnCodeInterpretation: 'r' };
    expect(completedProfile('Bash', { command: 'x' }, ok(response), context)).toBeUndefined();
    const observed = { cwd: '/work', facts: { exitCode: 1 } };
    expect(completedProfile('Bash', { command: 'x' }, ok(response), observed)?.result).toEqual({
      output: '',
      exit_code: 1,
    });
  });

  test('file.read: the text Claude received, images and PDFs as binary, failures as error', () => {
    const text = {
      type: 'text',
      file: { filePath: '/a', content: 'x\ny', numLines: 2, startLine: 9, totalLines: 10 },
    };
    // J18.16: the text is the call's tool_result string from the host, never `content` renumbered.
    expect(completedProfile('Read', { file_path: '/a' }, ok(text), context)).toBeUndefined();
    const given = { cwd: '/work', facts: { readText: '9→x\n10→y' } };
    expect(completedProfile('Read', { file_path: '/a' }, ok(text), given)?.result).toEqual({
      content: [{ type: 'text', text: '9→x\n10→y' }],
    });
    const image = { type: 'image', file: { base64: 'AA==', type: 'image/png', originalSize: 1 } };
    expect(completedProfile('Read', { file_path: '/a' }, ok(image), context)?.result).toEqual({
      content: [{ type: 'binary', media_type: 'image/png', data: 'AA==' }],
    });
    const pdf = { type: 'pdf', file: { filePath: '/a', base64: 'BB==', originalSize: 1 } };
    expect(completedProfile('Read', { file_path: '/a' }, ok(pdf), context)?.result).toEqual({
      content: [{ type: 'binary', media_type: 'application/pdf', data: 'BB==' }],
    });
    expect(
      completedProfile('Read', { file_path: '/a' }, failed('File does not exist.'), context)
        ?.result,
    ).toEqual({
      error: 'File does not exist.',
    });
  });

  test('file.modify is {} or { error }; web.fetch needs an HTTP status', () => {
    expect(completedProfile('Edit', { file_path: '/a' }, ok({}), context)?.result).toEqual({});
    expect(
      completedProfile('Edit', { file_path: '/a' }, failed('no match'), context)?.result,
    ).toEqual({ error: 'no match' });
    const fetched = {
      bytes: 1,
      code: 404,
      codeText: 'Not Found',
      result: 'gone',
      durationMs: 1,
      url: 'https://x.dev',
    };
    expect(
      completedProfile('WebFetch', { url: 'https://x.dev' }, ok(fetched), context)?.result,
    ).toEqual({
      status_code: 404,
      content: [{ type: 'text', text: 'gone' }],
    });
    expect(
      completedProfile('WebFetch', { url: 'https://x.dev' }, failed('DNS'), context),
    ).toBeUndefined();
  });

  test('web.search: one item per source; a string entry gives none (J18.16)', () => {
    const response = {
      query: 'q',
      durationSeconds: 1,
      results: [
        'Here is what I found',
        {
          tool_use_id: 't',
          content: [
            { title: 'T', url: 'https://u', snippet: 's' },
            { title: 'U', url: 'https://v' },
          ],
        },
      ],
    };
    expect(completedProfile('WebSearch', { query: 'q' }, ok(response), context)?.result).toEqual({
      items: [
        { title: 'T', url: 'https://u', snippet: 's' },
        { title: 'U', url: 'https://v' },
      ],
    });
  });

  test('mcp: the CallToolResult the host gives; none without it', () => {
    const facts = { mcpResult: { content: [], isError: true } };
    expect(completedProfile('mcp__s__t', {}, failed('x'), { cwd: '/w', facts })?.result).toEqual(
      facts.mcpResult,
    );
    expect(completedProfile('mcp__s__t', {}, ok('processed'), context)).toBeUndefined();
  });
});

describe('core §7.3.3: what a core handler’s run comes to', () => {
  const run = (stdout: string, exitCode: number | null = 0) => ({
    exitCode,
    stdout,
    stderr: '',
    timedOut: false,
  });

  test('no bytes or only JSON whitespace is no opinion', () => {
    expect(readCoreRun('Stop', run(''), 1000)).toEqual({});
    expect(readCoreRun('Stop', run(' \r\n\t'), 1000)).toEqual({});
  });

  test('one object of the event’s closed shapes is a result', () => {
    expect(readCoreRun('PreToolUse', run(' {"decision":"allow"} \n'), 1000)).toEqual({
      result: { decision: 'allow' },
    });
    expect(readCoreRun('Stop', run('{"decision":"continue","reason":"r"}'), 1000)).toEqual({
      result: { decision: 'continue', reason: 'r' },
    });
  });

  test('{}, an extra field, an empty reason, two objects and a nonzero status are failures', () => {
    for (const stdout of [
      '{}',
      '{"decision":"allow","x":1}',
      '{"decision":"deny","reason":""}',
      '{"decision":"allow"}{"decision":"allow"}',
    ])
      expect(readCoreRun('PreToolUse', run(stdout), 1000)).toHaveProperty('failure');
    expect(readCoreRun('PreToolUse', run('{"decision":"allow"}', 2), 1000)).toHaveProperty(
      'failure',
    );
  });

  test('J18.22: each failure’s cause is the one a Claude Code exec-form command hook gets', () => {
    const failing = (stderr: string) => ({ ...run('', 1), stderr });
    expect(readCoreRun('PreToolUse', failing('first\nsecond\n'), 1000)).toEqual({
      failure: 'Failed with non-blocking status code: first',
    });
    expect(readCoreRun('PreToolUse', failing(' \n'), 1000)).toEqual({
      failure: 'Failed with non-blocking status code: no stderr output',
    });
    expect(
      readCoreRun('Stop', { ...run('', null), startError: 'ENOENT: no such file' }, 1),
    ).toEqual({ failure: 'ENOENT: no such file' });
    const late = { ...run(''), timedOut: true, exitCode: null };
    expect(readCoreRun('Stop', late, 600_000)).toEqual({ failure: 'timed out' });
    expect(readCoreRun('UserPromptSubmit', late, 30_000)).toEqual({
      failure: "timed out after 30s; the hook's output was discarded",
    });
    expect(readCoreRun('Stop', run('text'), 1000)).toEqual({
      failure: 'wrote output that is not one JSON object',
    });
    expect(readCoreRun('Stop', run('{}'), 1000)).toEqual({
      failure: 'wrote an object that is not a Stop result',
    });
  });
});
