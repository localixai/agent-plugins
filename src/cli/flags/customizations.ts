import { coreModule } from '../../wasm/bridge.ts';
import type { Customization } from './types.ts';

const call = coreModule('cli/flags/customizations');

/**
 * The customizations a mode leaves out (cli-reference.md, `--bare`, `--safe-mode`,
 * `--disable-slash-commands`; headless.md, "Start faster with bare mode"; env-vars.md,
 * `CLAUDE_CODE_SIMPLE` and `CLAUDE_CODE_SAFE_MODE`). `installed-plugins` is the plugins installed
 * from marketplaces: a plugin loaded by `--plugin-dir` or `--plugin-url` still loads in bare mode
 * ("A plugin: `--plugin-dir <path>`, `--plugin-url <url>`" is what to pass to load one). The docs
 * name the eight of `--bare` and the fifteen of `--safe-mode`; whatever neither names stays loaded
 * (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/customizations.rs`, docs/RUST.md).
 */
export const SKIP_MODES: readonly ['bare', 'safe-mode', 'disable-slash-commands'] =
  call('SKIP_MODES');
export type SkipMode = (typeof SKIP_MODES)[number];

/** The customizations `mode` skips, in the order of `CUSTOMIZATIONS`. */
export const skippedBy = (mode: SkipMode): readonly Customization[] => call('skippedBy', mode);

/**
 * What `--safe-mode` does with the managed layer of a customization it skips: "Managed settings
 * policy still applies, including policy-configured hooks, status line, and file-suggestion
 * commands; managed plugins, managed skills, managed CLAUDE.md, and policy-configured MCP servers do
 * not." The docs say nothing of the other kinds, so they are absent.
 */
export const MANAGED_UNDER_SAFE_MODE: { readonly [C in Customization]?: 'applies' | 'skipped' } =
  call('MANAGED_UNDER_SAFE_MODE');
