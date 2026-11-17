/**
 * env-vars.md, CLAUDE_CODE_MCP_ALLOWLIST_ENV, and spec J3.22: under the allowlist a plugin's stdio
 * server still has the path variables its plugin exports. A judgment of the spec, outside spec().
 */
import { describe, expect, test } from 'bun:test';
import { mcpStdioEnvironment } from '../../src/format/mcp-knobs.ts';

describe("the plugin's path variables under the allowlist (spec J3.22)", () => {
  test('the exported CLAUDE_PLUGIN_ROOT and CLAUDE_PLUGIN_DATA stay; the configured env still wins', () => {
    const exported = { CLAUDE_PLUGIN_ROOT: '/p', CLAUDE_PLUGIN_DATA: '/d' };
    const out = mcpStdioEnvironment({
      snapshot: { PATH: '/usr/bin', EDITOR: 'vim', CLAUDE_CODE_MCP_ALLOWLIST_ENV: '1' },
      configured: { CLAUDE_PLUGIN_DATA: '/mine' },
      exported,
      projectDir: '/work',
    });
    expect(out).toEqual({
      PATH: '/usr/bin',
      CLAUDE_PLUGIN_ROOT: '/p',
      CLAUDE_PLUGIN_DATA: '/mine',
      CLAUDE_PROJECT_DIR: '/work',
    });
  });
});
