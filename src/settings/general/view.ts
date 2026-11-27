import type { Settings } from '../../protocol/official.ts';
import type { PolicyView } from '../view.ts';
import type { PolicyLayer } from '../layers.ts';
import { coreModule } from '../../wasm/bridge.ts';

/**
 * The policy view, plus every setting of the files as one object. There is one view: the
 * general keys are rows beside the policy keys, so `get`, `winner` and `eachFromHighest` answer for
 * them too, and `merged` is the whole of settings.md, "Settings precedence" and "Lists merge instead
 * of overriding" for a host that wants an object rather than a key.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/general/view.rs`, docs/RUST.md).
 */
export interface GeneralView extends PolicyView {
  /**
   * What Claude Code applies, key by key: every key with a row as its row combines it, every other
   * key by the general rule (blocks merge, lists union, a single value comes from the highest file).
   */
  merged(): Settings;
}

const call = coreModule('settings/general/view');

export const createGeneralView = (layers: readonly PolicyLayer[]): GeneralView =>
  call('createGeneralView', layers);
