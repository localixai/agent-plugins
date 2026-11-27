/**
 * Managed `sandbox.credentials` entries (settings-reference.md, "Invalid credential entries in
 * managed settings"): an entry in `files` or `envVars` with a valid `path` or `name` and a `mode` of
 * `mask` or `deny` that fails validation anywhere else "is degraded to `mode: "deny"` with a
 * warning, so the credential stays blocked, not masked"; an entry with an unknown `mode` or an
 * invalid `path` or `name` is stripped. Each case warns and the valid entries stay. A `deny` entry
 * accepts the mask fields and ignores them, so only a `mask` entry is checked against them. The two
 * lists have their own rules (the tables "Mask fields for files" and "Mask fields for environment
 * variables").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/sandbox_credentials.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/general/sandbox-credentials');

/** Whether a credential entry has `mode: "deny"`: it blocks the credential and needs no repair. */
export const isDenyEntry = (entry: unknown): boolean => call('isDenyEntry', entry);
