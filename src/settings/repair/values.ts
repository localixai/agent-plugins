/** Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/values.rs`, docs/RUST.md). */
import type { Repair } from './support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/repair/values');

/** A Boolean: anything else is dropped with a warning, even the text `"true"` (the docs' `disableAllHooks` case). */
export const boolean: Repair = (value, at) => call('boolean', value, at);

/**
 * A Boolean lock (`allowManagedHooksOnly`, `disableSkillShellExecution`...). In the policy tier the
 * text `"true"` or `"false"` reads as that Boolean with a notice to drop the quotes, and any other
 * unreadable value reads as `true`, the restrictive one (managed-settings.md, "Keys that fail
 * closed"). Elsewhere an unreadable value is dropped.
 */
export const lockBoolean: Repair = (value, at) => call('lockBoolean', value, at);

/**
 * `disableAllHooks`. In the SDK's `managedSettings` "any value but `false` and `"false"` binds" as
 * `allowManagedHooksOnly: true`, "because an enterprise configuration often yields a boolean as
 * text", so there an unreadable value reads as `true`, with a warning. Every other source reads a
 * Boolean and drops anything else.
 */
export const hooksSwitch: Repair = (value, at) => call('hooksSwitch', value, at);

export const string: Repair = (value, at) => call('string', value, at);

/** An object whose entries the kit reads later, per plugin or per server. */
export const record: Repair = (value, at) => call('record', value, at);

/**
 * `strictPluginOnlyCustomization` is `true` or a list of surfaces. Unreadable in the policy tier it
 * reads as `true`, locking all four; an entry of a list that this version does not know locks
 * nothing, and the customization policy counts it (managed-settings.md), so a list keeps its
 * entries as written.
 */
export const pluginOnly: Repair = (value, at) => call('pluginOnly', value, at);

/**
 * `enabledPlugins`: an invalid entry is dropped with a warning and the others stay; a value that is
 * no map of plugin ids, or whose every entry is invalid, is dropped whole (managed-settings.md).
 * The SDK also types a list or an object as the value (version constraints), kept as written.
 */
export const enabledPlugins: Repair = (value, at) => call('enabledPlugins', value, at);

/** `extraKnownMarketplaces`: a name maps to an object with a `source` object; others are dropped. */
export const marketplaceMap: Repair = (value, at) => call('marketplaceMap', value, at);

/** `skillOverrides`: a skill name maps to one of the four states; any other entry is dropped. */
export const skillOverrides: Repair = (value, at) => call('skillOverrides', value, at);

export const workflowSize: Repair = (value, at) => call('workflowSize', value, at);
