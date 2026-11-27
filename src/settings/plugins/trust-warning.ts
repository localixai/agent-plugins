/**
 * The trust warning `/plugin` shows before a plugin installs (settings-reference.md,
 * `pluginTrustMessage`; plugins/security.md, "Trust warning before you install").
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/plugins/trust_warning.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { PolicyView } from '../view.ts';

const call = coreModule('settings/plugins/trust-warning');

/**
 * The trust warning `/plugin` shows before a plugin installs (settings-reference.md,
 * `pluginTrustMessage`; plugins/security.md, "Trust warning before you install"). The published
 * warning is the default; a host can supply its own wording, unchanged and first.
 * Only managed settings carry the key. An empty or blank message adds nothing (a judgment).
 */
export const trustWarning = (view: PolicyView, standard?: string): string =>
  call('trustWarning', view, standard);
