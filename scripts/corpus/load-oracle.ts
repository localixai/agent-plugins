/**
 * The loading verdicts: what Claude Code itself loads from each corpus plugin, as `claude
 * --plugin-dir <dir> plugin details <name>` listed it, recorded by Agent Plugins Extended (`bun run
 * corpus:oracle` in its `extend/tools/`) under `<corpus>/oracle-load/<release>/`. This repository
 * reads them and never runs Claude Code for them.
 */
import { join } from 'node:path';
import { z } from 'zod';
import { CORPUS_DIR } from './location.ts';

/** What `plugin details` lists: names per component, hook events, or absence when it did not load. */
export interface LoadInventory {
  readonly loaded: boolean;
  readonly skills: readonly string[];
  readonly agents: readonly string[];
  readonly hookEvents: readonly string[];
  readonly mcpServers: readonly string[];
  readonly lspServers: readonly string[];
}

const inventorySchema = z.object({
  loaded: z.boolean(),
  skills: z.array(z.string()),
  agents: z.array(z.string()),
  hookEvents: z.array(z.string()),
  mcpServers: z.array(z.string()),
  lspServers: z.array(z.string()),
});

/** The recorded inventory of one plugin by its corpus key, or undefined when none was recorded. */
export async function loadOracle(release: string, key: string): Promise<LoadInventory | undefined> {
  const file = Bun.file(
    join(CORPUS_DIR, 'oracle-load', release, `${key.replaceAll('/', '__')}.json`),
  );
  const cached = inventorySchema.safeParse(await file.json().catch(() => undefined));
  return cached.success ? cached.data : undefined;
}
