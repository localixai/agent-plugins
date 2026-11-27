/**
 * The marketplace source `claude plugin marketplace add <source>` builds from the string you type
 * (plugins/cli-reference.md, "plugin marketplace add": the table of forms, the unrecognized-source
 * and bare-host messages, `--sparse`; plugins/marketplace-reference.md, "Marketplace sources"): pure
 * text in, a source or an issue out. The order the forms are tried in, and what the docs leave
 * open, are in spec/judgments.md.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/marketplace_input.rs`,
 * docs/RUST.md).
 */
import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('marketplace/marketplace-input');

export interface MarketplaceInputOptions {
  /** `.\`, `..\` and `C:\` paths are accepted only on Windows. */
  readonly platform?: 'posix' | 'windows';
  /** The `--sparse` directories; for `github` and `git` sources only. */
  readonly sparse?: readonly string[];
}

type Parsed = MarketplaceSource | PluginIssue;

/** The source the input stands for, or the issue that says it is none. */
export const parseMarketplaceInput = (
  text: string,
  options: MarketplaceInputOptions = {},
): Parsed => call('parseMarketplaceInput', text, options);
