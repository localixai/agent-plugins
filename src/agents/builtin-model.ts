/**
 * The model a built-in subagent runs on when the model order of "Choose a model" ends at the main
 * conversation's model (sub-agents.md, "Built-in subagents"). Only Explore has a rule: "When the main
 * conversation runs Fable, Explore's model depends on how you connect".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/agents/builtin_model.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('agents/builtin-model');

/** How the session reaches Claude, as far as the Explore rule tells the ways apart. */
export const CONNECTIONS: readonly [
  'subscription',
  'console',
  'llm-gateway',
  'bedrock',
  'vertex',
  'foundry',
  'claude-platform-on-aws',
  'claude-apps-gateway',
] = call('CONNECTIONS');
export type Connection = (typeof CONNECTIONS)[number];

/** A built-in, and the connection it is run over. */
export interface BuiltinModelInput {
  readonly name: string;
  readonly connection: Connection;
}

/** The alias a built-in's rule picks, or `undefined` when it follows the main conversation's model. */
export const builtinModelAlias = (
  input: BuiltinModelInput | undefined,
  mainFamily: string | undefined,
): string | undefined => call('builtinModelAlias', input, mainFamily);
