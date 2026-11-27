/**
 * The first step of a core load (core §5.1: the manifest is loaded and validated before any
 * component is discovered): a root `plugin.json` that is missing, not a regular file, outside the
 * plugin root (core §4.1, boundary 1), not JSON or invalid by core §5 rejects the package.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/core/manifest_step.rs`, docs/RUST.md).
 */
import type { CoreManifestResult } from '../../format/core/manifest.ts';
import type { LocateContext } from '../../format/locate.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('loading/core/manifest-step');

export const coreManifestStep = (
  context: LocateContext,
  implemented: readonly string[],
): Promise<CoreManifestResult> => call.async('coreManifestStep', context, implemented);
