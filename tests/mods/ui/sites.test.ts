/**
 * The render sites.
 * plugins/mods/reference.md § Render sites (each site's `e.props`, `e.requestId` and the apps that
 * raise it; `e.viewport`); plugins/mods/interface.md § Change what Claude Code already draws.
 * When Claude Code raises a site is the host's: the kit proves the table it holds equals the
 * reference's, which apps raise each site, and that a hook receives the documented fields. Rows
 * that describe their props in words ("The reply's text", "The tool's name, input, and result")
 * name no field, so for those sites the test proves the component, the app and the request id only
 * (spec/judgments.md). What is proven is the table (a transcription check against the docs), which
 * apps raise each site, which sites carry a request id, and that a hook can rewrite `e.props`
 * through `next`. The props themselves are the host's: the kit does not model that `UserMessage`
 * carries `task` or `from` by origin, nor the fields of the `Pane` `view` and `scroll` props.
 */
import { expect, test } from 'bun:test';
import {
  RENDER_SITE_NAMES,
  RENDER_SITES,
  UI_SURFACES,
  type RenderSite,
} from '../../../src/protocol/mods-ui-names.ts';
import { decideSite, renderEvent } from '../../../src/mods/ui/index.ts';
import { fn } from '../../support/mods-chain.ts';
import { MODS_REFERENCE } from '../../support/mods-reference.ts';
import { recordOf, uiChain } from '../../support/mods-ui.ts';
import { spec } from '../../support/spec.ts';

type Row = (typeof MODS_REFERENCE.renderSites)[number];

/** The sites with a row in the documentation's table: `ContextMenu` is the declarations' alone (J10.24). */
type DocumentedSite = Exclude<RenderSite, 'ContextMenu'>;
const DOCUMENTED = RENDER_SITE_NAMES.filter(
  (site): site is DocumentedSite => site !== 'ContextMenu',
);

const PROSE_SITES: Partial<Record<RenderSite, string>> = {
  AgentRow: 'mods.interface.Change what Claude Code already draws.AgentRow',
  AskUserQuestion: 'mods.interface.Change what Claude Code already draws.AskUserQuestion',
  AssistantMessage: 'mods.interface.Change what Claude Code already draws.AssistantMessage',
  CommandOutput: 'mods.interface.Change what Claude Code already draws.CommandOutput',
  InfoNotice: 'mods.interface.Change what Claude Code already draws.InfoNotice',
  PromptHint: 'mods.interface.Change what Claude Code already draws.PromptHint',
  SessionMode: 'mods.interface.Change what Claude Code already draws.SessionMode',
  Spinner: 'mods.interface.Change what Claude Code already draws.Spinner',
  ToolGroup: 'mods.interface.Change what Claude Code already draws.ToolGroup',
  ToolProgress: 'mods.interface.Change what Claude Code already draws.ToolProgress',
  ToolResult: 'mods.interface.Change what Claude Code already draws.ToolResult',
  ToolUse: 'mods.interface.Change what Claude Code already draws.ToolUse',
  TurnDuration: 'mods.interface.Change what Claude Code already draws.TurnDuration',
  UserMessage: 'mods.interface.Change what Claude Code already draws.UserMessage',
};

/**
 * The `e.requestId` cells the snapshot reads as no kind it knows, as the page writes them: "The
 * drawn row's id" and "The notice's `text` prefixed with `notice:`" (reference.md, "Render sites").
 */
const WRITTEN_REQUESTS: Partial<Record<RenderSite, string>> = {
  UserMessage: 'row-id',
  AssistantMessage: 'row-id',
  CommandOutput: 'row-id',
  TurnDuration: 'row-id',
  InfoNotice: 'notice',
};

/** The names a row's note gives as the spellings before v2.1.323: "Before v2.1.323, `scene` was `id` and `overrideColorScheme` was `colorScheme`". */
const FORMER_SPELLINGS: Partial<Record<RenderSite, readonly string[]>> = {
  Backdrop: ['id', 'colorScheme'],
};

