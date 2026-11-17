/**
 * The spec gate (docs/SPEC.md), offline and deterministic:
 *
 * - SP1 `spec/requirements.json` parses, and its ids are unique and sorted.
 * - SP2 every id a test names in `spec(...)` is a requirement.
 * - SP3 the installed `@anthropic-ai/claude-agent-sdk` describes the release the synced spec
 *   describes (SDK `0.3.N` ↔ Claude Code `2.1.N`): the types and the spec never disagree.
 * - SP4 `spec/SOURCE.json` pins Agent Plugins Extended at one commit, lists exactly the synced
 *   files (the folders' too, file by file), and every one of them is byte for byte what it
 *   recorded; the snapshots name its release.
 * - SP5 the snapshots the tests read parse (`docs-tables.json`, `mods-declarations.json`,
 *   `tools-table.json`), and `tests/support/tool-table.ts`, the tools table the tests read, is the
 *   spec's `tools-table.json`: the same tools, "Permission required" and rule format.
 */
import { join, relative } from 'node:path';
import { z } from 'zod';
import { ROOT } from '../docs/location.ts';
import { declarationsSchema } from './declarations.ts';
import { snapshotSchema } from './docs-tables.ts';
import { readRequirements } from './requirements.ts';
import { NAMING_RELEASE, SPEC_DIR, readSource, releaseIn } from './source.ts';
import { syncedOnDisk } from './synced-files.ts';
import { testIds } from './test-ids.ts';
import { toolTableDrift, toolsTableSchema } from './tool-table.ts';

const SYNC = 'run `bun run spec:sync` (docs/SPEC.md, "Syncing the spec")';
const problems: string[] = [];

const file = await readRequirements().catch((error: unknown) => {
  problems.push(`SP1 spec/requirements.json: ${String(error)}`);
  return undefined;
});
const ids = file?.requirements.map((r) => r.id) ?? [];
const known = new Set(ids);
if (known.size !== ids.length) problems.push('SP1 spec/requirements.json has duplicate ids');
if (ids.some((id, index) => index > 0 && (ids[index - 1] ?? '').localeCompare(id) > 0)) {
  problems.push('SP1 spec/requirements.json is not sorted');
}

for await (const path of new Bun.Glob('tests/**/*.ts').scan({ cwd: ROOT })) {
  const text = await Bun.file(join(ROOT, path)).text();
  for (const id of testIds(text)) {
    if (!known.has(id)) problems.push(`SP2 ${relative(ROOT, path)}: "${id}" is not a requirement`);
  }
}

const source = await readSource().catch((error: unknown) => {
  problems.push(`SP4 spec/SOURCE.json: ${String(error)}: ${SYNC}`);
  return undefined;
});

const patch = (version: string): string => version.split('.').at(-1) ?? '';
const sdk = z
  .object({ version: z.string() })
  .parse(
    await Bun.file(join(ROOT, 'node_modules/@anthropic-ai/claude-agent-sdk/package.json')).json(),
  );
if (source && patch(sdk.version) !== patch(source.release)) {
  problems.push(
    `SP3 the Agent SDK ${sdk.version} and the spec (Claude Code ${source.release}) describe ` +
      'different releases: update both together (docs/SPEC.md, "Moving to a new spec revision")',
  );
}

if (source) {
  const onDisk = await syncedOnDisk();
  const recorded = Object.keys(source.files).toSorted();
  const present = [...onDisk.keys()].toSorted();
  for (const name of recorded.filter((one) => !onDisk.has(one))) {
    problems.push(`SP4 spec/${name} is missing: ${SYNC}`);
  }
  for (const name of present.filter((one) => source.files[one] === undefined)) {
    problems.push(
      `SP4 spec/${name} is not a file of the spec's at ${source.commit.slice(0, 7)}: ${SYNC}`,
    );
  }
  for (const [name, bytes] of onDisk) {
    if (source.files[name] === undefined) continue;
    if (bytes.hash !== source.files[name]) {
      problems.push(`SP4 spec/${name} is not the spec's at ${source.commit.slice(0, 7)}: ${SYNC}`);
    } else if (
      NAMING_RELEASE.some((one) => one === name) &&
      releaseIn(bytes.bytes) !== source.release
    ) {
      problems.push(`SP4 spec/${name} names another release than ${source.release}: ${SYNC}`);
    }
  }
}

const parses = async (name: string, schema: z.ZodType): Promise<void> => {
  const parsed = schema.safeParse(
    await Bun.file(join(SPEC_DIR, name))
      .json()
      .catch(() => undefined),
  );
  if (!parsed.success) problems.push(`SP5 spec/${name} does not parse: ${parsed.error.message}`);
};
await parses('docs-tables.json', snapshotSchema);
await parses('mods-declarations.json', declarationsSchema);
await parses('tools-table.json', toolsTableSchema);
for (const drift of await toolTableDrift()) problems.push(`SP5 ${drift}`);

for (const problem of problems) console.error(problem);
process.exit(problems.length > 0 ? 1 : 0);
