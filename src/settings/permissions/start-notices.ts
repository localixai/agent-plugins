/**
 * The notice a terminal session shows under the HIPAA configuration, where the built-in `auto` default
 * does not apply (permission-modes.md, "Permission modes with the HIPAA configuration", verbatim).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/settings/permissions/start_notices.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('settings/permissions/start-notices');

export const HIPAA_AUTO_NOTICE: "Auto mode isn't the default for your organization · Shift+Tab to switch" =
  call('HIPAA_AUTO_NOTICE');
