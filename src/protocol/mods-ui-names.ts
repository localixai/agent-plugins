import type { Assert, Equal } from './type-equality.ts';

/**
 * What the mods reference says about the interface, as data (plugins/mods/reference.md, "Render
 * sites" and "Elements"): the render sites with the fields of `e.props`, what `e.requestId` is and the
 * apps that raise them; the elements with the apps that draw them, the props the VS Code extension
 * ignores, the `Box` border styles and spacing steps, and the `Text` values. Pure data.
 */
export const UI_SURFACES = ['terminal', 'desktop', 'vscode', 'mobile'] as const;
export type UiSurface = (typeof UI_SURFACES)[number];

/** The apps of the elements table: its three columns. */
export const ELEMENT_COLUMNS = ['terminal', 'desktop', 'vscode'] as const;
export type ElementColumn = (typeof ELEMENT_COLUMNS)[number];

/**
 * The column of the elements table each app draws by. The table has no column for the mobile app,
 * which the docs name only as an app that raises `Backdrop`; the kit takes it to draw what the
 * Desktop app draws, with the same props, but no `Client` (`drawsElement`; judgment,
 * spec/judgments.md).
 */
export const ELEMENT_COLUMN_OF: { readonly [S in UiSurface]: ElementColumn } = {
  terminal: 'terminal',
  desktop: 'desktop',
  vscode: 'vscode',
  mobile: 'desktop',
};

/** What `e.requestId` of a `ui.render` event is. */
export const REQUEST_KINDS = [
  'pane-id',
  'instance',
  /** "The drawn row's id" (2.1.330; before, "the message id"). */
  'row-id',
  'tool-call-id',
  'agent-id',
  'placed-id',
  'backdrop',
  /** "The notice's `text` prefixed with `notice:`": derived from `e.props.text` when the host gives none. */
  'notice',
  /** `ContextMenu`'s "the menu" (declarations only, Judgment J10.24). */
  'menu',
] as const;
export type RequestKind = (typeof REQUEST_KINDS)[number];

export interface RenderSiteDef {
  /**
   * The fields of `e.props` the table names. A row that describes its props in words ("The
   * reply's text", "The tool's name, input, and result") names none: the list is then empty
   * (judgment, spec/judgments.md).
   */
  readonly props: readonly string[];
  readonly request: RequestKind;
  readonly surfaces: readonly UiSurface[];
}

const ALL: readonly ElementColumn[] = ['terminal', 'desktop', 'vscode'];
const TERMINAL_AND_DESKTOP: readonly ElementColumn[] = ['terminal', 'desktop'];
const TERMINAL: readonly ElementColumn[] = ['terminal'];
/** The Desktop app and the mobile app: the only apps that raise `Backdrop`. */
const DESKTOP_AND_MOBILE: readonly UiSurface[] = ['desktop', 'mobile'];

/** The props of the band, each mod's row and the stack of rows alike (reference.md, "Render sites"). */
const BAND_PROPS = [
  'hasSurvey',
  'isCollapsed',
  'isWorking',
  'maxRows',
  'bodyColumns',
  'scroll',
  'view',
] as const;

/** `e.requestId` of the `Backdrop` site: one backdrop for each session. */
export const BACKDROP_REQUEST_ID = 'backdrop';

