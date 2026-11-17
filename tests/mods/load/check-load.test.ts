/**
 * plugins/mods/troubleshoot.md § Check whether mods can load: `claude plugin test`, run from a
 * directory that holds no mod, prints a message that says whether the setup lets mods load at all.
 */
import { expect, test } from 'bun:test';
import { policy } from '../../support/mods-load.ts';
import { runPluginTests } from '../../support/mods-testing.ts';
import { spec } from '../../support/spec.ts';

spec('mods.troubleshoot.Check whether mods can load.no hooks module to load', () => {
  test('mods can load: the command finds no mod to test in this directory and says `no hooks module to load`', async () => {
    const run = await runPluginTests({ name: 'empty', tests: {} });
    expect(run.lines).toEqual([
      'claude plugin test: .: no hooks module to load; there is no hooks/hooks.json naming one in "modules"',
    ]);
    expect(run.exitCode).toBe(0);
  });

  test('the same message under every setting that does not turn mods off, an organization limiting them to its own included', async () => {
    const limited = policy({
      managed: {
        pluginConfigs: {
          'cc-plugin-sec-default@builtin': { options: { allowManagedModsOnly: true } },
        },
      },
    });
    const run = await runPluginTests({ name: 'empty', tests: {}, policy: limited });
    expect(run.lines).toEqual([
      'claude plugin test: .: no hooks module to load; there is no hooks/hooks.json naming one in "modules"',
    ]);
  });
});

spec('mods.troubleshoot.Check whether mods can load.hooks modules are turned off here', () => {
  test("`disableAllHooks` in your own settings blocks your mods: Claude Code 2.1.325's line", async () => {
    const run = await runPluginTests({
      name: 'empty',
      tests: {},
      policy: policy({ user: { disableAllHooks: true } }),
    });
    expect(run.lines).toEqual([
      'claude plugin test: hooks modules are turned off here (disableAllHooks, allowManagedHooksOnly or a policy)',
    ]);
    expect(run.exitCode).toBe(1);
  });

  test('your organization`s policy blocks them the same way, and the rollout switch has its own message', async () => {
    const run = await runPluginTests({
      name: 'empty',
      tests: {},
      policy: policy({ managed: { disableAllHooks: true } }),
    });
    expect(run.lines).toEqual([
      'claude plugin test: hooks modules are turned off here (disableAllHooks, allowManagedHooksOnly or a policy)',
    ]);
    const onlyManaged = await runPluginTests({
      name: 'empty',
      tests: {},
      policy: policy({ managed: { allowManagedHooksOnly: true } }),
    });
    expect(onlyManaged.lines).toEqual([
      'claude plugin test: hooks modules are turned off here (disableAllHooks, allowManagedHooksOnly or a policy)',
    ]);
    expect(onlyManaged.exitCode).toBe(1);
    const remote = await runPluginTests({
      name: 'empty',
      tests: {},
      policy: policy({ rollout: 'served-off' }),
    });
    expect(remote.lines[0]).toContain('hooks modules are turned off in this process');
    expect(remote.lines[0]).not.toContain('turned off here');
  });
});
