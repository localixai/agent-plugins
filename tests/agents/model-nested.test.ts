/**
 * Agent Plugins Extended §5.8.2 (nested subagents run on the spawning subagent's model), §5.8.3
 * (which version an alias selects), §5.8.4 (when an allowlist blocks the model) and §5.8.6
 * (extended thinking); sub-agents.md, "Which model a nested subagent runs on", "Which version a
 * model alias selects", "When an allowlist blocks a subagent model", "Extended thinking in
 * subagents". The parent conversation is the main conversation for a subagent at depth 1 and the
 * spawning subagent for a nested one (§5.1).
 */
import { describe, expect, test } from 'bun:test';
import { resolveSubagentModel, subagentThinking } from '../../src/agents/launch-model.ts';
import { checkAgainstAllowlist, type ModelAllowlist } from '../../src/agents/model-choice.ts';
import { spec } from '../support/spec.ts';

const OPUS_MAIN = { mainModel: 'claude-opus-5-5', mainFamily: 'opus' } as const;
/** The main conversation on Opus, the spawning reviewer on Haiku (the example of §5.8.2). */
const UNDER_HAIKU = {
  ...OPUS_MAIN,
  parentModel: 'claude-haiku-5',
  parentFamily: 'haiku',
} as const;

spec('agents.behavior.Which model a nested subagent runs on', () => {
  test('a subagent a Haiku reviewer spawns runs on Haiku, with no model or with model: inherit', () => {
    expect(resolveSubagentModel(UNDER_HAIKU)).toEqual({
      model: 'claude-haiku-5',
      from: 'main',
      kind: 'inherit',
    });
    expect(resolveSubagentModel({ ...UNDER_HAIKU, definition: 'inherit' }).model).toBe(
      'claude-haiku-5',
    );
  });

  test('the built-ins Explore, Plan and general-purpose spawned by it run on Haiku too', () => {
    for (const name of ['Explore', 'Plan', 'general-purpose']) {
      const builtin = { name, connection: 'subscription' } as const;
      expect(resolveSubagentModel({ ...UNDER_HAIKU, builtin }).model).toBe('claude-haiku-5');
    }
  });

  test('a nested subagent whose own model names one runs on it', () => {
    expect(resolveSubagentModel({ ...UNDER_HAIKU, definition: 'claude-sonnet-5' }).model).toBe(
      'claude-sonnet-5',
    );
  });

  test('at depth 1 the parent is the main conversation', () => {
    expect(resolveSubagentModel(OPUS_MAIN).model).toBe('claude-opus-5-5');
  });

  test('Explore under a Fable parent follows the parent family rule, whatever the main model', () => {
    const fableParent = { ...OPUS_MAIN, parentModel: 'claude-fable-5', parentFamily: 'fable' };
    const builtin = { name: 'Explore', connection: 'console' } as const;
    expect(resolveSubagentModel({ ...fableParent, builtin }).model).toBe('opus');
  });
});

spec('agents.behavior.Which version a model alias selects', () => {
  test("a family alias of the parent's family selects the parent's exact model, [1m] included", () => {
    const parent = { ...OPUS_MAIN, parentModel: 'claude-haiku-5[1m]', parentFamily: 'haiku' };
    expect(resolveSubagentModel({ ...parent, definition: 'haiku' }).model).toBe(
      'claude-haiku-5[1m]',
    );
    expect(resolveSubagentModel({ ...parent, invocation: 'haiku' }).model).toBe(
      'claude-haiku-5[1m]',
    );
  });

  test("an alias of the main conversation's family is no longer the parent's for a nested subagent", () => {
    expect(resolveSubagentModel({ ...UNDER_HAIKU, definition: 'opus' })).toEqual({
      model: 'opus',
      from: 'definition',
      kind: 'alias',
    });
  });

  test('with the main conversation on claude-opus-4-7, model: opus runs on claude-opus-4-7', () => {
    const main = { mainModel: 'claude-opus-4-7', mainFamily: 'opus' } as const;
    expect(resolveSubagentModel({ ...main, definition: 'opus' }).model).toBe('claude-opus-4-7');
  });

  test('an unknown parent family: only opus follows the parent, and not while ANTHROPIC_DEFAULT_OPUS_MODEL is set', () => {
    const arn = 'arn:aws:bedrock:profile/parent';
    const parent = { ...OPUS_MAIN, parentModel: arn };
    expect(resolveSubagentModel({ ...parent, definition: 'opus' }).model).toBe(arn);
    expect(resolveSubagentModel({ ...parent, definition: 'sonnet' }).model).toBe('sonnet');
    expect(resolveSubagentModel({ ...parent, definition: 'opus', opusPinned: true }).model).toBe(
      'opus',
    );
  });

  test('an alias in CLAUDE_CODE_SUBAGENT_MODEL selects the version it points to (J5.20)', () => {
    expect(resolveSubagentModel({ ...UNDER_HAIKU, envModel: 'haiku' })).toEqual({
      model: 'haiku',
      from: 'env',
      kind: 'alias',
    });
  });
});

