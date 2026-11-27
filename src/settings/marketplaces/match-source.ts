/**
 * The matcher both marketplace lists share (settings-reference.md, `strictKnownMarketplaces`
 * "Allowed source types", "Owner wildcards", "Exact matching"; plugins/org.md "How entries match",
 * "Blocklist with `blockedMarketplaces`"). The allowlist matches by exact entry, owner wildcard or
 * regular expression; the blocklist matches the same entries more widely: canonical git URLs,
 * `github` and `git` as one, owner comparison in any case, no `ref` or `path` meaning every one.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/match_source.rs`,
 * docs/RUST.md).
 */
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/match-source');

/** Which list an entry is read as. */
export type ListKind = 'strict' | 'blocked';

/**
 * Whether a list entry matches the source of a marketplace. `skills-dir` entries match no
 * marketplace; they only decide whether skills-directory plugins load (see `gate.ts`).
 */
export const matchMarketplaceSource = (
  list: ListKind,
  entry: SdkMarketplaceSource,
  source: SdkMarketplaceSource,
): boolean => call('matchMarketplaceSource', list, entry, source);

/**
 * Whether a list entry matches a marketplace hosted on claude.ai, by its host `claude.ai`: only a
 * `hostPattern` entry can (plugins/org.md, "Marketplaces hosted on claude.ai"). Whose the marketplace
 * is decides the allowlist's admission (`MarketplaceGate.checkClaudeAi`), not the match.
 */
export const matchClaudeAiMarketplace = (list: ListKind, entry: SdkMarketplaceSource): boolean =>
  call('matchClaudeAiMarketplace', list, entry);
