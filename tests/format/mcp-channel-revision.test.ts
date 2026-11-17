/** mcp.md, "Push messages with channels" (Agent Plugins Extended §9.3 item 5): a docs paragraph beyond the snapshot. */
import { describe, expect, test } from 'bun:test';
import { registersAsChannel } from '../../src/entry/format.ts';

// mcp.md, "Push messages with channels" (spec §9.3 item 5): a docs paragraph beyond the snapshot.
describe('a channel server on revision 2026-07-28', () => {
  test('does not register as a channel on the v2 runtime; on the earlier handshake it does', () => {
    expect(registersAsChannel('v2', '2026-07-28')).toBe(false);
    expect(registersAsChannel('v2', '2025-11-25')).toBe(true);
  });

  test('registers on the v1 runtime', () => {
    expect(registersAsChannel('v1', '2026-07-28')).toBe(true);
  });
});
