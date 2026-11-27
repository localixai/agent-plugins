import type { PluginOnlySurface } from '../../protocol/settings-keys.ts';
import type { ParentSettingsContext } from '../managed/parent.ts';
import type { SkillsDirVerdict } from '../marketplaces/gate.ts';
import type { PolicyView } from '../view.ts';
import type { PreapprovalSkill } from './preapproval.ts';
import type { OriginOf } from './sources.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/customization/policy');

/**
 * `strictPluginOnlyCustomization` as one object over a view (docs/API.md, "Settings"): a host's
 * discovery of skills, agents, hooks and MCP servers asks `loads(surface, origin)` per candidate.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/customization/policy.rs`,
 * docs/RUST.md).
 */
export interface CustomizationPolicy {
  /** The kinds of customization the lock covers. */
  readonly locked: ReadonlySet<PluginOnlySurface>;
  /** Names in the list that no kind has: they lock nothing, and the status note counts them. */
  readonly unknownSurfaceNames: number;
  /** Whether a customization of `surface` from `origin` loads. */
  loads<S extends PluginOnlySurface>(surface: S, origin: OriginOf[S]): boolean;
  /** Whether skills-directory plugins load, and which key stops them when they do not. */
  skillsDirPlugins(): SkillsDirVerdict;
  /** Whether the skill's `allowed-tools` pre-approval holds. */
  skillAllowedToolsPreapproved(skill: PreapprovalSkill): boolean;
  /** The refusal of `claude mcp add` while MCP servers are locked to plugins; `undefined` otherwise. */
  readonly mcpAddRefusal: string | undefined;
}

/** The customization policy over a view the parent rules were already applied to (`resolvePolicy`). */
export const customizationOver = (effective: PolicyView): CustomizationPolicy =>
  call('customizationOver', effective);

/**
 * The key passes the parent filter (claude-apps-gateway.md), but a parent is still ignored where an
 * administrator deploys managed settings and does not let an embedding host add policy.
 */
export const createCustomizationPolicy = (
  view: PolicyView,
  context: ParentSettingsContext,
): CustomizationPolicy =>
  call('createCustomizationPolicy', view, {
    hasManagedSettings: context.hasManagedSettings,
    parentMerge: context.parentMerge,
  });
