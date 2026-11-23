import type { UiSurface } from '../../../protocol/mods-ui-names.ts';
import { isButton } from '../elements/button.ts';
import { isElementNode, walkTree, type ElementNode } from '../elements/node.ts';

export interface HotkeyContext {
  readonly surface: UiSurface;
  /** Whether the tree is the band above the prompt (digit hotkeys work there). */
  readonly isBand: boolean;
  /** Whether the user collapsed the band: "neither an `action` binding nor a hotkey presses a button in it". */
  readonly isCollapsed: boolean;
  /** Whether the pane or band has keyboard focus: "that happens only while your pane or band has keyboard focus. The rest of the time, keys go to the prompt". */
  readonly hasFocus: boolean;
  /** Whether an `Input` of the drawing has the focus: "every printable key goes to the field". */
  readonly isInputFocused: boolean;
  /**
   * The band's rows, each mod's tree (2.1.323 or later): given, hotkeys are read from them and not
   * from the tree, so two mods' rows on one key are told apart.
   */
  readonly rows?: readonly { readonly plugin: string; readonly tree: unknown }[] | undefined;
}

/** Buttons of a tree in document order. */
function buttons(tree: unknown): readonly ElementNode[] {
  return [...walkTree(tree)].flatMap(({ node }) =>
    isElementNode(node) && isButton(node) ? [node] : [],
  );
}

/** Whether keys can press buttons at all: not in the VS Code extension ("buttons have no hotkeys"), not in a collapsed band. */
const pressableByKey = (context: HotkeyContext): boolean =>
  context.surface !== 'vscode' && !(context.isBand && context.isCollapsed);

/**
 * The button each hotkey names, wherever the keyboard focus is: "when two buttons in one drawing
 * name the same `hotkey`, the later one gets it" (reference.md, "Elements").
 */
function treeHotkeys(tree: unknown): Map<string, ElementNode> {
  const table = new Map<string, ElementNode>();
  for (const button of buttons(tree)) {
    const { hotkey } = button.props;
    if (typeof hotkey === 'string') table.set(hotkey, button);
  }
  return table;
}

/**
 * The hotkeys of the band's rows: "Buttons of two mods' rows of the band on the same key: in the
 * terminal the key presses neither" (Agent Plugins Extended §10.10.5 item 4, §10.44.10); the
 * client does not pick one of the mods. The Desktop app keeps the later row's (judgment: the spec
 * states the rule for the terminal alone).
 */
function rowHotkeys(
  rows: NonNullable<HotkeyContext['rows']>,
  surface: UiSurface,
): Map<string, ElementNode> {
  const table = new Map<string, ElementNode>();
  const owner = new Map<string, string>();
  const shared = new Set<string>();
  for (const row of rows) {
    for (const [hotkey, button] of treeHotkeys(row.tree)) {
      const other = owner.get(hotkey);
      if (other !== undefined && other !== row.plugin) shared.add(hotkey);
      owner.set(hotkey, row.plugin);
      table.set(hotkey, button);
    }
  }
  if (surface === 'terminal') for (const hotkey of shared) table.delete(hotkey);
  return table;
}

function namedHotkeys(tree: unknown, context: HotkeyContext): ReadonlyMap<string, ElementNode> {
  if (!pressableByKey(context)) return new Map();
  return context.rows === undefined ? treeHotkeys(tree) : rowHotkeys(context.rows, context.surface);
}

/**
 * The button each hotkey presses now: a hotkey presses its button only while the pane or band has
 * keyboard focus, and while an `Input` has it every printable key goes to the field (interface.md,
 * "Keyboard focus and hotkeys", "What each key does").
 */
export function hotkeyTable(
  tree: unknown,
  context: HotkeyContext,
): ReadonlyMap<string, ElementNode> {
  if (!context.hasFocus || context.isInputFocused) return new Map();
  return namedHotkeys(tree, context);
}

/**
 * The button a digit typed alone into an empty prompt (and then a pause) presses: "a digit `hotkey`
 * on a button in the band also fires" (reference.md, "Elements"). Only the band has it, and it is
 * one of the two presses that need no keyboard focus (the other is an `action` binding).
 */
export function digitButton(
  tree: unknown,
  context: HotkeyContext,
  typed: string,
  isPromptEmpty: boolean,
): ElementNode | undefined {
  if (!context.isBand || !isPromptEmpty || !/^[0-9]$/.test(typed)) return undefined;
  return namedHotkeys(tree, context).get(typed);
}

/** Whether a keybinding presses a button's `action`: "when that binding is a chord or a modified key". */
const bindingPressesAction = (binding: string): boolean =>
  /\s/.test(binding.trim()) || /(ctrl|alt|meta|shift|cmd)\+/i.test(binding);

/** The buttons the user's `binding` of the keybinding action `action` presses. */
export function actionButtons(
  tree: unknown,
  context: HotkeyContext,
  action: string,
  binding: string,
): readonly ElementNode[] {
  if (!pressableByKey(context) || !bindingPressesAction(binding)) return [];
  return buttons(tree).filter((button) => button.props.action === action);
}
