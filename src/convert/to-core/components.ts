import { STANDARD_LAYOUT } from '../../format/layout.ts';
import type { Plugin } from '../../loading/plugin-model.ts';
import { notConverted } from '../notes.ts';
import { CLAUDE_MANIFEST } from '../to-claude-code/manifest.ts';
import type { ConversionNote } from '../types.ts';

/**
 * What of a Claude Code plugin has no core counterpart (Agent Plugins Extended §18.4.5): commands,
 * agents, output styles, themes, LSP servers, monitors, workflows, `bin/`, `settings.json` and mods.
 * Each is reported with its path; the files stay where they are, for Claude Code (§18.4.1 item 4).
 */

/** How each note is made: its outcome and what it says after naming the component. */
export interface ComponentWording {
  readonly note: typeof notConverted;
  readonly says: string;
}

const NOT_IN_CORE: ComponentWording = { note: notConverted, says: 'has no core counterpart' };

/**
 * One note per component; `mods`: the plugin's hooks file declares `modules`, which `Plugin` does not
 * carry.
 */
export function componentNotes(
  plugin: Plugin,
  mods: boolean,
  wording: ComponentWording = NOT_IN_CORE,
): ConversionNote[] {
  const { note, says } = wording;
  const files = (found: readonly { readonly path: string | undefined }[], what: string) =>
    found.map((one) =>
      one.path === undefined
        ? note(CLAUDE_MANIFEST, 'commands', `${what} ${says}`)
        : note(one.path, undefined, `${what} ${says}`),
    );
  const named = (names: readonly string[], path: string, what: string) =>
    names.map((name) => note(path, name, `${what} "${name}" ${says}`));
  const settings = Object.keys(plugin.settings).length > 0;
  return [
    ...files(plugin.commands, 'a command'),
    ...files(plugin.agents, 'an agent'),
    ...files(plugin.outputStyles, 'an output style'),
    ...files(plugin.themes, 'a theme'),
    ...files(plugin.workflows, 'a workflow'),
    ...named(Object.keys(plugin.lspServers), STANDARD_LAYOUT.lspServers.location, 'the LSP server'),
    ...named(
      plugin.monitors.map((monitor) => monitor.name),
      STANDARD_LAYOUT.monitors.location,
      'the monitor',
    ),
    ...(plugin.hasBin ? [note('bin/', undefined, `\`bin/\` ${says}`)] : []),
    ...(settings
      ? [note(STANDARD_LAYOUT.settings.location, undefined, `plugin settings ${says}`)]
      : []),
    ...(mods ? [note(STANDARD_LAYOUT.hooks.location, 'modules', `a mod ${says}`)] : []),
  ];
}
