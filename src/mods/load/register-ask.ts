import type { PluginManifest } from '../../format/manifest-schema.ts';
import type { LoaderEnv, ModPlugin } from './loader.ts';
import type { OrderedMod } from './order.ts';
import type { LoadOutcome, Prepared } from './prepare.ts';
import { askToLoad, type RegisterVerdict } from './register-call.ts';
import { registerEvent } from './register-event.ts';
import type { ModUses } from './scan/uses.ts';

/**
 * Asking the mods loaded so far about a plugin (Agent Plugins Extended §10.8.1, §10.19.5): a
 * refusal keeps the whole plugin out (2.1.326), and a hook that removes `modules` from `e.provides`
 * keeps only the hooks module out, with the line `<plugin>: hooks module left out by <mod>`. What
 * the mods kept out of a plugin is the host's to honour: it goes to the load report (`keptOut`).
 */

/** One plugin the mods kept a part of, or all of, out: `verdict` says which. */
export interface PluginKeptOut {
  /** The plugin's id, `name@marketplace`. */
  readonly plugin: string;
  readonly verdict: RegisterVerdict;
}

/** What the loader read of the plugin: its module, prepared, or the manifest of a plugin with none. */
type Subject =
  | { readonly prepared: Prepared; readonly uses: ModUses }
  | ({ readonly manifest: PluginManifest } & Pick<ModPlugin, 'register'>);

const keptAnything = (verdict: RegisterVerdict): boolean =>
  !verdict.loads || Object.keys(verdict.leftOut).length > 0;

/**
 * Fires `plugin.register` for `mod`; the outcome that stops the module from loading (a refusal,
 * or `modules` left out), or `undefined` when it loads.
 */
export async function askPlugin(
  env: LoaderEnv,
  mod: OrderedMod,
  subject: Subject,
): Promise<LoadOutcome | undefined> {
  const { name, id } = mod.candidate;
  const { prepared, manifest, facts } = subjectParts(subject);
  const event = registerEvent(
    {
      name,
      id,
      tier: mod.tier,
      root: env.pluginRoot(id),
      ...(manifest.version === undefined ? {} : { version: manifest.version }),
      ...(prepared === undefined ? {} : { module: prepared.entry }),
      ...('uses' in subject ? { uses: subject.uses } : {}),
    },
    facts,
  );
  const verdict = await askToLoad(env, event);
  if (keptAnything(verdict)) env.keptOut.push({ plugin: id, verdict });
  return outcomeOf(name, verdict);
}

/** The module prepared (when the plugin has one), the manifest and the host's facts of a subject. */
function subjectParts(subject: Subject): {
  readonly prepared: Prepared | undefined;
  readonly manifest: PluginManifest;
  readonly facts: ModPlugin['register'];
} {
  if ('prepared' in subject) {
    const { prepared } = subject;
    return { prepared, manifest: prepared.manifest, facts: prepared.register };
  }
  return { prepared: undefined, manifest: subject.manifest, facts: subject.register };
}

/** The outcome that stops the module from loading, or `undefined` when it loads. */
function outcomeOf(name: string, verdict: RegisterVerdict): LoadOutcome | undefined {
  if (!verdict.loads) {
    return { line: `${name}: refused by ${verdict.by}: ${verdict.reason}`, kind: 'refused-by-mod' };
  }
  if (verdict.leftOut.modules === undefined) return undefined;
  return {
    line: `${name}: hooks module left out by ${verdict.by ?? 'a mod'}`,
    kind: 'refused-by-mod',
  };
}
