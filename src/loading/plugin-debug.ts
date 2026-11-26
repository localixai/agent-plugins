/**
 * The checks of "Test and debug" (plugins/create.md): "When a change to your plugin doesn't show up,
 * work through these checks in order. Each one tells you what Claude Code did with the plugin." The
 * list only; running the commands is the cli stream's and the host's.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/plugin_debug.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('loading/plugin-debug');

/** Where the command is typed: "In your shell" or "in the running session". */
export type DebugPlace = 'shell' | 'session';

export interface DebugStep {
  readonly place: DebugPlace;
  readonly command: string;
  /** What the step tells about the plugin. */
  readonly tells: string;
}

export interface DebugFacts {
  /** The plugin sits in a skills directory (`~/.claude/skills/` or a project's `.claude/skills/`). */
  readonly inSkillsDirectory: boolean;
  /** The plugin's folder, for a plugin that is not installed: `claude plugin list` takes it as `--plugin-dir`. */
  readonly pluginDir?: string | undefined;
}

/** The ordered checks; the last applies only to a plugin in a skills directory. */
export const pluginDebugSteps = (facts: DebugFacts): readonly DebugStep[] =>
  call('pluginDebugSteps', facts);

/** The checks of the other components: `/mcp` for a server, the debug log for a hook. */
export const COMPONENT_CHECKS: {
  readonly mcpServer: DebugStep;
  readonly hook: DebugStep;
} = call('COMPONENT_CHECKS');