const rows = (): readonly Row[] => MODS_REFERENCE.renderSites;

/** The row a site leads, else the row that names it among others (`ToolUse`, `ToolResult`, `ToolGroup`). */
const siteRow = (site: RenderSite): Row => {
  const row =
    rows().find((candidate) => candidate.sites[0] === site) ??
    rows().find((candidate) => candidate.sites.includes(site));
  if (!row) throw new Error(`no row for ${site}`);
  return row;
};

/**
 * The fields a row's `e.props` cell names: a site's name stands for that site's props ("plus the
 * `AbovePrompt` props"), words that are not a field (`{ plugin, tree }`, `$.agent.list()`) are left
 * out, and so are the spellings of before v2.1.323.
 */
function fieldsOf(site: RenderSite): readonly string[] {
  const former = FORMER_SPELLINGS[site] ?? [];
  const fields = siteRow(site).props.flatMap((token) => {
    const named = RENDER_SITE_NAMES.find((name) => name === token && name !== site);
    if (named !== undefined) return siteRow(named).props;
    return /^[A-Za-z]\w*$/.test(token) && !former.includes(token) ? [token] : [];
  });
  return [...new Set(fields)];
}

test('the sites the kit holds are the sites of the reference table, 19 in all, and the declarations-only ContextMenu', () => {
  expect([...new Set(rows().flatMap((row) => row.sites))]).toEqual([...DOCUMENTED]);
  expect(DOCUMENTED).toHaveLength(19);
  expect(RENDER_SITE_NAMES.at(-1)).toBe('ContextMenu');
});

for (const site of DOCUMENTED) {
  const prose = PROSE_SITES[site];
  spec([`mods.site.${site}`, ...(prose ? [prose] : [])], () => {
    test('the table holds the site as the reference does: its props, its request id and the apps that raise it', () => {
      const row = siteRow(site);
      expect<readonly string[]>([...RENDER_SITES[site].props]).toEqual(fieldsOf(site));
      expect<string | undefined>(RENDER_SITES[site].request).toBe(
        row.request ?? WRITTEN_REQUESTS[site],
      );
      expect<readonly string[]>([...RENDER_SITES[site].surfaces]).toEqual(row.surfaces);
    });

    test('the site is raised in the apps the table names and not in the others', () => {
      const listed: readonly string[] = siteRow(site).surfaces;
      for (const surface of UI_SURFACES) {
        const raised = listed.includes(surface);
        expect(decideSite(site, surface)).toBe(raised);
        const built = renderEvent({
          site,
          surface,
          requestId: 'id-1',
          viewport: { columns: 80, rows: 24, isFullscreen: true },
        });
        expect('event' in built).toBe(raised);
      }
    });

    test('a hook receives the component, the app and the request id, and what it passes to next as e.props reaches Claude Code`s drawing', async () => {
      const surface = RENDER_SITES[site].surfaces[0] ?? 'terminal';
      const built = renderEvent({
        site,
        surface,
        props: { label: 'host' },
        requestId: 'id-1',
        viewport: { columns: 80, rows: 24, isFullscreen: true },
      });
      if (!('event' in built)) throw new Error(built.problem);
      const seen: unknown[] = [];
      const h = uiChain();
      h.mod('my-mod', 'user', [
        fn(
          'ui.render',
          (_$, e, next) => (
            seen.push(e),
            next({ ...e, props: { ...recordOf(e.props), label: 'hook' } })
          ),
        ),
      ]);
      const settled = await h.fire('ui.render', built.event, {
        core: (e) => Promise.resolve({ type: 'engine', ref: e }),
      });
      expect(seen).toMatchObject([{ component: site, surface }]);
      // `label` is a prop every site lets a hook rewrite, `AgentRow`'s read-only ones aside (§10.9 item 5).
      expect(settled).toMatchObject({ ref: { component: site, props: { label: 'hook' } } });
      // Every instance carries the id the host gave (claude-code/index.d.ts:10629), one-instance sites too.
      expect(seen).toMatchObject([{ requestId: 'id-1' }]);
    });
  });
}

