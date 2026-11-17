import { SIDELOAD_CLOSING } from '../../settings/managed/sideload.ts';
import type { FlagFacts, FlagName } from './types.ts';

/**
 * What managed `disableSideloadFlags` rejects (plugins/org.md, control matrix: "`--plugin-dir`,
 * `--plugin-url`, `--agents`, the Agent SDK `plugins` option, and folders named in the
 * `CLAUDE_CODE_PLUGIN_DIRS` variable at startup, and `--mcp-config` too unless every server in it is
 * an in-process `type: "sdk"` entry"; plugins/cli-reference.md, "Flags that load a plugin for one
 * session": Claude Code "prints that the flag is disabled by your organization's managed settings
 * and exits `1` without starting"). The wording follows `claude --plugin-dir ./x`'s "message naming
 * `disableSideloadFlags`" (plugins/org.md) and is the one `mods/load/plugin-dirs.ts` prints.
 */
export const SIDELOAD_FLAGS: readonly FlagName[] = [
  '--plugin-dir',
  '--plugin-url',
  '--agents',
  '--mcp-config',
];

export const sideloadMessage = (name: string): string =>
  `${name} is disabled by your organization's managed settings (disableSideloadFlags)`;

/** The refusal for `flag`, or `undefined` when managed settings allow it. */
export const sideloadRefusal = (flag: FlagName, facts: FlagFacts): string | undefined =>
  facts.sideloadDisabled && SIDELOAD_FLAGS.includes(flag) ? sideloadMessage(flag) : undefined;

/** The refusal for the folders `CLAUDE_CODE_PLUGIN_DIRS` names, when it names any: it says to unset it (J14.8). */
export function pluginDirsVariableRefusal(facts: FlagFacts): string | undefined {
  const named = (facts.env.CLAUDE_CODE_PLUGIN_DIRS ?? '').trim();
  return facts.sideloadDisabled && named !== ''
    ? `${sideloadMessage('CLAUDE_CODE_PLUGIN_DIRS')}: unset the variable`
    : undefined;
}

const SIDELOAD_MARK = '(disableSideloadFlags)';

/**
 * The refusals with the docs' closing line after the rejected flags, once, when any was rejected
 * (plugins/troubleshooting.md, "`--plugin-dir is disabled …`"), as `sideloadVerdict` words it.
 */
export function withSideloadClosing(refusals: readonly string[]): readonly string[] {
  const sideloaded = refusals.some((line) => line.includes(SIDELOAD_MARK));
  return sideloaded ? [...refusals, SIDELOAD_CLOSING] : refusals;
}
