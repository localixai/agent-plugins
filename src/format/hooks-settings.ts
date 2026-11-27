import type { HooksSettings } from '../protocol/hook-config.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { PluginIssue } from './issues.ts';

const call = coreModule('format/hooks-settings');

/**
 * Reading hook configuration (hooks.md, "Configuration"): three levels of nesting, an event, its
 * matcher groups, and each group's handlers. A hooks file (`hooks/hooks.json`) wraps the event map
 * in a top-level `"hooks"` key; an inline object (the manifest's `hooks`, a skill's frontmatter,
 * the `hooks` of `settings.json`) is the event map itself. A file that fails the hooks schema does
 * not load ("Failed to load hooks from <path>: <reason>"), so any error leaves `hooks` empty; the
 * caller, which knows the path, adds it to the reason.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/hooks_settings.rs`, docs/RUST.md).
 */
export interface HooksRead {
  /** Official shape. Empty when an error keeps the file from loading. */
  readonly hooks: HooksSettings;
  readonly issues: readonly PluginIssue[];
  /**
   * Whether the plugin holding these hooks does not load at all (oracle 2.1.319, read `lenient`
   * only): a guard hook that cannot load "may be what guards the permissions declared beside it,
   * so nothing it sits in is applied".
   */
  readonly rejectsPlugin?: boolean;
}

/** How a hooks object treats an entry it cannot read. */
export interface HooksReadOptions {
  /**
   * Drop the entry with a warning instead of failing the whole object, except under the guarded
   * events: a guard that silently went missing could leave what it protected open. Plugin hooks
   * load this way (oracle 2.1.319, docs/SPEC.md "Sources of truth"); `settings.json` hooks do not
   * (settings.md: a managed event that is not a list ignores the whole field).
   */
  readonly lenient?: boolean;
}

/** Reads an event map: the `hooks` object of `settings.json`, or an inline `hooks` object. */
export const readHooksSettings = (raw: unknown, options: HooksReadOptions = {}): HooksRead =>
  call('readHooksSettings', raw, options);

/**
 * Reads `hooks/hooks.json`: `{ "hooks": { <event>: [ { matcher?, hooks: [ <handler> ] } ] } }`.
 * Other top-level keys are ignored: `modules` belongs to the mods reader (components.md, "Hooks"),
 * and a file with `modules` alone holds no hooks.
 */
export const readHooksFile = (raw: unknown, options: HooksReadOptions = {}): HooksRead =>
  call('readHooksFile', raw, options);
