/**
 * skills.md § Permission checks on injected commands, "Bare repository folders" (Agent Plugins
 * Extended §4.8.5 item 3): a folder looks bare when it holds a `HEAD` file directly beside `refs` or
 * `commondir`; in such a repository the skill's `allowed-tools` do not cover a Bash command that
 * contains `git` or that cannot be parsed, so it aborts unless a permission rule allows it.
 */
import { expect, test } from 'bun:test';
import { looksLikeBareRepository } from '../../src/skills/bare-repository.ts';
import type { InjectionContext } from '../../src/skills/shell-run.ts';
import { failure, run } from '../support/shell-run.ts';
import { spec } from '../support/spec.ts';

const file = (name: string) => ({ name, isFile: true, isDirectory: false });
const folder = (name: string) => ({ name, isFile: false, isDirectory: true });

spec('skills.behavior.Permission checks on injected commands.Bare repository folders', () => {
  const bare: Partial<InjectionContext> = { bareRepository: true, grants: ['Bash(git *)'] };

  test('in a bare-looking repository a Bash command that contains git is asked without the skill grants', async () => {
    const { asked } = await run(['git status'], [{}], { context: bare });
    expect(asked).toEqual([{ tool: 'Bash', command: 'git status', grants: [] }]);
  });

  test('so is a command the kit cannot parse, whatever it runs', async () => {
    const { asked } = await run(['echo "unclosed'], [{}], {
      context: { ...bare, grants: ['Bash(echo *)'] },
    });
    expect(asked[0]?.grants).toEqual([]);
  });

  test('without its grant the command aborts unless a permission rule allows it', async () => {
    const { result, runner } = await run(['git log'], [], { behavior: 'ask', context: bare });
    expect(failure(result)).toContain('Shell command permission check failed');
    expect(runner.calls).toHaveLength(0);
    const allowed = await run(['git log'], [{ stdout: 'ok' }], { context: bare });
    expect(allowed.result).toEqual({ ok: true, outputs: ['ok'] });
  });

  test('a parsed command without git keeps the grants, and outside a bare repository git keeps them too', async () => {
    const other = await run(['ls -la'], [{}], { context: { ...bare, grants: ['Bash(ls *)'] } });
    expect(other.asked[0]?.grants).toEqual(['Bash(ls *)']);
    const normal = await run(['git status'], [{}], { context: { grants: ['Bash(git *)'] } });
    expect(normal.asked[0]?.grants).toEqual(['Bash(git *)']);
  });

  test('a folder looks bare when it holds a HEAD file directly beside refs or commondir', () => {
    expect(looksLikeBareRepository([file('HEAD'), folder('refs'), folder('objects')])).toBe(true);
    expect(looksLikeBareRepository([file('HEAD'), file('commondir')])).toBe(true);
    expect(looksLikeBareRepository([file('HEAD'), folder('objects')])).toBe(false);
    expect(looksLikeBareRepository([folder('HEAD'), folder('refs')])).toBe(false);
    expect(looksLikeBareRepository([folder('.git'), file('README.md')])).toBe(false);
  });

  test('a PowerShell command keeps its grants: the rule names Bash commands', async () => {
    const { asked } = await run(['git status'], [{}], {
      context: { ...bare, shell: 'powershell', grants: ['PowerShell(git *)'] },
    });
    expect(asked[0]?.grants).toEqual(['PowerShell(git *)']);
  });
});