spec('mods.site.Pane', () => {
  test('a pane request needs the pane id, the id its hook checks (e.requestId)', () => {
    expect(renderEvent({ site: 'Pane', surface: 'terminal' })).toEqual({
      problem: 'Pane needs a requestId',
    });
  });
});

spec('mods.site.InfoNotice', () => {
  test("a notice's request id is its text led by notice:", () => {
    expect(
      renderEvent({ site: 'InfoNotice', surface: 'terminal', props: { text: 'Update ready' } }),
    ).toMatchObject({ event: { requestId: 'notice:Update ready' } });
  });
});

spec('mods.site.UserMessage', () => {
  test("a drawn row's site needs the row's id", () => {
    expect(renderEvent({ site: 'UserMessage', surface: 'terminal' })).toEqual({
      problem: 'UserMessage needs a requestId',
    });
  });
});

test('ContextMenu, declarations only, is raised in the Desktop app alone and needs the menu (J10.24)', () => {
  for (const surface of UI_SURFACES) {
    expect(decideSite('ContextMenu', surface), surface).toBe(surface === 'desktop');
  }
  expect(RENDER_SITES.ContextMenu.props).toEqual(['row', 'link', 'selection', 'code']);
  expect(renderEvent({ site: 'ContextMenu', surface: 'desktop' })).toEqual({
    problem: 'ContextMenu needs a requestId',
  });
});

test('e.viewport is absent until the terminal or the Desktop app measured its window, and always present in the VS Code extension', () => {
  const measured = { columns: 100, rows: 30, isFullscreen: false };
  for (const surface of ['terminal', 'desktop'] as const) {
    const before = renderEvent({ site: 'AbovePrompt', surface });
    expect(before).toMatchObject({ event: { component: 'AbovePrompt' } });
    expect('event' in before && 'viewport' in before.event).toBe(false);
    const after = renderEvent({ site: 'AbovePrompt', surface, viewport: measured });
    expect(after).toMatchObject({ event: { viewport: measured } });
  }
  expect(renderEvent({ site: 'Pane', surface: 'vscode', requestId: 'p' })).toEqual({
    problem: 'the VS Code extension always has a viewport',
  });
  expect(
    renderEvent({ site: 'Pane', surface: 'vscode', requestId: 'p', viewport: measured }),
  ).toMatchObject({
    event: { viewport: measured },
  });
});

spec('mods.site.AgentRow', () => {
  const row = (props: Record<string, unknown>) => ({
    component: 'AgentRow' as const,
    surface: 'terminal' as const,
    requestId: 'agent-1',
    props,
  });

  test('a hook changes label, body and isHidden; what the last next passed decides', async () => {
    const h = uiChain();
    h.mod('my-mod', 'user', [
      fn('ui.render', (_$, e, next) =>
        next({ ...e, props: { ...recordOf(e.props), label: 'mine', body: '', isHidden: true } }),
      ),
    ]);
    const settled = await h.fire('ui.render', row({ label: 'Explore', rowIds: ['a'] }), {
      core: (e) => Promise.resolve({ type: 'engine', ref: e }),
    });
    expect(h.reports).toEqual([]);
    expect(settled).toMatchObject({
      ref: { props: { label: 'mine', body: '', isHidden: true, rowIds: ['a'] } },
    });
  });

  test('a rewrite of any other prop is refused and the hook fails', async () => {
    const h = uiChain();
    h.mod('my-mod', 'user', [
      fn('ui.render', (_$, e, next) => next({ ...e, props: { ...recordOf(e.props), rowIds: [] } })),
    ]);
    await h.fire('ui.render', row({ label: 'Explore', rowIds: ['a'] }), {
      core: (e) => Promise.resolve({ type: 'engine', ref: e }),
    });
    expect(h.reports[0]?.line).toContain('AgentRow prop "rowIds" is read-only');
  });
});