const allow = (permitted: readonly string[], newest: Record<string, string> = {}) =>
  ({
    permits: (model) => permitted.includes(model),
    newestPermitted: (family) => newest[family],
  }) satisfies ModelAllowlist;

spec('agents.behavior.When an allowlist blocks a subagent model', () => {
  const HAIKU = 'claude-haiku-5';

  test("a nested subagent falls back to the spawning subagent's model", () => {
    const allowlist = allow([HAIKU]);
    expect(
      checkAgainstAllowlist({ chosen: 'claude-sonnet-5', inherited: HAIKU, allowlist }).model,
    ).toBe(HAIKU);
  });

  test('the newest permitted version of a blocked family comes first, then the variable, then the parent', () => {
    const allowlist = allow([HAIKU, 'claude-fable-5'], { opus: 'claude-opus-4-1' });
    expect(checkAgainstAllowlist({ chosen: 'opus', inherited: HAIKU, allowlist }).model).toBe(
      'claude-opus-4-1',
    );
    const blocked = { chosen: 'claude-opus-5-5', inherited: HAIKU, allowlist };
    expect(checkAgainstAllowlist({ ...blocked, envModel: 'claude-fable-5' }).model).toBe(
      'claude-fable-5',
    );
    expect(checkAgainstAllowlist({ ...blocked, envModel: 'inherit' }).model).toBe(HAIKU);
  });

  test('the replacement is reported for the warning of a top-level subagent only', () => {
    const allowlist = allow([HAIKU]);
    const input = { chosen: 'claude-opus-5-5', inherited: HAIKU, allowlist } as const;
    expect(checkAgainstAllowlist(input).replaced).toEqual({
      requested: 'claude-opus-5-5',
      running: HAIKU,
    });
    expect(checkAgainstAllowlist({ ...input, nested: true })).toEqual({ model: HAIKU });
  });

  test('the built-in web-fetch agent is not checked: blocked, it runs on the starting conversation model', () => {
    const allowlist = allow(['claude-sonnet-5'], { haiku: 'claude-haiku-4' });
    const input = { chosen: 'haiku', inherited: 'claude-sonnet-5', allowlist } as const;
    expect(checkAgainstAllowlist({ ...input, agent: 'web-fetch' })).toEqual({
      model: 'claude-sonnet-5',
    });
    expect(
      checkAgainstAllowlist({ ...input, agent: 'web-fetch', allowlist: allow(['haiku']) }),
    ).toEqual({ model: 'haiku' });
    // Any other subagent of the same model gets the family's newest permitted version.
    expect(checkAgainstAllowlist(input).model).toBe('claude-haiku-4');
  });
});

spec('agents.behavior.Extended thinking in subagents', () => {
  test("a subagent takes the main conversation's thinking configuration: on is on, off is off", () => {
    expect(subagentThinking({ type: 'enabled', budgetTokens: 4000 })).toEqual({
      type: 'enabled',
      budgetTokens: 4000,
    });
    expect(subagentThinking({ type: 'disabled' })).toEqual({ type: 'disabled' });
    expect(subagentThinking({ type: 'adaptive' })).toEqual({ type: 'adaptive' });
  });
});

describe('the parent fields (judgment, spec/judgments.md)', () => {
  test('a parent family left out with a parent model set means the family cannot be told', () => {
    const parent = { ...OPUS_MAIN, parentModel: 'gateway/custom' };
    expect(resolveSubagentModel({ ...parent, definition: 'opus' }).model).toBe('gateway/custom');
  });
});
