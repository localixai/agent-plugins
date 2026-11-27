import type { UserConfigOption } from '../format/index.ts';
import type { ResolvedUserConfig, UserConfigValue } from '../loading/resolve/user-config.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * `userConfig` values that arrive as text from a shell (plugins/cli-reference.md, "plugin install",
 * `--config <key=value>`; "plugin configure", `--values-stdin`: "a JSON object of single-line
 * strings"): each is converted to the option's declared type and checked against it
 * (`validateUserConfigValues`: "Claude Code validates each value against the option's declared
 * type"). Pure. How a text becomes a number, a boolean or a list is a judgment (spec/judgments.md):
 * the docs show strings only.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/user_config_input.rs`, docs/RUST.md).
 */

const call = coreModule('cli/user-config-input');

export type DeclaredOptions = Readonly<Record<string, UserConfigOption>>;

export interface ConfigProblem {
  readonly key: string;
  readonly message: string;
  /** The key is not among the declared options. */
  readonly undeclared: boolean;
}

export interface CheckedInput {
  /** The values that passed, converted to their declared types. */
  readonly accepted: Readonly<Record<string, UserConfigValue>>;
  readonly problems: readonly ConfigProblem[];
}

/** A text as the option's type: numbers and booleans are parsed, a `multiple` list splits on commas. */
export const convertText = (option: UserConfigOption, text: string): UserConfigValue =>
  call('convertText', option, text);

/** Each given `key` → text against the declared options; undeclared keys and failing values are problems. */
export const checkGiven = (
  declared: DeclaredOptions,
  given: Readonly<Record<string, string>>,
): CheckedInput => call('checkGiven', declared, given);

/**
 * The options that are not set: no saved value. A `default` gives an option a value but does not set
 * it, so the dialog, `plugin configure` and the install line still count it (spec §3.4.4, §3.4.6).
 */
export const unsetKeys = (config: ResolvedUserConfig): readonly string[] =>
  call('unsetKeys', config);

/** The saved values merged with `accepted`: what a config reads once the values are stored. */
export const withValues = (
  config: ResolvedUserConfig,
  accepted: Readonly<Record<string, UserConfigValue>>,
): ResolvedUserConfig => call('withValues', config, accepted);
