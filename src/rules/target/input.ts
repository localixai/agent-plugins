/**
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/target/input.rs`, docs/RUST.md).
 */
import type { AgentInput, ArtifactInput, WebFetchInput } from '../../protocol/official-tools.ts';
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/target/input');

/**
 * The input fields the target matchers read: the SDK's own field names, so a rename in `AgentInput`,
 * `WebFetchInput` or `ArtifactInput` stops compiling here (E2). `skill` is the one the SDK has no
 * input type for (permissions.md, "Match by input parameter": `Skill(skill:<name>)`).
 */
type TargetField = keyof AgentInput | keyof WebFetchInput | keyof ArtifactInput | 'skill';

/** A string field of a tool's input, or `undefined` when the input has no such string. */
export const inputString = (input: unknown, field: TargetField): string | undefined =>
  call('inputString', input, field);
