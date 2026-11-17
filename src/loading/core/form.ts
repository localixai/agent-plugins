import { isCoreManifestValue } from '../../format/core/versions.ts';
import { STANDARD_LAYOUT } from '../../format/layout.ts';
import { leavesFolder } from '../../format/path-text.ts';
import { readJson } from '../../format/read-json.ts';
import type { PluginFiles } from '../../ports/index.ts';

/**
 * The form of a plugin root, decided before any component is read (Agent Plugins Extended §1.3):
 *
 * | `.claude-plugin/plugin.json` | root `plugin.json`            | form                          |
 * | ---------------------------- | ----------------------------- | ----------------------------- |
 * | present                      | absent, or no core `$schema`  | Claude Code plugin            |
 * | present                      | with a core `$schema`         | Claude Code plugin, dual      |
 * | absent                       | with a core `$schema`         | core package                  |
 * | absent                       | absent, or no core `$schema`  | Claude Code plugin, no manifest |
 *
 * A root `plugin.json` whose `$schema` is no canonical core identifier, that is not JSON or not an
 * object, is an ordinary file (J1.1). A dual package loads as a Claude Code plugin only (§1.3.1,
 * J1.2); `dual` says that it also carries a core manifest, which a host shows in its details.
 *
 * Judgment (spec/judgments.md): a root `plugin.json` that is a link leading outside the root is not
 * read, so it is not a core manifest: the root is then a Claude Code plugin, which Claude Code would
 * load the same way since it never reads that file.
 */

export const PLUGIN_FORMS = ['claude-code', 'core'] as const;
export type PluginForm = (typeof PLUGIN_FORMS)[number];

export interface PluginFormDecision {
  readonly form: PluginForm;
  /** The root carries both manifests (a dual package); its form is then `claude-code`. */
  readonly dual: boolean;
}

const CORE_MANIFEST = 'plugin.json';

/** Whether the root holds a core manifest: a contained regular file with a canonical core `$schema`. */
export async function hasCoreManifest(files: PluginFiles, signal?: AbortSignal): Promise<boolean> {
  const stat = await files.stat(CORE_MANIFEST, signal);
  if (stat?.isFile !== true || stat.checkFailed !== undefined) return false;
  if (stat.isSymlink && stat.realPath !== undefined && leavesFolder(stat.realPath, 'posix'))
    return false;
  const read = await readJson(files, CORE_MANIFEST, signal);
  return 'value' in read && isCoreManifestValue(read.value);
}

/** Decides the form of the plugin root `files` reads (§1.3). */
export async function detectPluginForm(input: {
  readonly files: PluginFiles;
  readonly signal?: AbortSignal | undefined;
}): Promise<PluginFormDecision> {
  const { files, signal } = input;
  const [claude, core] = await Promise.all([
    files.exists(STANDARD_LAYOUT.manifest.location, signal),
    hasCoreManifest(files, signal),
  ]);
  if (claude) return { form: 'claude-code', dual: core };
  return { form: core ? 'core' : 'claude-code', dual: false };
}
