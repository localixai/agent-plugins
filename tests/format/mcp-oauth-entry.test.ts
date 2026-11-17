/**
 * mcp.md § Override OAuth metadata discovery ("The URL must use `https://`") and spec §8.2, §8.7.2
 * (J8.3): an entry whose `oauth` has a wrong type or a metadata URL that is not https is invalid; the
 * loader drops it and the validator reports it at the field.
 */
import { expect, test } from 'bun:test';
import { readMcpServer } from '../../src/format/mcp-config.ts';
import { mcpFindings } from '../../src/format/validate/mcp-check.ts';
import { spec } from '../support/spec.ts';

const entry = (oauth: unknown) => ({ type: 'http', url: 'https://mcp.example.com/mcp', oauth });
const findings = (oauth: unknown) =>
  mcpFindings({ api: entry(oauth) }, new Set()).map((found) => [found.path, found.message]);

spec('mcp.behavior.Override OAuth metadata discovery', () => {
  test('an http:// metadata URL drops the entry and is reported', () => {
    const bad = {
      authServerMetadataUrl: 'http://auth.example.com/.well-known/openid-configuration',
    };
    expect(readMcpServer('api', entry(bad), '.mcp.json').server).toBeUndefined();
    expect(findings(bad)).toEqual([
      [
        'api.oauth.authServerMetadataUrl',
        'authServerMetadataUrl must use https://. The plugin loader silently drops this server at load.',
      ],
    ]);
    const good = {
      authServerMetadataUrl: 'https://auth.example.com/.well-known/openid-configuration',
    };
    expect(readMcpServer('api', entry(good), '.mcp.json').server).toBeDefined();
    expect(findings(good)).toEqual([]);
  });
});

test('a wrong-type oauth field drops the entry and draws the schema message (spec §8.3)', () => {
  expect(readMcpServer('api', entry({ scopes: ['a'] }), '.mcp.json').server).toBeUndefined();
  expect(readMcpServer('api', entry('x'), '.mcp.json').server).toBeUndefined();
  expect(findings({ scopes: ['a'] })).toEqual([
    [
      'api.oauth.scopes',
      'Invalid input: expected string, received array. The plugin loader silently drops this server at load.',
    ],
  ]);
  expect(
    readMcpServer('api', entry({ clientId: 'c', other: 1 }), '.mcp.json').server,
  ).toBeDefined();
});
