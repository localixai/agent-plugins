/**
 * The shape of `spec/docs-tables.json`: the tables of the mods reference page
 * (plugins/mods/reference.md) as data, as Agent Plugins Extended publishes them. `bun run spec:sync`
 * copies the file from the spec; the mods tests read it through this schema.
 */
import { z } from 'zod';

const SURFACE_COLUMNS = ['terminal', 'desktop', 'vscode'] as const;
/** The apps a render site's "Rendered on" cell names: the three columns and the mobile app (`Backdrop`). */
const SITE_SURFACES = [...SURFACE_COLUMNS, 'mobile'] as const;

export const snapshotSchema = z.strictObject({
  /** § Events: the events of each group, in the page's order. */
  events: z.array(z.strictObject({ group: z.string(), events: z.array(z.string()) })),
  /** § Mods API methods: each namespace (`$.` removed) and its methods. */
  apiMethods: z.array(z.strictObject({ namespace: z.string(), methods: z.array(z.string()) })),
  /** The `status` values the `$.agent` row lists. */
  agentStatuses: z.array(z.string()),
  /**
   * § Limits: each row as a short label (the first words of its first cell) and the amounts its
   * second cell writes (`10 seconds`, `4 MiB`, `28,000 levels`). Facts and labels only, never the
   * page's sentences.
   */
  limits: z.array(z.strictObject({ limit: z.string(), value: z.string() })),
  /** § Elements: each element, the props its row names and the apps that draw it. */
  elements: z.array(
    z.strictObject({
      name: z.string(),
      props: z.array(z.string()),
      surfaces: z.array(z.enum(SURFACE_COLUMNS)),
    }),
  ),
  /** § Elements, "`Box` border styles". */
  borderStyles: z.array(z.string()),
  /** § Render sites: each row's sites, props, request and apps. */
  renderSites: z.array(
    z.strictObject({
      sites: z.array(z.string()),
      props: z.array(z.string()),
      request: z
        .enum([
          'pane-id',
          'instance',
          'message-id',
          'tool-call-id',
          'agent-id',
          'placed-id',
          'backdrop',
        ])
        .nullable(),
      surfaces: z.array(z.enum(SITE_SURFACES)),
    }),
  ),
  /** § Settings and environment variables: the names. */
  settings: z.array(z.string()),
});
