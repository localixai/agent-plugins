/**
 * mcp.md, "Choose a time to live and a poll interval" (Agent Plugins Extended §8.10): a server keeps a
 * finished task's result for at least an hour and sends `ttlMs` (the longest run time plus the time
 * it keeps the result, or `null`) and `pollIntervalMs` (10000 to 30000 for a job over a minute, 2000
 * for a shorter one).
 */
import { expect, test } from 'bun:test';
import { recommendedMcpTaskTiming } from '../../src/format/mcp-task-timing.ts';
import { spec } from '../support/spec.ts';

const MINUTE = 60 * 1000;

spec('mcp.behavior.Choose a time to live and a poll interval', () => {
  test('the docs example: a 30-minute job kept one hour has ttlMs 5400000', () => {
    expect(recommendedMcpTaskTiming({ longestRunMs: 30 * MINUTE }).ttlMs).toBe(5_400_000);
    expect(
      recommendedMcpTaskTiming({ longestRunMs: 30 * MINUTE, keepResultMs: 60 * MINUTE }).ttlMs,
    ).toBe(5_400_000);
  });

  test('the result is kept at least an hour, and null means no limit', () => {
    expect(recommendedMcpTaskTiming({ longestRunMs: MINUTE, keepResultMs: 1000 }).ttlMs).toBe(
      61 * MINUTE,
    );
    expect(recommendedMcpTaskTiming({ longestRunMs: MINUTE, unlimited: true }).ttlMs).toBeNull();
  });

  test('a job over a minute polls every 10 to 30 seconds, a shorter one every 2 seconds', () => {
    expect(recommendedMcpTaskTiming({ longestRunMs: 5 * MINUTE }).pollIntervalMs).toEqual({
      min: 10_000,
      max: 30_000,
    });
    expect(recommendedMcpTaskTiming({ longestRunMs: 30_000 }).pollIntervalMs).toEqual({
      min: 2000,
      max: 2000,
    });
  });
});
