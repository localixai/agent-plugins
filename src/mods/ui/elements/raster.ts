import { firstProblem, sized, textOf, type ElementDef } from './def.ts';
import type { ElementNode, Props } from './node.ts';

/** A `Raster`'s `columns` go up to 512 and its `rows` up to 256 (reference.md, "Elements"). */
const RASTER_MAX_COLUMNS = 512;
const RASTER_MAX_ROWS = 256;

/** Base64 text: groups of four characters, the last with at most two `=` of padding. */
const BASE64 = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;
/** Each cell packs three unsigned 32-bit numbers: the code point, the colour, the background. */
const BYTES_PER_CELL = 12;

/** How many bytes base64 `text` decodes to, or `undefined` when it is not base64. */
function decodedBytes(text: string): number | undefined {
  if (!BASE64.test(text)) return undefined;
  const padding = text.endsWith('==') ? 2 : Number(text.endsWith('='));
  return (text.length / 4) * 3 - padding;
}

/**
 * `cells` decodes to exactly `columns × rows × 3` little-endian 32-bit numbers, or the tree does not
 * validate (Agent Plugins Extended §10.23.2, Judgment J10.105; the docs' example packs a
 * `Uint32Array`). Wording ours (judgment).
 */
function cellsProblem(props: Props): string | undefined {
  const { cells } = props;
  if (typeof cells !== 'string') return undefined;
  const expected = rasterCells(props) * BYTES_PER_CELL;
  return decodedBytes(cells) === expected
    ? undefined
    : `Raster prop "cells" must be base64 of exactly columns × rows × 3 unsigned 32-bit numbers (${String(expected)} bytes)`;
}

export const RASTER: ElementDef = {
  name: 'Raster',
  props: ['key', 'columns', 'rows', 'cells'],
  children: 'none',
  check: (props) =>
    firstProblem(
      sized('Raster', props, 'columns', RASTER_MAX_COLUMNS),
      sized('Raster', props, 'rows', RASTER_MAX_ROWS),
      textOf('Raster', props, 'cells'),
      cellsProblem(props),
    ),
};

/** Whether `node` is a `Raster`. */
export const isRaster = (node: ElementNode): boolean => node.type === RASTER.name;

/** The cells a `Raster` holds: `columns` times `rows`, 0 when either is not a number. */
export function rasterCells(props: Props): number {
  const { columns, rows } = props;
  return typeof columns === 'number' && typeof rows === 'number' ? columns * rows : 0;
}
