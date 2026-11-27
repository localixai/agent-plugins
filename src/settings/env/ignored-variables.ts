/**
 * The `env` variables Claude Code drops (settings-reference.md, "Variables Claude Code ignores in
 * `env`"), as data. Two lists: the variables "a checked-out repository shouldn't control", dropped
 * from project and local settings, and the variables Claude Code reads from the launch environment
 * only or exports itself, dropped from every file. The page's two open-ended lists ("such as" the
 * operating-system directories, the hosting-environment identity variables) hold the names the page
 * spells out and nothing the kit would have to guess (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/env/ignored_variables.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyScope } from '../layers.ts';

const call = coreModule('settings/env/ignored-variables');

const IGNORE_REASONS = [
  'storage',
  'windows',
  'session-content',
  'telemetry',
  'startup',
  'sign-in',
  'accessibility',
  'dialog-timer',
  'attachments',
  'file-mentions',
  'identity',
  'launch-only',
] as const;
export type IgnoreReason = (typeof IGNORE_REASONS)[number];

/**
 * The text of an `env` value as the process gets it: claude sets JavaScript's `String(value)` for a
 * value that is not a string (oracle 2.1.321, spec/judgments.md `src/settings/env/check-env.ts`).
 */
export const envText = (value: unknown): string => call('envText', value);

/**
 * Why Claude Code drops the variable `name` set to `value` in a file of `scope`, or `undefined` when
 * the variable stays. `windows`: the host's variable names are case-insensitive.
 */
export const ignoredBecause = (
  scope: PolicyScope,
  name: string,
  value: unknown,
  windows = false,
): IgnoreReason | undefined => call('ignoredBecause', scope, name, value, windows);
