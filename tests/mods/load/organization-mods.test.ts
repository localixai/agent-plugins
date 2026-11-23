/**
 * Agent Plugins Extended §14.9.5 item 1 (plugins/mods/admin.md § Install your organization's mods
 * and set the order, 2.1.330): a mod from a remote marketplace counts as the organization's when
 * managed settings enable it, declare its marketplace and list it in `prependPlugins` or
 * `appendPlugins` (Claude Code 2.1.324); §14.9.1, the guard refuses every other mod a user enabled.
 */
import { expect, test } from 'bun:test';
import { orderMods } from '../../../src/mods/load/index.ts';
import { installed, managed, modPlugin, policy, session } from '../../support/mods-load.ts';
import { spec } from '../../support/spec.ts';

const names = (order: ReturnType<typeof orderMods>) =>
  order.ordered.map((mod) => `${mod.candidate.name}:${mod.tier}`);
const registerCalls = (on: unknown): void => void on;

spec(
  [
    "policy.mods.Install your organization's mods and set the order",
    "policy.mods.Install your organization's mods and set the order.enabledPlugins",
    "policy.mods.Install your organization's mods and set the order.extraKnownMarketplaces",
  ],
  () => {
    test("a mod from a remote marketplace counts as the organization's when managed settings enable it, declare its marketplace and list it (2.1.324)", () => {
      const remote = {
        ...installed('acme-remote', 'acme-hub'),
        managed: { isEnabled: true, isLoadedInPlace: false, isMarketplaceManaged: true },
      };
      const prepended = orderMods(
        managed({ prependPlugins: ['acme-remote@acme-hub', 'sec-default@builtin'] }),
        [installed('user-mod'), remote],
      );
      expect(names(prepended)).toEqual([
        'acme-remote:prepend',
        'cc-plugin-sec-default:prepend',
        'user-mod:user',
      ]);
      expect(prepended.notes).toEqual([]);
      const appended = orderMods(managed({ appendPlugins: ['acme-remote@acme-hub'] }), [
        remote,
        installed('user-mod'),
      ]);
      expect(names(appended)).toEqual([
        'cc-plugin-sec-default:prepend',
        'user-mod:user',
        'acme-remote:append',
      ]);
    });

    test("a remote marketplace's mod that no managed list names, or whose marketplace managed settings do not declare, counts as a user's, with the debug line", () => {
      const remote = {
        ...installed('acme-remote', 'acme-hub'),
        managed: { isEnabled: true, isLoadedInPlace: false, isMarketplaceManaged: true },
      };
      const unlisted = orderMods(managed(), [remote]);
      expect(names(unlisted)).toEqual(['cc-plugin-sec-default:prepend', 'acme-remote:user']);
      expect(unlisted.notes[0]).toStartWith(`${remote.id} is enabled by managed settings, but `);
      const undeclared = { ...remote, managed: { ...remote.managed, isMarketplaceManaged: false } };
      const order = orderMods(managed({ prependPlugins: ['acme-remote@acme-hub'] }), [undeclared]);
      expect(names(order)).toEqual(['acme-remote:user']);
      // a list in the user's settings does not make it the organization's
      const userList = orderMods(
        policy({ managed: {}, user: { prependPlugins: ['acme-remote@acme-hub'] } }),
        [remote],
      );
      expect(names(userList)).toEqual(['cc-plugin-sec-default:prepend', 'acme-remote:user']);
    });
  },
);

spec(
  [
    'policy.mods.Stop user-installed mods from loading.The guard refuses these mods',
    "policy.mods.Stop user-installed mods from loading.Your organization's mods still run",
  ],
  () => {
    test("the guard refuses a mod in a plugin only the user enabled; a remote marketplace's mod counts as the organization's only when managed settings list it", async () => {
      const remote = {
        ...installed('acme-remote', 'acme-hub'),
        managed: { isEnabled: true, isLoadedInPlace: false, isMarketplaceManaged: true },
      };
      const guardOn = {
        'cc-plugin-sec-default@builtin': { options: { allowManagedModsOnly: true } },
      };
      const listed = managed({
        pluginConfigs: guardOn,
        prependPlugins: ['sec-default@builtin', 'acme-remote@acme-hub'],
      });
      const unlisted = managed({ pluginConfigs: guardOn });
      const run = async (settings: ReturnType<typeof managed>) => {
        const { host } = session(
          { first: registerCalls, 'acme-remote': registerCalls },
          { policy: settings },
        );
        return host.load([
          modPlugin(installed('first'), registerCalls),
          modPlugin(remote, registerCalls),
        ]);
      };
      const kept = await run(listed);
      expect(kept.loaded.map((mod) => mod.name)).toEqual(['cc-plugin-sec-default', 'acme-remote']);
      const refused = await run(unlisted);
      expect(refused.loaded.map((mod) => mod.name)).toEqual(['cc-plugin-sec-default']);
      expect(refused.lines).toContain(
        "acme-remote: refused by cc-plugin-sec-default: mods are limited to your organization's by policy (allowManagedModsOnly)",
      );
    });
  },
);
