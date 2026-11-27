import type { ParentSettingsContext } from '../managed/parent.ts';
import type { SyncDecision } from '../plugins/sync-plugins.ts';
import type { PolicyView } from '../view.ts';
import type { BundledSkillsDecision } from './bundled.ts';
import type { SkillSubject, SkillVisibility } from './overrides.ts';
import type { ShellOrigin, ShellVerdict } from './shell.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/skills/policy');

/** What the host tells the skill policy besides the layers. */
export interface SkillPolicyContext extends ParentSettingsContext {
  /** The host's environment as data (`CLAUDE_CODE_DISABLE_BUNDLED_SKILLS`); the kit reads no `process.env`. */
  readonly env: Readonly<Record<string, string | undefined>>;
}

/** The skill keys of the settings as one object over a view (docs/API.md, "Settings"). */
export interface SkillSettingsPolicy {
  visibility(skillName: string, subject?: SkillSubject): SkillVisibility;
  bundled(): BundledSkillsDecision;
  readonly bundledSkillsDisabled: boolean;
  shell(origin: ShellOrigin): ShellVerdict;
  syncClaudeAiSkills(): SyncDecision;
}

/**
 * `disableSkillShellExecution: true` in the SDK's `managedSettings` takes effect, but parent settings
 * are ignored where an administrator deploys managed settings (claude-apps-gateway.md): the shell
 * verdict reads the view under the parent rules.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/skills/policy.rs`, docs/RUST.md).
 */
export const createSkillSettingsPolicy = (
  view: PolicyView,
  context: SkillPolicyContext,
): SkillSettingsPolicy =>
  call('createSkillSettingsPolicy', view, {
    hasManagedSettings: context.hasManagedSettings,
    parentMerge: context.parentMerge,
    env: context.env,
  });
