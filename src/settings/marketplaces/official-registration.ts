import type { MarketplaceGate } from './gate.ts';
import { OFFICIAL_SOURCE } from './registry.ts';

/**
 * Whether Claude Code registers the official marketplace by itself (settings-reference.md, "Allow only
 * the official marketplace"; env-vars.md, `CLAUDE_CODE_DISABLE_OFFICIAL_MARKETPLACE_AUTOINSTALL`). Pure
 * over facts the host keeps; the host registers it and records what the verdict says to record.
 */

/**
 * How this session was launched: an interactive terminal session, a `-p` run, a terminal attached to
 * a cloud session, or the VS Code extension. Only the first registers ("the first time you start an
 * interactive terminal session").
 */
const OFFICIAL_REGISTRATION_LAUNCHES = ['terminal', 'print', 'cloud-terminal', 'vs-code'] as const;
export type OfficialRegistrationLaunch = (typeof OFFICIAL_REGISTRATION_LAUNCHES)[number];

const REGISTERS_AT: { readonly [L in OfficialRegistrationLaunch]: boolean } = {
  terminal: true,
  print: false,
  'cloud-terminal': false,
  'vs-code': false,
};

export interface OfficialRegistrationFacts {
  readonly launch: OfficialRegistrationLaunch;
  /** The official marketplace is already registered on this machine: it is kept. */
  readonly registered: boolean;
  /** The host recorded a blocked attempt or a skip before: "doesn't retry after the policy changes". */
  readonly skipRecorded: boolean;
  /** `officialMarketplaceAutoinstallEnabled(env)`: `false` "skips the registration permanently". */
  readonly autoinstallEnabled: boolean;
}

export type OfficialRegistration =
  /** Already registered, or not this kind of launch: nothing happens and nothing is recorded. */
  | { readonly action: 'none' }
  /** The host registers `claude-plugins-official` from its GitHub repository. */
  | { readonly action: 'register' }
  /**
   * The policy blocks the marketplace or the variable is set: the host records the skip and never
   * registers it by itself again; a managed `extraKnownMarketplaces` entry, an `enabledPlugins`
   * entry for one of its plugins or `claude plugin marketplace add anthropics/claude-plugins-official`
   * still registers it.
   */
  | { readonly action: 'record-skip' };

const NONE: OfficialRegistration = { action: 'none' };

export function officialMarketplaceRegistration(
  gate: MarketplaceGate,
  facts: OfficialRegistrationFacts,
): OfficialRegistration {
  if (facts.registered || facts.skipRecorded || !REGISTERS_AT[facts.launch]) return NONE;
  if (!facts.autoinstallEnabled || !gate.check(OFFICIAL_SOURCE).allowed) {
    return { action: 'record-skip' };
  }
  return { action: 'register' };
}
