import type { RenderRequest } from '../sites/registry.ts';
import type { PaneRegistry } from './registry.ts';

/**
 * A `Pane` request as the kit completes it from the pane the mod opened: `plugin` names the mod
 * whose pane it is (§10.13.3), and in the Desktop app, the one surface with widgets, a pane opened
 * with `placement: 'widget'` says so in its props (§10.25.1, §10.9). Other requests pass unchanged.
 */
export function paneRequest(panes: PaneRegistry, request: RenderRequest): RenderRequest {
  if (request.site !== 'Pane') return request;
  const pane = panes.list().find((open) => open.id === request.requestId);
  if (pane === undefined) return request;
  const widget = request.surface === 'desktop' && pane.request.placement === 'widget';
  return {
    ...request,
    plugin: pane.plugin,
    ...(widget ? { props: { ...request.props, placement: 'widget' } } : {}),
  };
}
