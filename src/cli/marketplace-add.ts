/**
 * `claude plugin marketplace add <source>` as two decisions around the host's fetch
 * (plugins/cli-reference.md, "plugin marketplace add", "JSON result for marketplace commands";
 * plugins/marketplace-reference.md, "Reserved names"): `startMarketplaceAdd` reads the arguments and
 * says what to fetch, or what the command ends with; `concludeMarketplaceAdd` takes what the fetch
 * found and says what to declare, what to install and what to print. The source forms, the
 * unrecognized-source and bare-host messages and `--sparse` are `parseMarketplaceInput`'s. What the
 * docs leave open is in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/cli/marketplace_add.rs`, docs/RUST.md).
 */
import type { RegisteredMarketplace } from '../format/marketplace/marketplace-names.ts';
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import type { InstallPlan } from '../marketplace/dependency-install-plan.ts';
import type { GraphFacts } from '../marketplace/dependency-graph.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { CommandOutput } from './result.ts';
import type { ConfigScope } from './scope.ts';

const call = coreModule('cli/marketplace-add');

export interface AddArguments {
  readonly source: string;
  /** The raw `--scope` value. */
  readonly scope?: string;
  /** `--sparse`. */
  readonly sparse?: readonly string[];
  /** `--claudeai`: `source` is the name of a marketplace hosted on claude.ai. */
  readonly claudeai?: boolean;
  /** `--json`. */
  readonly json?: boolean;
  readonly platform?: 'posix' | 'windows';
}

export type AddStart =
  | {
      readonly kind: 'fetch';
      readonly source: MarketplaceSource;
      readonly scope: ConfigScope;
      /** The host prints the JSON line (`--json`). */
      readonly json: boolean;
    }
  /** "`--json` ... has no effect with `--claudeai`": the human line is printed. */
  | { readonly kind: 'claudeai'; readonly name: string; readonly json: false }
  | { readonly kind: 'done'; readonly output: CommandOutput };

/** What to fetch, or what the command ends with before any fetch. */
export const startMarketplaceAdd = (args: AddArguments): AddStart =>
  call('startMarketplaceAdd', args);

/** What the host's fetch found. */
export interface AddFetched {
  /** The `name` of the marketplace's own manifest. */
  readonly name: string;
  /** A copy of it is already on disk. */
  readonly alreadyOnDisk: boolean;
  /** The marketplaces already registered, for the download-folder rule. */
  readonly registered: readonly RegisteredMarketplace[];
  /** The plugin graph with this marketplace's catalog read; absent when the host has no installed plugins to check. */
  readonly graph?: GraphFacts;
  /** The source a user or managed `extraKnownMarketplaces` entry lists for this name; absent when none does. */
  readonly declared?: MarketplaceSource;
  /** How many LFS-tracked files the clone left as pointer files (`github` and `git` sources). */
  readonly lfsPointers?: number;
}

export interface AddConclusion {
  readonly output: CommandOutput;
  /** The `extraKnownMarketplaces` entry to write; absent when nothing is declared (a failure, a claude.ai marketplace). */
  readonly declare?: {
    readonly scope: ConfigScope;
    readonly name: string;
    readonly source: MarketplaceSource;
  };
  /** The dependency installs of installed plugins the new catalog can now satisfy, one plan per plugin. */
  readonly dependencies: readonly InstallPlan[];
}

/** The first reason the name is refused, or `undefined`; `plugin install --marketplace` runs the same check. */
export const nameRefusal = (
  found: Pick<AddFetched, 'name' | 'registered'>,
  source: MarketplaceSource | undefined,
): string | undefined =>
  call('nameRefusal', { name: found.name, registered: found.registered }, source);

/** The starts that wait for the host's fetch. */
export type PendingAdd = Extract<AddStart, { kind: 'fetch' | 'claudeai' }>;

/** What the add ends with once the marketplace is fetched. */
export const concludeMarketplaceAdd = (start: PendingAdd, found: AddFetched): AddConclusion =>
  call('concludeMarketplaceAdd', start, found);
