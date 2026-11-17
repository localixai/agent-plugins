/**
 * agent-view.md § What carries over when you background (the configuration flags, what carries,
 * relaunches or stops, the `Background this session?` dialog, a subagent that owns a monitor) and §
 * Settings, plugins, and MCP servers (what agent view applies to itself and passes on). No
 * requirement id covers these paragraphs, so the tests stand outside `spec(...)`
 * (spec/judgments.md).
 */
import { describe, expect, test } from 'bun:test';
import {
  agentViewFlags,
  backgroundSessionFlags,
  planBackgrounding,
} from '../../src/cli/background-session.ts';
import type { GivenFlags } from '../../src/cli/flags/types.ts';

const launch: GivenFlags = {
  '--plugin-dir': ['./fmt'],
  '--mcp-config': ['./mcp.json'],
  '--strict-mcp-config': true,
  '--settings': './s.json',
  '--setting-sources': 'user',
  '--add-dir': ['../lib'],
  '--permission-mode': 'plan',
  '--bare': true,
};

describe('flags of a background session', () => {
  test('the configuration flags carry over; the others do not', () => {
    expect(backgroundSessionFlags(launch)).toEqual({
      '--plugin-dir': ['./fmt'],
      '--mcp-config': ['./mcp.json'],
      '--strict-mcp-config': true,
      '--settings': './s.json',
      '--setting-sources': 'user',
      '--add-dir': ['../lib'],
    });
    expect(backgroundSessionFlags({ '--bare': true })).toEqual({});
  });

  test('agent view applies three to itself and passes every configuration flag on', () => {
    const flags = agentViewFlags(launch);
    expect(flags.own).toEqual({
      '--settings': './s.json',
      '--setting-sources': 'user',
      '--plugin-dir': ['./fmt'],
    });
    expect(flags.dispatched).toEqual(backgroundSessionFlags(launch));
  });
});

describe('work when the session goes to the background', () => {
  test('shells, subagents, loops and comment replies carry; a workflow waits; a monitor stops', () => {
    const plan = planBackgrounding([
      { id: 'b1', kind: 'shell' },
      { id: 'a1', kind: 'subagent' },
      { id: 'l1', kind: 'loop' },
      { id: 'r1', kind: 'artifact-replies' },
      { id: 'w1', kind: 'workflow' },
      { id: 'm1', kind: 'monitor' },
    ]);
    expect(plan.fates).toEqual({
      b1: 'carries',
      a1: 'carries',
      l1: 'carries',
      r1: 'carries',
      w1: 'relaunch',
      m1: 'stops',
    });
    expect(plan.confirm).toBe(true);
  });

  test('a subagent that owns a monitor, at any depth, stops with it', () => {
    const plan = planBackgrounding([
      { id: 'outer', kind: 'subagent' },
      { id: 'inner', kind: 'subagent', startedBy: 'outer' },
      { id: 'm', kind: 'monitor', startedBy: 'inner' },
      { id: 'free', kind: 'subagent' },
      { id: 'sh', kind: 'shell', startedBy: 'free' },
    ]);
    expect(plan.fates).toMatchObject({ outer: 'stops', inner: 'stops', free: 'carries' });
  });

  test('nothing that stops: no dialog', () => {
    expect(planBackgrounding([{ id: 'b', kind: 'shell' }]).confirm).toBe(false);
    expect(planBackgrounding([]).confirm).toBe(false);
  });
});
