/**
 * The marketplace gate: `strictKnownMarketplaces` and `blockedMarketplaces` as verdicts over a
 * policy view (settings-reference.md, both keys; plugins/org.md, "Restrict what users can install").
 * Effects stay the host's: the kit says whether a source may be added, installed, updated,
 * refreshed, auto-updated or loaded, and why.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/marketplaces/gate.rs`,
 * docs/RUST.md).
 */
import type { SdkMarketplaceSource } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/marketplaces/gate');

export type SourceVerdict =
  | { readonly allowed: true }
  | {
      readonly allowed: false;
      readonly by: 'blockedMarketplaces' | 'strictKnownMarketplaces';
      readonly message: string;
    };

export type RegisteredVerdict =
  | { readonly loads: true }
  | { readonly loads: false; readonly message: string };

/** Whether skills-directory plugins (`@skills-dir`) load, and the list that stops them. */
export type SkillsDirVerdict =
  | { readonly admitted: true }
  | {
      readonly admitted: false;
      readonly by:
        | 'blockedMarketplaces'
        | 'strictKnownMarketplaces'
        | 'strictPluginOnlyCustomization';
    };

/**
 * Whose a marketplace hosted on claude.ai is, as claude.ai states it: the organization's, a
 * claude.ai default one, or one made of a member's own uploads; `undefined` when it did not say.
 */
export type ClaudeAiMarketplaceScope = 'organization' | 'default' | 'member';

export interface MarketplaceGate {
  /** On add, install, update, refresh and auto-update: may a marketplace come from this source? */
  check(source: SdkMarketplaceSource): SourceVerdict;
  /** At session start: does a marketplace registered from this source still load its plugins? */
  checkRegistered(name: string, source: SdkMarketplaceSource): RegisteredVerdict;
  /** A marketplace hosted on claude.ai, matched by its host `claude.ai` (plugins/org.md). */
  checkClaudeAi(name: string, scope: ClaudeAiMarketplaceScope | undefined): RegisteredVerdict;
  /** A marketplace Claude Code cannot find: it does not load while a restriction is active. */
  checkMissing(name: string): RegisteredVerdict;
  /** Any allowlist is set, or the blocklist names a source other than `skills-dir`. */
  restrictionActive(): boolean;
  /** `skills-dir` entries: whether skills-directory plugins may load. */
  skillsDirectoryPlugins(): SkillsDirVerdict;
}

/** The gate over a view; the view never changes, so each list is read from it once. */
export const createMarketplaceGate = (view: PolicyView): MarketplaceGate =>
  call('createMarketplaceGate', view);
