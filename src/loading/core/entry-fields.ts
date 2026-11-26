import type { PluginIssue } from '../../format/issues.ts';
import type { MarketplaceEntry } from '../../format/marketplace/types.ts';
import { coreModule } from '../../wasm/bridge.ts';

/** Implemented by the Rust core (`crates/agent-plugins/src/loading/core/entry_fields.rs`, docs/RUST.md). */
const call = coreModule('loading/core/entry-fields');

/**
 * What a marketplace entry says that a core package does not read (Agent Plugins Extended §18.1.5
 * item 2, J18.15): its component fields and `strict: false` are Claude Code configuration, and a
 * core package has fixed locations and nothing configured beside them (core §5.1, §6.1). Each is
 * reported in the package's errors and the package loads as the core says; the entry's metadata
 * applies as for any entry. Judgment: every component key the entry reader keeps is reported, the
 * chapter's eight and `userConfig`, `channels`, `settings` and `workflows`, none of which a core
 * package reads either.
 */
export const coreEntryIssues = (entry: MarketplaceEntry | undefined): readonly PluginIssue[] =>
  call('coreEntryIssues', entry);
