/**
 * hooks.md, "JSON output": "If your string contains a `<system-reminder>` or `</system-reminder>`
 * tag, Claude receives the string with that tag's `<` replaced by `&lt;`." The context of a
 * foreground hook and of a background one go through this one replacement.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/reminder_tags.rs`, docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/reminder-tags');

export const escapeReminderTags = (text: string): string => call('escapeReminderTags', text);
