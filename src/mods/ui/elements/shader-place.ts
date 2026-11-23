import type { RenderSite } from '../../../protocol/mods-ui-names.ts';
import { isBox } from './box.ts';
import { isElementNode, walkTree, type ElementNode } from './node.ts';
import { isShader } from './shader.ts';
import { isSvg } from './svg.ts';

/** The one site a `Shader` draws at. */
const BACKDROP: RenderSite = 'Backdrop';

/** Why one `Shader` is out of place, or `undefined`: its site, its parent, its one `Svg`. */
function placeProblem(
  site: RenderSite,
  shader: ElementNode,
  parent: ElementNode | undefined,
): string | undefined {
  if (site !== BACKDROP) return 'Shader draws only in a Backdrop';
  if (parent !== undefined && !isBox(parent)) return 'a Shader sits at the root or in a Box';
  const [only, ...rest] = shader.children;
  const oneSvg = rest.length === 0 && isElementNode(only) && isSvg(only);
  return oneSvg ? undefined : 'Shader holds exactly one Svg';
}

/**
 * Why the `Shader`s of a tree are refused, or `undefined`: "A `Shader` draws only in a `Backdrop`,
 * at most one, at the root or in a `Box`; anywhere else, or over anything but one `Svg`, the tree is
 * refused" (Agent Plugins Extended §10.10, Judgment J10.24). The wording is the kit's.
 */
export function shaderProblem(site: RenderSite, tree: unknown): string | undefined {
  let count = 0;
  for (const { node, parent } of walkTree(tree)) {
    if (!isElementNode(node) || !isShader(node)) continue;
    count += 1;
    if (count > 1) return 'a Backdrop holds at most one Shader';
    const found = placeProblem(site, node, parent);
    if (found !== undefined) return found;
  }
  return undefined;
}
