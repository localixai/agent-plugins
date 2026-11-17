/**
 * mcp.md § discovery cache and spec §8.8 item 1: only a remote (`http` or `sse`) server is served
 * from the discovery cache. The transport rule is the spec's, tested outside spec().
 */
import { describe, expect, test } from 'bun:test';
import { mcpStartupWaitsFor } from '../../src/format/mcp-startup.ts';

describe('only an http or sse server is served from the discovery cache (spec §8.8 item 1)', () => {
  test('a cached stdio or ws server with alwaysLoad is still waited for', () => {
    const interactive = { printMode: false };
    for (const transport of ['stdio', 'ws'] as const) {
      const server = { alwaysLoad: true, cached: true, transport };
      expect(mcpStartupWaitsFor(() => undefined, server, interactive)).toBe(true);
    }
    for (const transport of ['http', 'sse'] as const) {
      const server = { alwaysLoad: true, cached: true, transport };
      expect(mcpStartupWaitsFor(() => undefined, server, interactive)).toBe(false);
    }
  });
});
