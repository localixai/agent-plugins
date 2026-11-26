import type { Contribution, FlagContext } from './types.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('cli/flags/channels');

/**
 * `--channels` and `--dangerously-load-development-channels` (cli-reference.md, "CLI flags";
 * channels.md, "Enterprise controls"; channels-reference.md, "Test during the research preview").
 * `--channels` takes a "space-separated list of `plugin:<name>@<marketplace>` entries"; the
 * development flag "accepts `plugin:<name>@<marketplace>` and `server:<name>` entries", prompts for
 * confirmation, "so it takes effect in interactive sessions", and with `-p` Claude Code "ignores the
 * flag". The kit builds the entries `channelVerdict` reads; the verdict is that module's. The wording
 * of an entry that is neither form is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/flags/channels.rs`, docs/RUST.md).
 */

export const channels = (values: readonly string[]): Contribution => call('channels', values);

export const developmentChannels = (
  values: readonly string[],
  context: FlagContext,
): Contribution => call('developmentChannels', values, context);
