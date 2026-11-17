/**
 * plugins/manifest-reference.md § Validate the manifest (the MCP checks of `claude plugin validate`:
 * errors for "a remote `url` that is not a valid absolute URL", warnings for "an `http://` or `ws://`
 * URL to a non-loopback host, and a header value that looks like a literal credential"); spec §2.9,
 * §8 item 3 ("once its references are set aside"), §17.13.3. The wording and which values look like a
 * credential are oracle 2.1.325's, outside `spec(...)` and in spec/judgments.md.
 */
import { describe, expect, test } from 'bun:test';
import { validatePlugin } from '../../src/index.ts';
import { memoryFiles } from '../support/ports.ts';
import { spec } from '../support/spec.ts';

const MANIFEST = JSON.stringify({
  name: 'p',
  version: '1.0.0',
  description: 'd',
  author: { name: 'a' },
});

const findings = async (servers: Record<string, unknown>, inline = false) => {
  const tree = inline
    ? {
        '.claude-plugin/plugin.json': JSON.stringify({
          ...JSON.parse(MANIFEST),
          mcpServers: servers,
        }),
      }
    : {
        '.claude-plugin/plugin.json': MANIFEST,
        '.mcp.json': JSON.stringify({ mcpServers: servers }),
      };
  const report = await validatePlugin({ files: memoryFiles(tree) });
  return report.issues.map((issue) => [issue.severity, issue.path, issue.message] as const);
};

const http = (url: string, headers?: Record<string, string>) => ({
  type: 'http',
  url,
  ...(headers === undefined ? {} : { headers }),
});

const NOT_ABSOLUTE =
  'url is not a valid absolute URL (for example "https://host/path"). The plugin loader marks this server as invalid config and it never connects.';
const WS_CLEARTEXT =
  'url uses ws:// to a non-loopback host — requests, headers and any credentials travel in cleartext. Use wss://; plugin directories may reject insecure remote server urls.';
const CREDENTIAL =
  'header value looks like a literal credential. Everything shipped in a plugin is readable by everyone who installs it; reference a sensitive userConfig option (${user_config.KEY}) or an environment variable (${VAR}) instead of committing the value.';

spec('cli.rule.Validate the manifest.Errors', () => {
  test.each(['not a url', '/api/mcp', 'https://'])('a remote url %p is an error', async (url) => {
    for (const inline of [false, true]) {
      const found = await findings({ api: http(url) }, inline);
      expect(found.map(([severity, path]) => [severity, path])).toEqual([
        ['error', 'mcpServers.api.url'],
      ]);
    }
  });

  test('a url written empty is a placeholder, not an error (spec §8.2, §8.8 item 6)', async () => {
    for (const inline of [false, true]) {
      const found = await findings({ api: http('') }, inline);
      expect(found.filter(([severity]) => severity === 'error')).toEqual([]);
    }
  });

  test('a url that holds a reference is checked with its references set aside', async () => {
    expect(await findings({ api: http('${BASE_URL}/mcp') })).toEqual([]);
    expect(await findings({ api: http('https://${HOST}/mcp') })).toEqual([]);
  });
});

spec('cli.rule.Validate the manifest.Warnings', () => {
  test('a ws:// url to a host that is not loopback warns; to loopback, or wss://, it does not', async () => {
    const found = await findings({
      far: { type: 'ws', url: 'ws://example.com/mcp' },
      near: { type: 'ws', url: 'ws://localhost:3000/mcp' },
      safe: { type: 'ws', url: 'wss://example.com/mcp' },
    });
    expect(found.map(([severity, path]) => [severity, path])).toEqual([
      ['warning', 'mcpServers.far.url'],
    ]);
  });

  test('a header value that looks like a literal credential warns; a reference does not', async () => {
    for (const inline of [false, true]) {
      const found = await findings(
        {
          api: http('https://example.com/mcp', {
            Authorization: 'Bearer sk-abcdef1234567890abcdef',
            'X-Token': 'Bearer ${TOKEN}',
            Accept: 'application/json',
          }),
        },
        inline,
      );
      expect(found.map(([severity, path]) => [severity, path])).toEqual([
        ['warning', 'mcpServers.api.headers.Authorization'],
      ]);
    }
  });
});

describe('the MCP url and header checks: wording and the credential shapes (oracle 2.1.325)', () => {
  test('the messages', async () => {
    const found = await findings({
      a: http('not a url'),
      b: { type: 'ws', url: 'ws://example.com/x' },
      c: http('https://e.com', { 'X-Api-Key': 'abcd1234efgh5678ijkl9012' }),
    });
    expect(found).toEqual([
      ['error', 'mcpServers.a.url', NOT_ABSOLUTE],
      ['warning', 'mcpServers.b.url', WS_CLEARTEXT],
      ['warning', 'mcpServers.c.headers.X-Api-Key', CREDENTIAL],
    ]);
  });

  test.each([
    ['Authorization', 'Bearer abcd1234efgh5678ijkl9012', true],
    ['Authorization', 'Bearer abcd1234efgh5678', false],
    ['X-Api-Key', 'abcd1234efgh5678ijkl', true],
    ['X-Api-Key', 'abcd1234efgh5678', false],
    ['Cookie', 'abcd1234efgh5678ijkl9012', false],
    ['X-Other', 'abcd1234efgh5678ijkl9012', false],
    ['X-Other', 'sk-abcdefghij1234567890', true],
    ['X-Other', 'sk-abcdef12', false],
    ['X-Other', 'ghp_abcdefghijklmnopqrstuvwxyz0123456789', true],
    ['X-Other', 'glpat-abcdefghij1234567890', true],
    ['X-Other', 'xoxb-1234-5678-abcdefghij', true],
    ['X-Other', 'AKIAABCDEFGHIJKLMNOP', true],
    ['X-Other', 'AKIAIOSFODNN7EXAMPLE', false],
    ['X-Other', 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc', true],
  ])('%s: %s looks like a credential: %p', async (name, value, warns) => {
    const found = await findings({ api: http('https://e.com', { [name]: value }) });
    expect(found.length).toBe(warns ? 1 : 0);
  });
});
