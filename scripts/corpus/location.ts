/**
 * Where the corpus lives. Agent Plugins Extended fetches it and asks Claude Code about every plugin
 * (`bun run corpus:fetch` and `bun run corpus:oracle` in its `extend/tools/`); this repository only
 * reads what that left: `CORPUS_DIR` when it is set, else the spec checkout's `extend/.cache/corpus`
 * beside this repository. The comparisons write their reports here, under `.cache/corpus/`
 * (never committed; docs/TESTING.md, "Corpus").
 */
import { existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { z } from 'zod';
import { ROOT } from '../docs/location.ts';

export const CORPUS_DIR =
  process.env['CORPUS_DIR'] ??
  join(ROOT, '..', 'agent-plugins-extended', 'extend', '.cache', 'corpus');
const PLUGINS_DIR = join(CORPUS_DIR, 'plugins');
const INDEX_FILE = join(CORPUS_DIR, 'index.json');

/** Where the comparisons write what differs. */
export const REPORTS_DIR = join(ROOT, '.cache', 'corpus');
export const REPORT_FILE = join(REPORTS_DIR, 'report.json');

/** The marketplaces the spec's corpus clones. */
const MARKETPLACES = ['claude-plugins-official', 'claude-plugins-community'] as const;

/** The step of the spec that fills what a run found missing. */
export const FILL = 'run it in agent-plugins-extended/extend/tools (docs/TESTING.md, "Corpus")';

/** One plugin of the corpus, as `corpus:fetch` left it. */
const corpusPluginSchema = z.object({
  marketplace: z.string(),
  name: z.string(),
  /** The plugin root, relative to the corpus folder (so the checkout can move), when it was fetched. */
  dir: z.string().optional(),
  planKind: z.string(),
  /** Why the plugin is not in the corpus: a refused plan, an unsupported kind or a failed fetch. */
  skipped: z.string().optional(),
});

/** One fetched plugin, with the absolute path of its root on this machine. */
export interface CorpusTarget {
  /** `marketplace/name`. */
  readonly key: string;
  readonly name: string;
  readonly dir: string;
}

/**
 * A root `corpus:fetch` saw on disk that is gone: a plugin it cloned, or one inside a marketplace
 * clone that is itself gone. (A marketplace may list a local path it does not have; that one is
 * compared as it is.)
 */
const isLost = (dir: string): boolean =>
  !existsSync(dir) &&
  (dir.startsWith(`${PLUGINS_DIR}/`) ||
    !MARKETPLACES.some((name) => existsSync(join(CORPUS_DIR, name))));

/**
 * The fetched plugins of the index, in its order. A lost root fails the run: an oracle and a kit
 * that both say "no such directory" would otherwise agree on every plugin.
 */
export async function readCorpusTargets(): Promise<CorpusTarget[]> {
  if (!existsSync(INDEX_FILE))
    throw new Error(`${INDEX_FILE} is missing: \`bun run corpus:fetch\`, ${FILL}`);
  const index = z.array(corpusPluginSchema).parse(await Bun.file(INDEX_FILE).json());
  const targets = index.flatMap((plugin) =>
    plugin.dir === undefined
      ? []
      : [
          {
            key: `${plugin.marketplace}/${plugin.name}`,
            name: plugin.name,
            dir: resolve(CORPUS_DIR, plugin.dir),
          },
        ],
  );
  const missing = targets.filter((target) => isLost(target.dir));
  if (missing.length > 0) {
    const first = missing[0]?.dir ?? '';
    throw new Error(
      `${missing.length} of ${targets.length} corpus plugins are not on disk (first: ${first}); \`bun run corpus:fetch\`, ${FILL}`,
    );
  }
  return targets;
}