export const RENDER_SITES = {
  Pane: {
    props: ['title', 'isFocused', 'bodyColumns', 'placement', 'scroll', 'view'],
    request: 'pane-id',
    surfaces: ALL,
  },
  /** One mod's row of the band (2.1.323 or later): `e.plugin` names the mod whose row it is. */
  AbovePrompt: {
    props: BAND_PROPS,
    request: 'instance',
    surfaces: TERMINAL_AND_DESKTOP,
  },
  /** The stack of the band's rows, `rows` one `{ plugin, tree }` for each, plus the `AbovePrompt` props (2.1.323 or later). */
  AbovePromptList: {
    props: ['rows', ...BAND_PROPS],
    request: 'instance',
    surfaces: TERMINAL_AND_DESKTOP,
  },
  /** A row `$.ui.place` put in the transcript: its `requestId` is the one the call returned. */
  Placed: { props: ['id', 'plugin', 'onScreen'], request: 'placed-id', surfaces: TERMINAL },
  /** The session's backdrop (`$.theme.set`): its `requestId` is always `backdrop`. */
  Backdrop: {
    props: ['scene', 'plugin', 'glass', 'overrideColorScheme'],
    request: 'backdrop',
    surfaces: DESKTOP_AND_MOBILE,
  },
  UserMessage: {
    props: ['text', 'origin', 'isExpanded', 'task', 'from'],
    request: 'row-id',
    surfaces: ALL,
  },
  AssistantMessage: { props: [], request: 'row-id', surfaces: ALL },
  ToolUse: { props: [], request: 'tool-call-id', surfaces: ALL },
  ToolResult: { props: [], request: 'tool-call-id', surfaces: ALL },
  ToolGroup: { props: [], request: 'tool-call-id', surfaces: ALL },
  CommandOutput: {
    props: ['command', 'text'],
    request: 'row-id',
    surfaces: TERMINAL_AND_DESKTOP,
  },
  AskUserQuestion: { props: [], request: 'tool-call-id', surfaces: ALL },
  ToolProgress: { props: ['kind'], request: 'tool-call-id', surfaces: TERMINAL },
  Spinner: { props: ['word', 'message', 'suffix', 'mode'], request: 'agent-id', surfaces: ALL },
  TurnDuration: { props: ['word', 'durationMs'], request: 'row-id', surfaces: TERMINAL },
  InfoNotice: { props: ['text', 'command'], request: 'notice', surfaces: TERMINAL },
  SessionMode: { props: ['modes'], request: 'instance', surfaces: TERMINAL_AND_DESKTOP },
  PromptHint: {
    props: ['isDraft', 'isWorking', 'hint'],
    request: 'instance',
    surfaces: TERMINAL_AND_DESKTOP,
  },
  /**
   * One agent's row in the agent list (2.1.326 or later): the agent's `$.agent.list()` fields, plus
   * those the table names. The terminal, and the VS Code extension's Focus view.
   */
  AgentRow: {
    props: ['rowIds', 'columns', 'body', 'isHidden'],
    request: 'agent-id',
    surfaces: ['terminal', 'vscode'],
  },
  /** The Desktop app's menu on a transcript row; declarations only (Judgment J10.24, `RenderPropsOf.ContextMenu`, claude-code/index.d.ts:13771). */
  ContextMenu: {
    props: ['row', 'link', 'selection', 'code'],
    request: 'menu',
    surfaces: ['desktop'],
  },
} as const satisfies Readonly<Record<string, RenderSiteDef>>;

export type RenderSite = keyof typeof RENDER_SITES;
/** The sites, in the order of the reference table. */
export const RENDER_SITE_NAMES = [
  'Pane',
  'AbovePrompt',
  'AbovePromptList',
  'Placed',
  'Backdrop',
  'UserMessage',
  'AssistantMessage',
  'ToolUse',
  'ToolResult',
  'ToolGroup',
  'CommandOutput',
  'AskUserQuestion',
  'ToolProgress',
  'Spinner',
  'TurnDuration',
  'InfoNotice',
  'SessionMode',
  'PromptHint',
  'AgentRow',
  'ContextMenu',
] as const;
/** @public Drift guard: the list names every key of the table. */
export type RenderSiteNamesMatch = Assert<Equal<(typeof RENDER_SITE_NAMES)[number], RenderSite>>;

export const ELEMENT_NAMES = [
  'Box',
  'Text',
  'Button',
  'Link',
  'Code',
  'Markdown',
  'Input',
  'Select',
  'Svg',
  'Client',
  'Raster',
  'Image',
  // In the 2.1.330 declarations' per-surface tables, not in the documentation's (Judgment J10.24).
  'Progress',
  'Shader',
] as const;
export type ElementName = (typeof ELEMENT_NAMES)[number];

