/**
 * The names the docs print (plugins/marketplace-reference.md, "Reserved names"): one list per
 * group, in the docs' order. The checks that use them are `marketplace-names.ts`.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/marketplace/marketplace_name_lists.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('format/marketplace/marketplace-name-lists');

export const OFFICIAL_NAMES: readonly [
  'claude-code-marketplace',
  'claude-code-plugins',
  'claude-plugins-official',
  'anthropic-marketplace',
  'anthropic-plugins',
  'agent-skills',
  'anthropic-agent-skills',
  'life-sciences',
  'knowledge-work-plugins',
  'claude-for-legal',
  'claude-for-financial-services',
  'financial-services-plugins',
  'first-party-plugins',
  'claude-tag-plugins',
] = call('OFFICIAL_NAMES');

/**
 * The official names whose auto-update default is off, though they are official: "on for
 * Anthropic's official marketplaces such as `claude-plugins-official`, off for
 * `knowledge-work-plugins` and `first-party-plugins`" (plugins/loading.md, "Which marketplaces and
 * plugins auto-update", item 3). The one list: the registry's plan and the update pass read it.
 */
export const OFFICIAL_BUT_OFF: readonly ['knowledge-work-plugins', 'first-party-plugins'] =
  call('OFFICIAL_BUT_OFF');

/** Whether `name` is one of `OFFICIAL_BUT_OFF`. */
export const isOfficialButOff = (name: string): boolean => call('isOfficialButOff', name);

export const COMMUNITY_NAMES: readonly [
  'claude-community',
  'claude-plugins-community',
  'healthcare',
] = call('COMMUNITY_NAMES');
export const DIRECTORY_NAMES: readonly ['anthropic-plugin-directory', 'claude-plugin-directory'] =
  call('DIRECTORY_NAMES');

/** Names Claude Code gives plugins that come from no marketplace, and what each one means. */
export const NON_MARKETPLACE_NAMES: ReadonlyMap<string, string> = call('NON_MARKETPLACE_NAMES');

/** Reserved "in any casing" (requires Claude Code v2.1.275 or later). */
export const ANY_CASING_NAMES: ReadonlySet<string> = call('ANY_CASING_NAMES');

/** Member names every JavaScript object has (requires Claude Code v2.1.296 or later). */
export const OBJECT_MEMBER_NAMES: ReadonlySet<string> = call('OBJECT_MEMBER_NAMES');
