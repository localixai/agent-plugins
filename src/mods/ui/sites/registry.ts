import {
  RENDER_SITES,
  type RenderSite,
  type RequestKind,
  type UiSurface,
} from '../../../protocol/mods-ui-names.ts';
import type { Props } from '../elements/node.ts';
import { viewportProblem, type Viewport } from './viewport.ts';

/** Whether `site` is raised in `surface`: the "Rendered on" column of the render sites table. */
export function decideSite(site: RenderSite, surface: UiSurface): boolean {
  const raisedIn: readonly UiSurface[] = RENDER_SITES[site].surfaces;
  return raisedIn.includes(surface);
}

/** What a `ui.render` hook receives as `e` (reference.md, "Render sites"). */
export type RenderEvent = {
  readonly component: RenderSite;
  readonly surface: UiSurface;
  readonly props: Props;
  /**
   * The instance (claude-code/index.d.ts:10629, `RenderInputOf.requestId`, declared on every site):
   * the pane's `id`, the drawn row's id, the tool call id, the agent id or a placed row's `requestId`;
   * for a site that has "One instance", the one the host gave, else the site's name (judgment).
   */
  readonly requestId: string;
  readonly viewport?: Viewport;
  /**
   * The mod whose pane or row of the band it is (2.1.323 or later; `plugin` is a default matcher
   * key, Agent Plugins Extended §10.13.3); absent at a site Claude Code draws itself.
   */
  readonly plugin?: string;
};

export interface RenderRequest {
  readonly site: RenderSite;
  readonly surface: UiSurface;
  readonly props?: Props;
  readonly requestId?: string;
  /** The window as the app measured it; absent until it has. */
  readonly viewport?: Viewport;
  /** The mod whose pane or band row it is, as `RenderEvent.plugin`. */
  readonly plugin?: string;
}

/** The id a request without one gets, or `undefined` when the site needs the host to give one. */
type DefaultId = (site: RenderSite, props: Props) => string | undefined;
const NEEDED: DefaultId = () => undefined;

/**
 * What each kind of `requestId` falls back to: an `instance` is the site's name (judgment), a notice
 * "the notice's `text` prefixed with `notice:`" (reference.md, "Render sites"); the others are the
 * host's to give.
 */
const REQUEST_IDS: { readonly [K in RequestKind]: DefaultId } = {
  'pane-id': NEEDED,
  'row-id': NEEDED,
  'tool-call-id': NEEDED,
  'agent-id': NEEDED,
  'placed-id': NEEDED,
  backdrop: NEEDED,
  menu: NEEDED,
  instance: (site) => site,
  notice: (_site, props) => (typeof props.text === 'string' ? `notice:${props.text}` : undefined),
};

/**
 * The `ui.render` event for one site, or why it is not raised: the site is not drawn in that app,
 * or the request lacks its id or (in the VS Code extension) its viewport. Firing is the host's:
 * the kit builds the event and proves what a hook receives.
 */
export function renderEvent(
  request: RenderRequest,
): { readonly event: RenderEvent } | { readonly problem: string } {
  const { site, surface, viewport } = request;
  const props = request.props ?? {};
  const requestId = request.requestId ?? REQUEST_IDS[RENDER_SITES[site].request](site, props);
  if (!decideSite(site, surface)) return { problem: `${site} is not raised in the ${surface} app` };
  if (requestId === undefined) return { problem: `${site} needs a requestId` };
  const missing = viewportProblem(surface, viewport);
  if (missing !== undefined) return { problem: missing };
  return {
    event: {
      component: site,
      surface,
      props,
      requestId,
      ...(viewport === undefined ? {} : { viewport }),
      ...(request.plugin === undefined ? {} : { plugin: request.plugin }),
    },
  };
}
