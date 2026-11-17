import { loadCorePackage } from '../loading/core/load-core.ts';
import type { Plugin } from '../loading/plugin-model.ts';
import type { PluginFiles } from '../ports/index.ts';
import { reported, stop } from './notes.ts';
import { componentNotes } from './to-core/components.ts';
import type { ConversionNote } from './types.ts';

/**
 * What each target makes of a conversion's output before it is written (Agent Plugins Extended
 * §18.4.1 items 1 and 3), over the folder as it would be (`overlayFiles`).
 */

/**
 * A core package converted to Claude Code: the Claude Code locations the core never read
 * (`commands/`, `agents/`, `settings.json`, …, §18.1.1 item 3) now load in Claude Code, a behaviour
 * the core package did not have; each is reported.
 */
export const claudeCodeExtras = (plugin: Plugin | undefined): ConversionNote[] =>
  plugin === undefined
    ? []
    : componentNotes(plugin, false, {
        note: reported,
        says: 'was an ordinary file of the core package and now loads in Claude Code',
      });

/**
 * A Claude Code plugin converted to the core: the written files must load (core §5–§7): a manifest
 * that rejects the package, or a server or handler the loader skips, stops the conversion. A skill
 * the core skips is already reported, and its file is not written by the conversion.
 */
export async function coreOutputProblems(
  overlay: PluginFiles,
  signal?: AbortSignal,
): Promise<readonly ConversionNote[]> {
  const loaded = await loadCorePackage({ files: overlay, ...(signal && { signal }) });
  return loaded.issues
    .filter((found) => found.severity === 'error' && !found.path.startsWith('skills/'))
    .map((found) =>
      stop(found.path, undefined, `the core loader refuses the output: ${found.message}`),
    );
}
