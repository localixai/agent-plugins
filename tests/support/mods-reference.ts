import { snapshotSchema } from '../../scripts/spec/docs-tables.ts';
import tables from '../../spec/docs-tables.json' with { type: 'json' };

/**
 * The tables of plugins/mods/reference.md the mods tests compare the kit with: the spec's
 * `docs-tables.json`, synced into `spec/` (`bun run spec:sync`; `check:spec` checks its hash).
 */
export const MODS_REFERENCE = snapshotSchema.parse(tables);
