import type { PluginIssue } from '../../format/issues.ts';
import { coreModule } from '../../wasm/bridge.ts';
import type { ResolvedUserConfig } from './user-config.ts';

/**
 * The checks of a saved `userConfig` value that the manifest's fields state (plugins/
 * manifest-reference.md, "User configuration"): `type` ("One of `string`, `number`, `boolean`,
 * `directory`, or `file`"), `required` ("the configuration dialog doesn't accept an empty value"),
 * `multiple` ("allows an array of strings"), `options` ("the values the field accepts") and `min` /
 * `max` ("Bounds for `number`"). The dialog is the host's; this tells it, and a loader that reads
 * saved settings, whether a value would be accepted. Judgment: the value checked is the one
 * `${user_config.KEY}` stands for, the saved one else the `default`, and a value that is not
 * declared is not checked (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/resolve/user_config_values.rs`,
 * docs/RUST.md).
 */

const call = coreModule('loading/resolve/user-config-values');

/** One error per declared option whose value would not be accepted: `userConfig.<key>`. */
export const validateUserConfigValues = (config: ResolvedUserConfig): PluginIssue[] =>
  call('validateUserConfigValues', config);
