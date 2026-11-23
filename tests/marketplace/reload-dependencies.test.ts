/**
 * Spec §16.12 item 1, **Dependencies** (plugins/cli-reference.md, "/reload-plugins"; changelog
 * 2.1.32x): when a reloaded plugin is missing a dependency the client can resolve (§16.23), the reload
 * installs it; when the reload applies, the summary includes `(+ N dependencies: <names>) resolved`.
 * When the reload is not applied (item 2) or the summary says the dependencies were `installed but not
 * applied`, `/reload-plugins --force` applies them. The joining of the clause is a row in
 * spec/judgments.md.
 */
import { expect, test } from 'bun:test';
import { installClosure } from '../../src/marketplace/dependency-install-plan.ts';
import {
  reloadGate,
  reloadSummary,
  type ReloadCounts,
} from '../../src/marketplace/reload-summary.ts';
import { facts, installed, plugin } from '../support/dependency-facts.ts';
import { spec } from '../support/spec.ts';

const counts = (extra: Partial<ReloadCounts> = {}): ReloadCounts => ({
  plugins: 2,
  skills: 1,
  agents: 0,
  hooks: 0,
  mcpServers: 1,
  lspServers: 0,
  errors: 0,
  ...extra,
});

const HEAD =
  'Reloaded: 2 plugins · 1 skills · 0 agents · 0 hooks · 1 plugin MCP servers · 0 plugin LSP servers';

spec('cli.plugin.Dependencies installed during a reload', () => {
  test('the reload installs a missing dependency it can resolve', () => {
    const graph = facts(
      [plugin('formatter', { manifest: ['lib'] }), plugin('lib')],
      [installed('formatter@m')],
    );
    const plan = installClosure(
      { root: 'formatter@m', scope: 'user', operation: 'reload-plugins' },
      graph,
    );
    expect(plan.ok).toBe(true);
    expect(plan.steps.map((step) => [step.id, step.action])).toEqual([['lib@m', 'install']]);
  });

  test('a reload that applies names them as resolved', () => {
    expect(reloadSummary(counts({ resolvedDependencies: ['lib@m'] }), { interactive: true })).toBe(
      `${HEAD} (+ 1 dependencies: lib@m) resolved`,
    );
  });

  test('installed but not applied, the summary says so and points at --force', () => {
    expect(
      reloadSummary(counts({ unappliedDependencies: ['lib@m', 'base@m'] }), { interactive: true }),
    ).toBe(
      `${HEAD} (+ 2 dependencies: lib@m, base@m) installed but not applied. Run /reload-plugins --force to apply.`,
    );
  });

  test('a reload held back for the prompt cache applies with --force', () => {
    const change = {
      addedMcp: ['lib-server'],
      removedMcp: [],
      lsp: 'unchanged' as const,
      invalidatesPromptCache: true,
    };
    const held = reloadGate(change, { force: false, interactive: true });
    expect(held.apply).toBe(false);
    expect('message' in held && held.message).toContain('Run /reload-plugins --force to apply.');
    expect(reloadGate(change, { force: true, interactive: true }).apply).toBe(true);
  });
});
