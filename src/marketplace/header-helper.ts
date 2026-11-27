import type { PluginIssue } from '../format/issues.ts';
import type { CommandRunner } from '../ports/command-runner.ts';
import type { Clock } from '../ports/clock.ts';
import type { Environment } from '../ports/environment.ts';
import type { Locations } from '../ports/locations.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { HeaderLayer, HeaderPlace, HeadersSetting } from './header-policy.ts';
import type { ResolvedHeaders } from './types.ts';

/**
 * Running a `headersHelper` command (plugins/host-marketplace.md, "Write the headersHelper
 * command", "When Claude Code skips a headersHelper command or drops its output"): through the
 * `CommandRunner` port, from the configuration directory, under 10 seconds, printing one JSON
 * object of header names and string values. A command that fails means the fetch or download it
 * was run for does not happen: a `PluginIssue`, never a throw.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/header_helper.rs`,
 * docs/RUST.md): the run is the core's; `helperCache` stays here, at the edge, as it keeps the
 * host's promises of runs still in flight, so concurrent downloads on one origin share one run.
 */

const call = coreModule('marketplace/header-helper');

export interface HeadersHelperPorts {
  /**
   * Runs the command with `cwd` the configuration directory. Claude Code has that directory by the
   * time a command runs, a missing one made at startup (oracle 2.1.322, spec/judgments.md), so a
   * host whose configuration directory may not exist yet creates it before it runs the command.
   */
  readonly runner: CommandRunner;
  /** The environment the command inherits, less the credential variables for a command that was not the user's own. */
  readonly environment: Environment;
  /** `homeDir()` and `joinPath` find the configuration directory (`~/.claude`) when `CLAUDE_CONFIG_DIR` is unset. */
  readonly locations: Locations;
}

const CACHE_MS = 60_000;

/** Runs the layer's command and reads what it printed; a layer without a command prints no headers. */
export const runHeadersHelper = (
  layer: HeaderLayer,
  ports: HeadersHelperPorts,
  setting: Pick<HeadersSetting, 'platform'>,
  { signal }: { readonly signal: AbortSignal },
): Promise<ResolvedHeaders | PluginIssue> =>
  call.async('runHeadersHelper', layer, ports, { platform: setting.platform }, { signal });

/** Which places reuse one run: "Claude Code reuses one run's output for up to 60 seconds" (a `url` source's command). */
const REUSED: { readonly [P in HeaderPlace]: boolean } = { 'url-source': true, entry: false };

export interface HelperCache {
  /** The run of `produce` for this layer's command, reused for up to 60 seconds when the place allows; a failure is never kept. */
  remember(
    layer: HeaderLayer,
    produce: () => Promise<ResolvedHeaders | PluginIssue>,
  ): Promise<ResolvedHeaders | PluginIssue>;
}

const keyOf = (layer: HeaderLayer): string =>
  JSON.stringify([layer.origin, layer.headersHelper, layer.variables]);

/** A cache of command runs over the clock (`Clock.now`); keep one per host session. */
export function helperCache(clock: Clock): HelperCache {
  const kept = new Map<
    string,
    { readonly at: number; readonly found: Promise<ResolvedHeaders | PluginIssue> }
  >();
  return {
    remember(layer, produce) {
      if (!REUSED[layer.place]) return produce();
      const key = keyOf(layer);
      const found = kept.get(key);
      if (found !== undefined && clock.now() - found.at < CACHE_MS) return found.found;
      // A run still in flight is shared too: concurrent downloads on one origin run the command once.
      const run = produce();
      const own = { at: clock.now(), found: run };
      kept.set(key, own);
      const forgetFailure = (produced: ResolvedHeaders | PluginIssue): void => {
        if ('severity' in produced && kept.get(key) === own) kept.delete(key);
      };
      return run.then(
        (produced) => {
          forgetFailure(produced);
          return produced;
        },
        (reason: unknown) => {
          if (kept.get(key) === own) kept.delete(key);
          return Promise.reject(reason);
        },
      );
    },
  };
}
