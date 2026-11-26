import type { HooksSettings } from '../protocol/hook-config.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HookSource, RegisteredHandler } from './contracts.ts';

const call = coreModule('hooks/register');

/** Where a hooks file was declared: what every entry made from it carries. */
export interface HookDeclaration {
  /** Prefix of the entry ids; two declarations of one registry never share one. */
  readonly idPrefix: string;
  readonly source: HookSource;
  /** The full `name@marketplace` id of the plugin, for a plugin's hooks. */
  readonly pluginId?: string;
}

/**
 * A hooks file, in the shape of the `hooks` key of settings (hooks.md, "Configuration"; a plugin's
 * `hooks/hooks.json` wraps it the same way), as the handlers a registry holds: one entry per
 * handler, in file order, with the matcher of its group and where it was declared. The handler
 * stays the official object. A key that names no event of the official union registers nothing:
 * no input is ever built for it.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/register.rs`, docs/RUST.md).
 */
export const registerHooks = (
  hooks: HooksSettings,
  declaration: HookDeclaration,
): readonly RegisteredHandler[] => call('registerHooks', hooks, declaration);
