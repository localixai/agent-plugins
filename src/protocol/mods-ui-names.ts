/**
 * Implemented by the Rust core (`crates/agent-plugins/src/protocol/mods_ui_names.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';
import type { Assert, Equal } from './type-equality.ts';

const call = coreModule('protocol/mods-ui-names');

/**
 * What the mods reference says about the interface, as data (plugins/mods/reference.md, "Render
 * sites" and "Elements"): the render sites with the fields of `e.props`, what `e.requestId` is and the
 * apps that raise them; the elements with the apps that draw them, the props the VS Code extension
 * ignores, the `Box` border styles and spacing steps, and the `Text` values. Pure data.
 */
export const UI_SURFACES: readonly ['terminal', 'desktop', 'vscode', 'mobile'] =
  call('UI_SURFACES');

export type UiSurface = (typeof UI_SURFACES)[number];

/** The apps of the elements table: its three columns. */
export const ELEMENT_COLUMNS: readonly ['terminal', 'desktop', 'vscode'] = call('ELEMENT_COLUMNS');

export type ElementColumn = (typeof ELEMENT_COLUMNS)[number];

/**
 * The column of the elements table each app draws by. The table has no column for the mobile app,
 * which the docs name only as an app that raises `Backdrop`; the kit takes it to draw what the
 * Desktop app draws, with the same props, but no `Client` (`drawsElement`; judgment,
 * spec/judgments.md).
 */
export const ELEMENT_COLUMN_OF: { readonly [S in UiSurface]: ElementColumn } =
  call('ELEMENT_COLUMN_OF');

/** What `e.requestId` of a `ui.render` event is. */
export const REQUEST_KINDS: readonly [
  'pane-id',
  'instance',
  'message-id',
  'tool-call-id',
  'agent-id',
  'placed-id',
  'backdrop',
] = call('REQUEST_KINDS');

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

/** `e.requestId` of the `Backdrop` site: one backdrop for each session. */
export const BACKDROP_REQUEST_ID: 'backdrop' = call('BACKDROP_REQUEST_ID');

export const RENDER_SITES: {
  readonly Pane: {
    readonly props: readonly ['title', 'isFocused', 'bodyColumns', 'placement', 'scroll', 'view'];
    readonly request: 'pane-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly AbovePrompt: {
    readonly props: readonly ['hasSurvey', 'isWorking', 'maxRows', 'bodyColumns', 'scroll', 'view'];
    readonly request: 'instance';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly Placed: {
    readonly props: readonly ['id', 'plugin', 'onScreen'];
    readonly request: 'placed-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly Backdrop: {
    readonly props: readonly ['id', 'plugin', 'glass', 'colorScheme'];
    readonly request: 'backdrop';
    readonly surfaces: readonly UiSurface[];
  };
  readonly UserMessage: {
    readonly props: readonly ['text', 'origin', 'isExpanded', 'task', 'from'];
    readonly request: 'message-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly AssistantMessage: {
    readonly props: readonly [];
    readonly request: 'message-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly ToolUse: {
    readonly props: readonly [];
    readonly request: 'tool-call-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly ToolResult: {
    readonly props: readonly [];
    readonly request: 'tool-call-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly ToolGroup: {
    readonly props: readonly [];
    readonly request: 'tool-call-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly CommandOutput: {
    readonly props: readonly ['command', 'text'];
    readonly request: 'message-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly AskUserQuestion: {
    readonly props: readonly [];
    readonly request: 'tool-call-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly ToolProgress: {
    readonly props: readonly ['kind'];
    readonly request: 'tool-call-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly Spinner: {
    readonly props: readonly ['word', 'message', 'suffix', 'mode'];
    readonly request: 'agent-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly TurnDuration: {
    readonly props: readonly ['word', 'durationMs'];
    readonly request: 'message-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly InfoNotice: {
    readonly props: readonly ['text', 'command'];
    readonly request: 'message-id';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly SessionMode: {
    readonly props: readonly ['modes'];
    readonly request: 'instance';
    readonly surfaces: readonly ElementColumn[];
  };
  readonly PromptHint: {
    readonly props: readonly ['isDraft', 'isWorking', 'hint'];
    readonly request: 'instance';
    readonly surfaces: readonly ElementColumn[];
  };
} = call('RENDER_SITES');

export type RenderSite = keyof typeof RENDER_SITES;

/** The sites, in the order of the reference table. */
export const RENDER_SITE_NAMES: readonly [
  'Pane',
  'AbovePrompt',
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
] = call('RENDER_SITE_NAMES');

/** @public Drift guard: the list names every key of the table. */
export type RenderSiteNamesMatch = Assert<Equal<(typeof RENDER_SITE_NAMES)[number], RenderSite>>;

export const ELEMENT_NAMES: readonly [
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
] = call('ELEMENT_NAMES');

export type ElementName = (typeof ELEMENT_NAMES)[number];

/** The apps that draw each element (the check marks of the elements table). */
export const ELEMENT_SURFACES: { readonly [N in ElementName]: readonly ElementColumn[] } =
  call('ELEMENT_SURFACES');

/** Whether `surface` draws the element `name`: its column's check mark, less what the app leaves out. */
export const drawsElement = (name: ElementName, surface: UiSurface): boolean =>
  call('drawsElement', name, surface);

/**
 * The props the VS Code extension ignores: it draws the element as if they were left out
 * (reference.md, "Elements"). The terminal and the Desktop app ignore none the docs name.
 */
export const VSCODE_IGNORED: { readonly [N in ElementName]: readonly string[] } =
  call('VSCODE_IGNORED');

/** `Box` `borderStyle` names; any other value draws no border (reference.md, "`Box` border styles"). */
export const BORDER_STYLES: readonly [
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
] = call('BORDER_STYLES');

export type BorderStyle = (typeof BORDER_STYLES)[number];

/** What each spacing step is, in terminal cells (reference.md, "`Box` spacing steps"). */
export const SPACING_STEPS: {
  readonly xs: { readonly across: 1; readonly down: 0 };
  readonly sm: { readonly across: 1; readonly down: 0 };
  readonly md: { readonly across: 2; readonly down: 1 };
  readonly lg: { readonly across: 3; readonly down: 1 };
  readonly xl: { readonly across: 4; readonly down: 2 };
} = call('SPACING_STEPS');

export type SpacingStep = keyof typeof SPACING_STEPS;

export const FLEX_DIRECTIONS: readonly ['row', 'column', 'row-reverse', 'column-reverse'] =
  call('FLEX_DIRECTIONS');

export const TEXT_WRAPS: readonly [
  'wrap',
  'truncate',
  'truncate-start',
  'truncate-middle',
  'truncate-end',
] = call('TEXT_WRAPS');

export const TEXT_SIZES: readonly ['caption', 'footnote', 'body', 'heading', 'title'] =
  call('TEXT_SIZES');

export const TEXT_WEIGHTS: readonly ['regular', 'medium', 'semibold', 'bold'] =
  call('TEXT_WEIGHTS');

export const BUTTON_VARIANTS: readonly ['primary', 'secondary'] = call('BUTTON_VARIANTS');

export const BUTTON_ROLES: readonly ['dismiss'] = call('BUTTON_ROLES');