/** The apps that draw each element (the check marks of the elements table). */
export const ELEMENT_SURFACES: { readonly [N in ElementName]: readonly ElementColumn[] } = {
  Box: ALL,
  Text: ALL,
  Button: ALL,
  Link: ALL,
  Code: ALL,
  Markdown: ALL,
  Input: ALL,
  Select: ALL,
  Svg: ['desktop', 'vscode'],
  Client: TERMINAL_AND_DESKTOP,
  Raster: TERMINAL,
  Image: TERMINAL,
  Progress: ALL,
  // The Desktop app; the mobile app takes the Desktop column (claude-code/index.d.ts:4819-4891).
  Shader: ['desktop'],
};

/**
 * The elements an app leaves out though its column of the table draws them: the mobile app takes
 * the Desktop column but has no `Client` (the declarations' mobile `Elements` table,
 * claude-code/index.d.ts:4108; Agent Plugins Extended §10.27, Judgment J10.104).
 */
const NOT_DRAWN_BY: { readonly [S in UiSurface]: readonly ElementName[] } = {
  terminal: [],
  desktop: [],
  vscode: [],
  mobile: ['Client'],
};

/** Whether `surface` draws the element `name`: its column's check mark, less what the app leaves out. */
export const drawsElement = (name: ElementName, surface: UiSurface): boolean =>
  ELEMENT_SURFACES[name].includes(ELEMENT_COLUMN_OF[surface]) &&
  !NOT_DRAWN_BY[surface].includes(name);

/**
 * The props the VS Code extension ignores: it draws the element as if they were left out
 * (reference.md, "Elements"). The terminal and the Desktop app ignore none the docs name.
 */
export const VSCODE_IGNORED: { readonly [N in ElementName]: readonly string[] } = {
  Box: ['position', 'top', 'left', 'right', 'bottom', 'alignSelf', 'borderDimColor', 'hover'],
  Text: ['size', 'mono', 'tabular', 'hover'],
  Button: ['hotkey', 'plain', 'autoFocus', 'action', 'hover', 'variant', 'role'],
  Link: [],
  Code: ['language', 'path', 'startLine', 'format', 'wrap'],
  Markdown: [],
  Input: ['autoFocus', 'rows'],
  Select: ['autoFocus'],
  Svg: ['isInteractive'],
  Client: [],
  Raster: [],
  Image: [],
  Progress: [],
  Shader: [],
};

/** `Box` `borderStyle` names; any other value draws no border (reference.md, "`Box` border styles"). */
export const BORDER_STYLES = [
  'single',
  'double',
  'round',
  'bold',
  'singleDouble',
  'doubleSingle',
  'classic',
  'arrow',
  'dashed',
  'quote',
] as const;
export type BorderStyle = (typeof BORDER_STYLES)[number];

/** What each spacing step is, in terminal cells (reference.md, "`Box` spacing steps"). */
export const SPACING_STEPS = {
  xs: { across: 1, down: 0 },
  sm: { across: 1, down: 0 },
  md: { across: 2, down: 1 },
  lg: { across: 3, down: 1 },
  xl: { across: 4, down: 2 },
} as const satisfies Readonly<Record<string, { across: number; down: number }>>;
export type SpacingStep = keyof typeof SPACING_STEPS;

export const FLEX_DIRECTIONS = ['row', 'column', 'row-reverse', 'column-reverse'] as const;
/** `Text` `wrap` (claude-code/index.d.ts:16833 of 2.1.330: `end`, `middle` and `clip` from 2.1.326). */
export const TEXT_WRAPS = [
  'wrap',
  'end',
  'middle',
  'truncate',
  'truncate-start',
  'truncate-middle',
  'truncate-end',
  'clip',
] as const;
/** `Box` `overflowX` (claude-code/index.d.ts:1382 of 2.1.330, from 2.1.326). */
export const BOX_OVERFLOW_X = ['visible', 'scroll'] as const;
export const TEXT_SIZES = ['caption', 'footnote', 'body', 'heading', 'title'] as const;
export const TEXT_WEIGHTS = ['regular', 'medium', 'semibold', 'bold'] as const;
export const BUTTON_VARIANTS = ['primary', 'secondary'] as const;
export const BUTTON_ROLES = ['dismiss'] as const;
