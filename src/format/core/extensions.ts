/**
 * Client extension data (core §8, §8.1): an `extensions` field is an object whose members are
 * extension namespaces and whose values are objects. The kit checks that shape only: a namespace's
 * contents belong to the client that owns it, and the members of a namespace the host does not
 * implement are ignored without validating their values.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/extensions.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

/** The data of the namespaces a host implements, as written; namespaces it does not implement are dropped. */
export type CoreExtensions = Readonly<Record<string, Readonly<Record<string, unknown>>>>;

const call = coreModule('format/core/extensions');

/** Why an `extensions` value breaks core §8.1, or `undefined` when its shape is right. */
export const extensionsProblem = (value: unknown): string | undefined =>
  call('extensionsProblem', value);

/**
 * The members of a valid `extensions` value that the host implements (core §8.1: the others are
 * ignored). `undefined` when nothing implemented is there, so a result carries no empty object.
 */
export const implementedExtensions = (
  value: unknown,
  implemented: readonly string[],
): CoreExtensions | undefined => call('implementedExtensions', value, implemented);
