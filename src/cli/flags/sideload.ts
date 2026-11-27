/**
 * What managed `disableSideloadFlags` rejects (plugins/org.md, control matrix: "`--plugin-dir`,
 * `--plugin-url`, `--agents`, the Agent SDK `plugins` option, and folders named in the
 * `CLAUDE_CODE_PLUGIN_DIRS` variable at startup, and `--mcp-config` too unless every server in it is
 * an in-process `type: "sdk"` entry"; plugins/cli-reference.md, "Flags that load a plugin for one
 * session": Claude Code "prints that the flag is disabled by your organization's managed settings
 * and exits `1` without starting"). The wording follows `claude --plugin-dir ./x`'s "message naming
 * `disableSideloadFlags`" (plugins/org.md) and is the one `mods/load/plugin-dirs.ts` prints.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/sideload.rs`, docs/RUST.md).
 */
import type { FlagFacts, FlagName } from './types.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('cli/flags/sideload');

export const SIDELOAD_FLAGS: readonly FlagName[] = call('SIDELOAD_FLAGS');

export const sideloadMessage = (name: string): string => call('sideloadMessage', name);

/** The refusal for the folders `CLAUDE_CODE_PLUGIN_DIRS` names, when it names any: it says to unset it (J14.8). */
export const pluginDirsVariableRefusal = (facts: FlagFacts): string | undefined =>
  call('pluginDirsVariableRefusal', facts);

/**
 * The refusals with the docs' closing line after the rejected flags, once, when any was rejected
 * (plugins/troubleshooting.md, "`--plugin-dir is disabled …`"), as `sideloadVerdict` words it.
 */
export const withSideloadClosing = (refusals: readonly string[]): readonly string[] =>
  call('withSideloadClosing', refusals);
