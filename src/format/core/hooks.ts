import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../issues.ts';
import type { CoreExtensions } from './extensions.ts';
import type { CoreVersion } from './versions.ts';

/**
 * A core package's `hooks.json` (core §7.3.1, §7.3.2): the outer object disables the portable hooks
 * component when it fails (item 2); a member whose event the core does not define, or whose value is
 * not a nonempty array, is skipped (item 3); a handler that is not a closed `{ command, args?,
 * extensions? }` with a valid command token is skipped (item 4). Each skip is reported; the rest
 * loads. The schema is `schemas/1.1.0/hooks.schema.json` (`hooks`, `#/$defs/handlers`, `handler`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/hooks.rs`, docs/RUST.md).
 */

const call = coreModule('format/core/hooks');

/** The five events of the core (core §7.3.4–§7.3.8, §11.1 item 6). */
export const CORE_HOOK_EVENTS: readonly [
  'SessionStart',
  'UserPromptSubmit',
  'PreToolUse',
  'PostToolUse',
  'Stop',
] = call('CORE_HOOK_EVENTS');
export type CoreHookEvent = (typeof CORE_HOOK_EVENTS)[number];

export interface CoreHookHandler {
  readonly command: string;
  readonly args?: readonly string[];
  readonly extensions?: CoreExtensions;
}

export type CoreHooks = { readonly [E in CoreHookEvent]?: readonly CoreHookHandler[] };

/** Where each kept handler sits in its event's array of the file: `positions.<Event>[i]` is the index of `hooks.<Event>[i]`. */
export type CoreHookPositions = { readonly [E in CoreHookEvent]?: readonly number[] };

export interface CoreHooksResult {
  readonly hooks: CoreHooks;
  /** The file index of each kept handler, so a later issue names the handler it means. */
  readonly positions: CoreHookPositions;
  /** The portable hooks component is disabled: the outer object failed. */
  readonly disabled: boolean;
  readonly issues: readonly PluginIssue[];
}

/** Reads a parsed `hooks.json` against the version `plugin.json` targets. */
export const readCoreHooks = (
  raw: unknown,
  manifestVersion: CoreVersion,
  implemented: readonly string[] = [],
): CoreHooksResult => call('readCoreHooks', raw, manifestVersion, implemented);
