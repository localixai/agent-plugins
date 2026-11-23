import type { Settings } from '../../protocol/official.ts';
import type { ManagedComposition } from './compose.ts';
import { withCrossSourceKeys } from './cross-source.ts';
import { signInRequirementOf, withSignInRequirement } from './login-keys.ts';

/**
 * The managed tier after a policy helper emitted `managedSettings` (settings-reference.md, "Write
 * the helper output"): "that object becomes the only managed settings source for the run", the
 * cross-source keys read from it alone, and parent settings are never merged.
 */
export function withHelperSettings(
  composition: ManagedComposition,
  helperOutput: Settings,
): ManagedComposition {
  // `forceRemoteSettingsRefresh` "has its own startup rule" that runs before the helper and reads
  // the admin sources, so the helper's copy takes no part (`composition.forceRemoteRefresh`).
  const emitted: Settings = Object.fromEntries(
    Object.entries(helperOutput).filter(([name]) => name !== 'forceRemoteSettingsRefresh'),
  );
  // `enforceAnthropicSignIn` is still read from the MDM policy or file, never from the helper.
  const settings = withSignInRequirement(
    withCrossSourceKeys({ admins: [emitted], onMachine: [true], merging: false }, emitted),
    signInRequirementOf(composition.layer.settings),
  );
  return {
    ...composition,
    layer: { scope: 'managed', settings },
    used: [],
    skipped: [],
    settingSources: 'Enterprise managed settings (helper)',
    skippedSources: undefined,
    hasManagedSettings: true,
    parentMerge: false,
    admins: [emitted],
  };
}
