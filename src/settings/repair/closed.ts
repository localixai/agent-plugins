/**
 * The repairs of the keys that have their own fallback (managed-settings.md, "Keys that fail
 * closed", the table): in the policy tier an unreadable value reads as the fallback the table gives;
 * every other source loses the value, and an individual invalid entry is stripped everywhere.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/repair/closed.rs`, docs/RUST.md).
 */
import type { Repair } from './support.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/repair/closed');

/**
 * `allowedProviders`: unreadable in managed settings it is an empty allowlist, so every provider is
 * refused; an entry that is no known provider name is dropped and reported, the rest enforced.
 */
export const providerAllowlist: Repair = (value, at) => call('providerAllowlist', value, at);

/** `availableModelsMatch`: `prefix` or `exact`; unreadable in managed settings it reads as `exact`. */
export const matchMode: Repair = (value, at) => call('matchMode', value, at);

/**
 * `deniedModels`: a non-text entry is stripped and the rest enforced; a wholly invalid value is
 * dropped with a warning and blocks no model until fixed.
 */
export const modelDenylist: Repair = (value, at) => call('modelDenylist', value, at);

/**
 * `forceLoginOrgUUID`: a UUID or a list of them. In managed settings "a value that isn't a string or
 * an array of strings" admits no organization, which the empty list says: any of no organizations
 * (settings-reference.md, `forceLoginOrgUUID`; spec/judgments.md). A list with one non-text entry is
 * not an array of strings, so it reads that way whole: stripping the entry would admit the others.
 * Outside the policy tier an invalid entry is stripped like any list entry.
 */
export const orgUuid: Repair = (value, at) => call('orgUuid', value, at);

/**
 * `gatewayInternalNetworks`: unreadable in managed settings, `/login` "refuses every new cloud
 * gateway sign-in on that machine until the value is fixed". No value says that, so the layer records
 * the key as invalid (`PolicyLayer.invalid`).
 */
export const gatewayNetworks: Repair = (value, at) => call('gatewayNetworks', value, at);

/** A key another module judges from the document as written: kept whatever the value. */
export const asWritten: Repair = (value) => call('asWritten', value);
