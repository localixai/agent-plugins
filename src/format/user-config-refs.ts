import { coreModule } from '../wasm/bridge.ts';

/**
 * `${user_config.KEY}` references (plugins/manifest-reference.md, "Reference a saved value") and the
 * option keys they name: the one reader every field that substitutes, refuses or checks them uses.
 * A reference is a braced `${...}` whose text starts with `user_config.`; the key is what follows,
 * as written (a bundled server's keys included). Whether the key is a valid option key is
 * `USER_CONFIG_KEY`'s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/user_config_refs.rs`, docs/RUST.md).
 */

const call = coreModule('format/user-config-refs');

/** `userConfig` keys: letters, digits and underscores, not starting with a digit. */
export const USER_CONFIG_KEY: RegExp = call('USER_CONFIG_KEY');

/** The key of a `user_config.KEY` reference (the text inside `${...}`), or `undefined` for any other. */
export const userConfigKeyIn = (inner: string): string | undefined =>
  call('userConfigKeyIn', inner);

/**
 * The option a reference names when its key is a valid option key (`USER_CONFIG_KEY`), else
 * `undefined`: `${user_config.2fa}` names no option.
 */
export const validOptionKeyIn = (inner: string): string | undefined =>
  call('validOptionKeyIn', inner);

/** The keys `text` references, in order, repeats kept. */
export const userConfigReferences = (text: string): string[] => call('userConfigReferences', text);

/** `text` with each reference replaced by `replace(key, reference)`; other `${...}` stay as written. */
export const replaceUserConfigReferences = (
  text: string,
  replace: (key: string, reference: string) => string,
): string => call('replaceUserConfigReferences', text, replace);
