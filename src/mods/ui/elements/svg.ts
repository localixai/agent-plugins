import { firstProblem, textOf, type ElementDef } from './def.ts';
import type { ElementNode } from './node.ts';

/** `Svg` takes "a `source` of SVG markup up to 131,072 characters" (reference.md, "Elements"). */
const SVG_SOURCE_CHARS = 131_072;

export const SVG: ElementDef = {
  name: 'Svg',
  props: ['source', 'alt', 'width', 'height', 'isInteractive'],
  children: 'none',
  check: (props) =>
    firstProblem(textOf('Svg', props, 'source', SVG_SOURCE_CHARS), textOf('Svg', props, 'alt')),
};

/** Whether `node` is an `Svg`. */
export const isSvg = (node: ElementNode): boolean => node.type === SVG.name;
