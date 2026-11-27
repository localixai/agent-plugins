/**
 * Where a skill asks for dynamic context (skills.md, "Inject dynamic context"): the inline form
 * `` !`<command>` `` and a block opened with ```` ```! ````. The scan runs on the original text, so
 * a command's output, an argument or a variable can never add a placeholder: "Substitution runs
 * once over the original file ... a command cannot emit a placeholder for a later pass to expand".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/shell_scan.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/shell-scan');

export type InjectionKind = 'inline' | 'fence';

export interface Injection {
  readonly kind: InjectionKind;
  /** Offsets in the scanned text; `[start, end)` is the whole placeholder. */
  readonly start: number;
  readonly end: number;
  /** What the shell runs, as written: a block's lines without the fences. */
  readonly command: string;
  /** The text before and after the command, kept to put a placeholder back as it was written. */
  readonly open: string;
  readonly close: string;
}

/** The placeholders of `content`, in order, without overlap. */
export const findInjections = (content: string): readonly Injection[] =>
  call('findInjections', content);
