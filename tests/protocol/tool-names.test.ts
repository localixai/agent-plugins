/**
 * tools-reference.md § the tool table: the canonical tool names. Whether the kit's lists agree with
 * the SDK is a compile-time check in `src/protocol/tool-names.ts` (`ToolNamesMatchSdk`); these
 * tests guard what a type cannot: the table's size, order, and that the lists do not overlap.
 * Behaviour per tool has its own ids (`tools.tool.*`), so nothing here is under spec().
 */
import { describe, expect, test } from 'bun:test';
import {
  DOCS_ONLY_TOOLS,
  DOCUMENTED_TOOLS,
  REMOVED_TOOLS,
  SDK_ONLY_TOOLS,
} from '../../src/protocol/tool-names.ts';

const key = (name: string): string => name.toLowerCase();

describe('tool names', () => {
  test('the table lists 53 tools, each once, in alphabetical order as the docs print them', () => {
    expect(DOCUMENTED_TOOLS).toHaveLength(53);
    expect(new Set(DOCUMENTED_TOOLS).size).toBe(53);
    expect([...DOCUMENTED_TOOLS]).toEqual(
      [...DOCUMENTED_TOOLS].toSorted((a, b) => (key(a) < key(b) ? -1 : 1)),
    );
  });

  test('a name is a canonical identifier: no label such as "Stop Task"', () => {
    for (const name of [...DOCUMENTED_TOOLS, ...SDK_ONLY_TOOLS]) {
      expect(name).toMatch(/^[A-Za-z_]+$/);
    }
  });

  test('TaskOutput is the removed tool and is still a documented name', () => {
    expect(REMOVED_TOOLS).toEqual(['TaskOutput']);
    expect(DOCUMENTED_TOOLS).toContain('TaskOutput');
  });

  test('SDK-only tools are not in the docs table', () => {
    const documented = new Set<string>(DOCUMENTED_TOOLS);
    expect(SDK_ONLY_TOOLS.filter((name) => documented.has(name))).toEqual([]);
    expect(SDK_ONLY_TOOLS).toHaveLength(8);
  });

  test('SetGoal is an SDK-only name and ProposeGoal is gone (SDK 0.3.330)', () => {
    expect(SDK_ONLY_TOOLS).toContain('SetGoal');
    expect([...DOCUMENTED_TOOLS, ...SDK_ONLY_TOOLS]).not.toContain('ProposeGoal');
  });

  test('the claude.ai account tools are documented names the SDK does not list', () => {
    expect([...DOCS_ONLY_TOOLS]).toEqual([
      'ListPlugins',
      'ListSkills',
      'SearchPlugins',
      'SearchSkills',
      'SuggestPluginInstall',
      'SuggestSkills',
    ]);
    const sdkOnly = new Set<string>(SDK_ONLY_TOOLS);
    expect(DOCS_ONLY_TOOLS.filter((name) => sdkOnly.has(name))).toEqual([]);
  });
});
