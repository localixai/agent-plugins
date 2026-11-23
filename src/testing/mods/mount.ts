import {
  isElementNode,
  routeControl,
  foldRender,
  renderEvent,
  type ElementNode,
} from '../../mods/ui/index.ts';
import { ENGINE_TYPE, walkTree } from '../../mods/ui/elements/node.ts';
import type { RenderSite, UiSurface } from '../../protocol/mods-ui-names.ts';
import type { World } from './world.ts';

/**
 * `$.ui.mount` of the test kit (plugins/mods/test.md, "Test a drawing"): it draws one of the mod's
 * render sites through its `ui.render` hook and returns a handle to press, type into, pick in and
 * find the elements it drew, by the `key` the mod gave them. It checks nothing about how an app paints
 * a tree: the tree is the element data the hook returned.
 */
export interface MountSpec {
  readonly plugin?: string;
  readonly component: RenderSite;
  readonly surface: UiSurface;
  readonly requestId?: string;
  readonly viewport?: {
    readonly columns: number;
    readonly rows: number;
    readonly isFullscreen?: boolean;
  };
  readonly props?: Readonly<Record<string, unknown>>;
}

export interface FindQuery {
  readonly key?: string;
  readonly type?: string;
  /** The element's text, which a string must equal and a regular expression must match. */
  readonly text?: string | RegExp;
}

export interface MountHandle {
  press(target: { readonly key: string }): Promise<void>;
  input(target: {
    readonly key: string;
    readonly text: string;
    readonly kind?: 'change';
  }): Promise<void>;
  select(target: { readonly key: string; readonly value: string }): Promise<void>;
  find(
    query: FindQuery,
  ): Promise<
    { readonly type: string; readonly props: unknown; readonly children: unknown } | undefined
  >;
  /** Whether the tree as last drawn holds what `next(e)` answered (claude-code/index.d.ts:21874). */
  keepsBeneath(): Promise<boolean>;
  unmount(): Promise<void>;
}

/** The text an element holds: its string children, and those of the elements inside it. */
function textOf(node: ElementNode): string {
  return node.children
    .map((child) => (typeof child === 'string' ? child : isElementNode(child) ? textOf(child) : ''))
    .join('');
}

function matches(node: ElementNode, query: FindQuery): boolean {
  if (query.key !== undefined && node.props.key !== query.key) return false;
  if (query.type !== undefined && node.type !== query.type) return false;
  if (query.text === undefined) return true;
  const text = textOf(node);
  return typeof query.text === 'string' ? text === query.text : query.text.test(text);
}

/** The first element of the tree that fits the query, in document order. */
function findIn(tree: ElementNode | undefined, query: FindQuery): ElementNode | undefined {
  if (!tree) return undefined;
  for (const { node } of walkTree(tree)) {
    if (isElementNode(node) && matches(node, query)) return node;
  }
  return undefined;
}

/**
 * What `next(e)` answers at the end of the chain: the test's `ui.render` stub, or, with none,
 * Claude Code's own drawing (test.md, "Follow the test kit's rules": "the mount succeeds with no
 * stub", from 2.1.321), as it is when no stub's matcher lets the event through (`stubAdmits`).
 */
function beneathOf(world: World, e: unknown): Promise<unknown> {
  if (!world.stubs.admits('ui.render', e)) {
    return Promise.resolve({ type: ENGINE_TYPE, ref: e });
  }
  // oxlint-disable-next-line typescript/no-unsafe-type-assertion -- `ui.render` is no stream event, so the stubs answer it with a promise
  return world.answer('ui.render', e) as Promise<unknown>;
}

/** Whether `settled` is, or holds at any depth, the value `next(e)` answered. */
const holds = (settled: unknown, answered: unknown): boolean =>
  answered !== undefined && [...walkTree(settled)].some(({ node }) => node === answered);

export async function mountUi(world: World, spec: MountSpec): Promise<MountHandle> {
  await world.ensure();
  const host = world.host();
  const request = {
    site: spec.component,
    surface: spec.surface,
    ...(spec.props === undefined ? {} : { props: spec.props }),
    ...(spec.requestId === undefined ? {} : { requestId: spec.requestId }),
    ...(spec.plugin === undefined ? {} : { plugin: spec.plugin }),
    ...(spec.viewport === undefined ? {} : { viewport: { isFullscreen: false, ...spec.viewport } }),
  };
  const built = renderEvent(request);
  if ('problem' in built) throw new Error(built.problem);
  let tree: ElementNode | undefined;
  /** What `next(e)` answered at the last drawing, and whether the drawing kept it. */
  let beneath: { readonly answered: unknown; isKept: boolean } | undefined;
  const draw = async (): Promise<void> => {
    world.needsRedraw = false;
    const answered: unknown[] = [];
    const settled = await host.fire('ui.render', built.event, {
      core: async (e) => {
        const answer = await beneathOf(world, e);
        answered.push(answer);
        return answer;
      },
    });
    beneath = { answered: answered[0], isKept: answered.some((one) => holds(settled, one)) };
    const drawn = foldRender(request.site, request.surface, settled);
    if ('refused' in drawn) throw new Error(drawn.line);
    tree = 'tree' in drawn ? drawn.tree : undefined;
  };
  await draw();
  const signal = new AbortController().signal;
  const gesture = async (
    kind: 'press' | 'submit' | 'change' | 'select',
    key: string,
    value?: unknown,
  ) => {
    const routed = await routeControl(
      {
        chain: host.chain,
        tree,
        surface: spec.surface,
        signal,
        report: (report) => world.reports.push(report),
      },
      kind,
      key,
      value,
    );
    const [first] = routed.issues;
    if (first) throw new Error(first.message);
    await draw();
  };
  return {
    press: ({ key }) => gesture('press', key),
    input: ({ key, text, kind }) => gesture(kind ?? 'submit', key, text),
    select: ({ key, value }) => gesture('select', key, value),
    async find(query) {
      if (world.needsRedraw) await draw();
      return findIn(tree, query);
    },
    async keepsBeneath() {
      if (world.needsRedraw) await draw();
      return beneath?.isKept ?? false;
    },
    unmount: () => {
      tree = undefined;
      return Promise.resolve();
    },
  };
}
