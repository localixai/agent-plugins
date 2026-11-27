import type { PluginIssue } from '../../format/issues.ts';
import type { ScratchFiles } from '../../ports/scratch-files.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('hooks/output/text-cap');

/**
 * The cap on a hook's text (hooks.md, "JSON output"): "A hook's `additionalContext`,
 * `systemMessage`, and `initialUserMessage` strings, and its plain stdout, are capped at 10,000
 * characters". Each string is measured on its own; over the limit, "Claude Code saves the output to
 * a file in the session directory. It replaces the output with the file path and a preview of up to
 * the first 2,000 characters." Except for PreCompact stdout, which is cut at the cap with a marker.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/hooks/output/text_cap.rs`, docs/RUST.md).
 */

export const TEXT_CAP: number = call('TEXT_CAP');
export const PREVIEW_LENGTH: number = call('PREVIEW_LENGTH');

export interface Limited {
  readonly text: string;
  readonly issues: readonly PluginIssue[];
}

/**
 * `text` as the model gets it: as is under the cap, else a pointer to the saved file with a
 * preview. Without a `ScratchFiles` port, or when the write fails, the text is cut at the cap with
 * a warning, so a host that cannot save a file never injects more than the cap.
 */
export const limitText = (
  name: string,
  text: string,
  scratch: ScratchFiles | undefined,
  signal: AbortSignal,
): Promise<Limited> => call.async('limitText', name, text, scratch, signal);
