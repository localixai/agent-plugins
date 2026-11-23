import { MOD_LIMITS } from '../../../protocol/mods-limits.ts';
import type { UiSurface } from '../../../protocol/mods-ui-names.ts';
import { clipText, measureTree, type TreeSize } from '../elements/limits.ts';
import { isElementNode, type ElementNode, type Props } from '../elements/node.ts';
import { propSizeProblem } from '../elements/prop-size.ts';
import { drawnNode } from '../elements/surface.ts';
import { foldRender, refusal, type Drawn } from './fold.ts';
import type { RenderRequest } from './registry.ts';
import type { Viewport } from './viewport.ts';

/** One mod's row of the band, as `AbovePromptList`'s `rows` holds it (reference.md, "Render sites"). */
export interface BandRow {
  readonly plugin: string;
  readonly tree: ElementNode;
}

/** What the host gives to draw the band: the app, the `AbovePrompt` props, the measured window. */
export interface BandRequest {
  readonly surface: UiSurface;
  readonly props?: Props;
  readonly viewport?: Viewport;
}

export interface BandDrawn {
  /** The rows that settled and fit, in chain order: what `AbovePromptList` was handed. */
  readonly rows: readonly BandRow[];
  /** What the stack shows: Claude Code's stack of the rows the last `next(e)` passed, or a mod's tree. */
  readonly drawn: Drawn;
  /** The transcript lines: `hooks: ui.render (AbovePrompt) refused: a row past the limits of the band; nothing was drawn`. */
  readonly lines: readonly string[];
}

/** The reason of a row left out (troubleshoot.md, "A `ui.render` line says `refused`", 2.1.330). */
const PAST_THE_BAND = 'a row past the limits of the band; nothing was drawn';

const EMPTY: TreeSize = { depth: 0, nodes: 0, rasterCells: 0, imageBytes: 0 };

/** What the rows kept so far hold with `size` added, stacking adding one level of depth. */
const added = (sum: TreeSize, size: TreeSize): TreeSize => ({
  depth: Math.max(sum.depth, size.depth + 1),
  nodes: sum.nodes + size.nodes,
  rasterCells: sum.rasterCells + size.rasterCells,
  imageBytes: sum.imageBytes + size.imageBytes,
});

const fits = (size: TreeSize): boolean =>
  size.depth <= MOD_LIMITS.treeDepth &&
  size.nodes <= MOD_LIMITS.treeNodes &&
  size.rasterCells <= MOD_LIMITS.rasterCells &&
  size.imageBytes <= MOD_LIMITS.imageBytes;

/**
 * The rows kept from the top against the limits of one tree, counted across the rows (§10.9 item 4,
 * §10.11): a row past them is left out and the rows after it are still tried (judgment: "draws
 * nothing while the other rows draw"). Each row's text is cut at the text limit of its own.
 */
function stackRows(
  surface: UiSurface,
  candidates: readonly BandRow[],
): { readonly rows: readonly BandRow[]; readonly isCut: boolean } {
  const rows: BandRow[] = [];
  let sum = EMPTY;
  for (const row of candidates) {
    const next = added(sum, measureTree(row.tree));
    if (!fits(next) || propSizeProblem(row.tree) !== undefined) continue;
    sum = next;
    rows.push({ plugin: row.plugin, tree: drawnNode(surface, clipText(row.tree)) });
  }
  return { rows, isCut: rows.length < candidates.length };
}

/** How the band asks the chain for a site: the runtime's `ui.render` dispatch. */
export type SettleRender = (request: RenderRequest) => Promise<unknown>;

/**
 * Draws the band from 2.1.323 (§10.21 item 3, §10.9 item 4): `AbovePrompt` once for each mod that
 * hooks `ui.render`, in chain order, `e.plugin` naming it; the rows that settled on a tree and fit
 * are then handed to `AbovePromptList`, whose hooks may hide, reorder or wrap them.
 */
export async function drawBand(
  settle: SettleRender,
  request: BandRequest,
  asked: readonly string[],
): Promise<BandDrawn> {
  const { surface, viewport } = request;
  const props = request.props ?? {};
  const shared = { surface, ...(viewport === undefined ? {} : { viewport }) };
  const candidates: BandRow[] = [];
  for (const plugin of asked) {
    const tree = await settle({ site: 'AbovePrompt', ...shared, props, plugin });
    if (isElementNode(tree)) candidates.push({ plugin, tree });
  }
  const { rows, isCut } = stackRows(surface, candidates);
  const settled = await settle({ site: 'AbovePromptList', ...shared, props: { ...props, rows } });
  return {
    rows,
    drawn: foldRender('AbovePromptList', surface, settled, { mods: asked.length }),
    lines: isCut ? [refusal('AbovePrompt', PAST_THE_BAND, asked.length)] : [],
  };
}
