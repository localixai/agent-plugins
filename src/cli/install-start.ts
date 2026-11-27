import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { Clock } from '../ports/clock.ts';
import type { Environment } from '../ports/environment.ts';
import type { KnownMarketplace, RefreshDecision } from '../updates/refresh-before-install.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ConfigScope } from './scope.ts';

/**
 * The first half of `claude plugin install <plugin>` (plugins/cli-reference.md, "plugin install"):
 * what the arguments mean and what the host does before it looks the plugin up: add the
 * `--marketplace` source first, refresh the marketplace the install names (plugins/loading.md, "When
 * Claude Code refreshes a marketplace before an install", through `refreshBeforeInstall`). The rest
 * is `planInstall`. A marketplace added by `--marketplace` is "declared in your user settings even
 * when you pass `--scope project`" (plugins/install.md, "Add and install from your shell"); it was
 * just fetched, so no refresh follows it (judgment, spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/install_start.rs`, docs/RUST.md).
 */

const call = coreModule('cli/install-start');

export interface InstallArguments {
  /** `name`, or `name@marketplace`; the bare name alone with `--marketplace`. */
  readonly plugin: string;
  readonly scope?: string;
  /** Each `--config` flag as written, `key=value`. */
  readonly config?: readonly string[];
  readonly json?: boolean;
  readonly yes?: boolean;
  readonly acceptCommand?: string;
  /** `--marketplace <source>`. */
  readonly marketplace?: string;
  readonly platform?: 'posix' | 'windows';
}

export interface InstallContext {
  readonly marketplaces: readonly KnownMarketplace[];
  readonly env: Environment;
  readonly clock: Clock;
  /** The command runs inside a Claude Code session. */
  readonly session: boolean;
  /** Both stdin and stdout are a terminal. */
  readonly tty: boolean;
}

/** What the host does before the lookup. */
export interface PendingInstall {
  readonly plugin: string;
  readonly scope: ConfigScope;
  readonly json: boolean;
  readonly given: Readonly<Record<string, string>>;
  readonly yes: boolean;
  readonly acceptCommand?: string;
  readonly session: boolean;
  readonly tty: boolean;
  /** Add this marketplace first, to the user settings. */
  readonly addMarketplace?: { readonly source: MarketplaceSource; readonly scope: 'user' };
  readonly refresh: RefreshDecision;
}

export type InstallStart =
  | { readonly kind: 'done'; readonly output: CommandOutput }
  | { readonly kind: 'prepare'; readonly pending: PendingInstall };

/** Reads the arguments and says what to do before the lookup, or what the command ends with. */
export const startInstall = (args: InstallArguments, context: InstallContext): InstallStart =>
  call('startInstall', args, context);
