import { MOD_LIMITS } from '../../../protocol/mods-limits.ts';
import { firstProblem, textOf, type ElementDef } from './def.ts';
import type { ElementNode } from './node.ts';
import { isRecord } from '../../../protocol/is-record.ts';

/** `values`: "Finite numbers ... 8 at most" (`ShaderProps`, claude-code/index.d.ts:15868 of 2.1.330). */
const SHADER_VALUES = 8;

function valuesProblem(values: unknown): string | undefined {
  if (values === undefined) return undefined;
  const numbers = isRecord(values) ? Object.values(values) : [];
  const fits =
    isRecord(values) &&
    numbers.length <= SHADER_VALUES &&
    numbers.every((value) => typeof value === 'number' && Number.isFinite(value));
  return fits ? undefined : `Shader prop "values" holds at most ${SHADER_VALUES} finite numbers`;
}

/**
 * `Shader`: a scene the Desktop app plays on the GPU over exactly one `Svg`, in a `Backdrop` only
 * (Agent Plugins Extended §10.10, Judgment J10.24). Where it is placed is checked over the tree
 * (`shaderProblem`); the file `module` names is the host's to read.
 */
export const SHADER: ElementDef = {
  name: 'Shader',
  props: ['module', 'values'],
  children: 'any',
  check: (props) =>
    firstProblem(
      textOf('Shader', props, 'module', MOD_LIMITS.fieldChars),
      valuesProblem(props.values),
    ),
};

/** Whether `node` is a `Shader`. */
export const isShader = (node: ElementNode): boolean => node.type === SHADER.name;
