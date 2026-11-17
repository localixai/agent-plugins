import { join } from 'node:path';
import { z } from 'zod';
import { ROOT } from '../docs/location.ts';

/**
 * The requirements of Agent Plugins Extended at the pinned revision, synced into `spec/` by `bun run
 * spec:sync` (docs/SPEC.md). Never edited here: `check:spec` (SP4) fails on a hand edit.
 */
export const REQUIREMENTS_FILE = join(ROOT, 'spec', 'requirements.json');

/** The categories of the requirements, in the order the compatibility report lists them. */
export const CATEGORIES = [
  'manifest',
  'layout',
  'environment',
  'user-config',
  'marketplace',
  'dependencies',
  'settings',
  'skills',
  'commands',
  'agents',
  'output-styles',
  'hooks',
  'permissions',
  'mcp',
  'lsp',
  'monitors',
  'channels',
  'tools',
  'mods',
  'install',
  'updates',
  'trust',
  'policy',
  'settings-merge',
  'mcp-settings',
  'themes',
  'workflows',
  'cli',
] as const;
export type Category = (typeof CATEGORIES)[number];

const requirementSchema = z.strictObject({
  /** `<category path>.<key>`, such as `hooks.handler.common.timeout`. */
  id: z.string().min(1),
  category: z.enum(CATEGORIES),
  /** The Claude Code docs page the spec cites, relative to https://code.claude.com/docs/en/. */
  page: z.string().min(1),
  /** The heading the requirement sits under. */
  section: z.string(),
});
export type Requirement = z.infer<typeof requirementSchema>;

const fileSchema = z.strictObject({
  /** The Claude Code release the spec's requirements describe. */
  claudeCode: z.string(),
  requirements: z.array(requirementSchema),
});
export type RequirementsFile = z.infer<typeof fileSchema>;

export async function readRequirements(): Promise<RequirementsFile> {
  return fileSchema.parse(await Bun.file(REQUIREMENTS_FILE).json());
}
