/**
 * Where `spec/` comes from: Agent Plugins Extended (https://github.com/localixai/agent-plugins-extended)
 * at one commit. `spec/SOURCE.json` records the repository, the commit, the Claude Code release the
 * spec's snapshots describe and the SHA-256 of every synced file; `bun run spec:sync` writes it and
 * `check:spec` (SP4) holds the files to it, so a hand edit or a stale copy fails (docs/SPEC.md).
 */
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { z } from 'zod';
import { ROOT } from '../docs/location.ts';

export const SPEC_DIR = join(ROOT, 'spec');
export const SOURCE_FILE = join(SPEC_DIR, 'SOURCE.json');

/** The spec's repository, and its checkout beside this one when there is one. */
export const SPEC_REPOSITORY = 'https://github.com/localixai/agent-plugins-extended';
export const SPEC_SLUG = 'localixai/agent-plugins-extended';
export const SIBLING_CHECKOUT = join(ROOT, '..', 'agent-plugins-extended');

/** Where the synced files sit in the spec's repository. */
export const SPEC_FOLDER = 'extend/conformance';

/** The files synced into `spec/`, by name, from `extend/conformance/` of the spec. */
export const SYNCED = [
  'requirements.json',
  'mods-declarations.json',
  'docs-tables.json',
  'out-of-scope.json',
  'out-of-scope-pages.json',
  'scope-audit.md',
] as const;

/** The synced files that name the Claude Code release they describe (`claudeCode`). */
export const NAMING_RELEASE = ['requirements.json', 'mods-declarations.json'] as const;

const sha256 = z.string().regex(/^[0-9a-f]{64}$/);

export const sourceSchema = z.strictObject({
  repository: z.literal(SPEC_REPOSITORY),
  /** The full commit of the spec the files were copied from. */
  commit: z.string().regex(/^[0-9a-f]{40}$/),
  /** The Claude Code release the spec's snapshots describe (`claudeCode` in each of them). */
  release: z.string().regex(/^\d+\.\d+\.\d+$/),
  files: z.record(z.string(), sha256),
});
export type Source = z.infer<typeof sourceSchema>;

export const hashOf = (bytes: Uint8Array): string =>
  createHash('sha256').update(bytes).digest('hex');

export async function readSource(): Promise<Source> {
  return sourceSchema.parse(await Bun.file(SOURCE_FILE).json());
}

/** The Claude Code release a snapshot names, or undefined when it names none. */
export function releaseIn(bytes: Uint8Array): string | undefined {
  const parsed = z
    .object({ claudeCode: z.string() })
    .safeParse(JSON.parse(new TextDecoder().decode(bytes)));
  return parsed.success ? parsed.data.claudeCode : undefined;
}
