/** Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/lists.rs`, docs/RUST.md). */
import type { Repair } from './support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/repair/lists');

/** A list of texts: a non-text entry is dropped, a value that is no list is dropped whole. */
export const stringList: Repair = (value, at) => call('stringList', value, at);

/**
 * `allowedHttpHookUrls` and `httpHookAllowedEnvVars`: an allowlist that is unreadable in managed
 * settings is enforced as an empty one (managed-settings.md, "Keys that fail closed"), an entry
 * that is invalid is stripped and the rest enforced.
 */
export const closedStringList: Repair = (value, at) => call('closedStringList', value, at);

/**
 * `allowedChannelPlugins`: an entry is `{ marketplace, plugin }` or the string `plugin@marketplace`,
 * which Claude Code "treats as the equivalent object", so the result holds objects only.
 */
export const channelAllowlist: Repair = (value, at) => call('channelAllowlist', value, at);

/**
 * `strictKnownMarketplaces`: unreadable in managed settings it is an empty allowlist, a total
 * lockdown; an entry that is invalid or "can't be enforced, such as a `hostPattern` regex that
 * doesn't compile, is stripped and the valid subset is enforced" (managed-settings.md).
 */
export const strictSources: Repair = (value, at) => call('strictSources', value, at);

/**
 * `blockedMarketplaces`: an invalid entry is stripped; "an entry that parses but can never match,
 * such as a `hostPattern` regex that doesn't compile, is kept with a warning" (so the restriction
 * stays active); a wholly invalid value is dropped with a warning, since blocking every
 * marketplace would block sources the policy never named (managed-settings.md).
 */
export const blockedSources: Repair = (value, at) => call('blockedSources', value, at);
