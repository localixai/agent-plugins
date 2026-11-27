/**
 * A settings handler written in several settings files runs once (hooks.md, "Hook handler fields").
 * Judgment (spec/judgments.md): identical means the same event, matcher text and handler fields.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/select/dedupe.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';
import type { RegisteredHandler } from '../contracts.ts';

const call = coreModule('hooks/select/dedupe');

/** The entries with each settings handler kept once, the first in registry order. */
export const dedupeSettingsHandlers = (
  entries: readonly RegisteredHandler[],
): readonly RegisteredHandler[] => call('dedupeSettingsHandlers', entries);
