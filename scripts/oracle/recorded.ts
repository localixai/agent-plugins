/**
 * What Claude Code did on the fixtures Agent Plugins Extended wrote for it, as the spec records it
 * (`bun run oracle:record` in its `extend/tools/`) and `bun run spec:sync` copies into
 * `spec/oracle/`. This repository never runs Claude Code: it reads these outcomes and compares the
 * kit with them (AGENTS.md, E5a; docs/TESTING.md, "Oracle runs").
 */
import { readdirSync } from 'node:fs';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { z } from 'zod';
import { SPEC_DIR } from '../spec/source.ts';

export const ORACLE_DIR = join(SPEC_DIR, 'oracle');
/** One plugin per folder, each with the `expected.json` the session oracle recorded. */
export const RUNTIME_DIR = join(ORACLE_DIR, 'runtime');
export const MODEL_CONTENT_DIR = join(ORACLE_DIR, 'model-content');
/** The settings a runtime fixture's session runs under (the user layer). */
export const SETTINGS_FILE = 'oracle-settings.json';

const json = async (path: string): Promise<unknown> => JSON.parse(await readFile(path, 'utf8'));

/** The folders of a fixture oracle, sorted: one case each. */
export const caseFolders = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .toSorted();

/**
 * What a session registered from a plugin, ids only: the names that belong to it, sorted.
 * `outputStyle` is the session's: `init` echoes the `outputStyle` setting.
 */
export const registrySchema = z.object({
  loaded: z.boolean(),
  agents: z.array(z.string()),
  slashCommands: z.array(z.string()),
  skills: z.array(z.string()),
  mcpServers: z.array(z.string()),
  outputStyle: z.string(),
});
export type Registry = z.infer<typeof registrySchema>;

/** The registry recorded for a runtime fixture, or undefined when none is. */
export async function readExpected(dir: string): Promise<Registry | undefined> {
  return registrySchema.safeParse(await json(join(dir, 'expected.json')).catch(() => undefined))
    .data;
}

const release = { claudeCodeVersion: z.string() };

const scrubSchema = z.object({
  ...release,
  /** What the hook received for each candidate; `null` when the scrub removed it. */
  left: z.record(z.string(), z.string().nullable()),
});
const scrubDir = join(ORACLE_DIR, 'subprocess-scrub');
export const readScrubCandidates = async () =>
  z
    .array(z.object({ name: z.string(), value: z.string() }))
    .parse(await json(join(scrubDir, 'candidates.json')));
export const readScrubExpected = async () =>
  scrubSchema.parse(await json(join(scrubDir, 'expected.json')));

/** The remote MCP credential names Claude Code reads as empty, and those it expands. */
export const readRemoteMcpEnv = async () =>
  z
    .object({ ...release, empty: z.array(z.string()), kept: z.array(z.string()) })
    .parse(await json(join(ORACLE_DIR, 'remote-mcp-env', 'expected.json')));

/** The variables a supplied `headersHelper` runs without, and those it receives. */
export const readHelperEnv = async () =>
  z
    .object({ ...release, removed: z.array(z.string()), kept: z.array(z.string()) })
    .parse(await json(join(ORACLE_DIR, 'headers-helper-env', 'expected.json')));

const modelCaseSchema = z.object({
  prompt: z.string(),
  skill: z.string(),
  arguments: z.string(),
  settings: z.unknown().optional(),
});
export type ModelContentCase = z.infer<typeof modelCaseSchema>;
export const readModelCase = async (name: string): Promise<ModelContentCase> =>
  modelCaseSchema.parse(await json(join(MODEL_CONTENT_DIR, name, 'case.json')));
/** The user text blocks of the first request Claude Code sent, normalized. */
export const readModelExpected = async (name: string) =>
  z
    .object({ ...release, blocks: z.array(z.string()) })
    .parse(await json(join(MODEL_CONTENT_DIR, name, 'expected.json')));

const settingsCaseSchema = z.object({
  name: z.string(),
  user: z.unknown().optional(),
  project: z.unknown().optional(),
  local: z.unknown().optional(),
  /** Settings a parent process hands down: the kit's `parent` scope. */
  parent: z.unknown().optional(),
  keys: z.array(z.string()),
  /** The Agent SDK resolver's merged value per key, canonical (keys sorted, lists as sorted sets). */
  sdk: z.record(z.string(), z.unknown()),
  /** The mode `claude -p` started in, for a `defaultMode` case. */
  cli: z.string().optional(),
});
export type SettingsCase = z.infer<typeof settingsCaseSchema>;

/** The settings oracle: the cases, and the keys the recording machine's managed settings set. */
export const readSettingsRecord = async () =>
  z
    .object({
      ...release,
      sdkVersion: z.string(),
      managed: z.object({ present: z.boolean(), keys: z.array(z.string()) }),
      cases: z.array(settingsCaseSchema),
    })
    .parse(await json(join(ORACLE_DIR, 'settings.json')));
