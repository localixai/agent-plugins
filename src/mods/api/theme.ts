import { z } from 'zod';
import type { ModsNamespaceApi } from '../types.ts';
import { call, parse, type Deps } from './call.ts';
import { nameProblem } from './names.ts';
import type {
  BackdropDrawn,
  BackdropGlass,
  SessionBackdrop,
  SessionTheme,
  ThemeBackdrop,
} from './ports.ts';

/**
 * `$.theme` (Agent Plugins Extended §10.25.6 and §10.25.7; `ThemeSetArgs`,
 * claude-code/index.d.ts:17320): `set` gives the session its one theme, a backdrop behind the
 * conversation for the Desktop or mobile app, the Desktop app's colours, sound and style, and
 * `overrideColorScheme`; `clear` takes it off, whichever mod set it.
 */

/** What `$.theme.clear` resolves with: the reason the docs quote, `the theme is cleared`. */
export const THEME_CLEARED = 'the theme is cleared';

/** The request id of a backdrop at `ui.render`. */
const BACKDROP_REQUEST = 'backdrop';

/** A raw color: `#hex`, or `rgb()`, `rgba()`, `hsl()`, `hsla()` of numbers; a theme key or a name is refused. */
const RAW_COLOR =
  /^(?:#(?:[0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})|(?:rgba?|hsla?)\(\s*-?[\d.]+%?(?:\s*[,/ ]\s*-?[\d.]+%?){2,3}\s*\))$/i;

/**
 * `glass` (`ThemeBackdropGlass`): `blur` 0 to 64 (absent, 16), `tint` a raw color (absent, the
 * surface's panel color), `opacity` 0 to 1 (absent, 0.6); or `false`.
 */
const glassFields = z.union([
  z.literal(false),
  z.object({
    blur: z.number().min(0).max(64).default(16),
    tint: z.string().regex(RAW_COLOR).optional(),
    opacity: z.number().min(0).max(1).default(0.6),
  }),
]);

/** The glass of a scene that names none: the default glass, `{}` with its defaults (Agent Plugins Extended §10.26). */
export const DEFAULT_GLASS = { blur: 16, opacity: 0.6 } as const;

const list = z.array(z.unknown());
const flag = z.boolean().optional();

/** The four kinds of backdrop (`ThemeBackdrop`, :16859; `ThemeGround`, :17128), one key each. */
const BACKDROP_KINDS = ['scene', 'color', 'gradient', 'vector'] as const;
const backdropFields = z
  .looseObject({
    scene: z.string().optional(),
    glass: glassFields.optional(),
    color: z.string().optional(),
    gradient: z
      .looseObject({
        stops: list,
        angle: z.number().optional(),
        repeat: flag,
        shape: z.enum(['linear', 'radial']).optional(),
        size: list.optional(),
      })
      .optional(),
    vector: z
      .looseObject({
        paths: list,
        viewBox: z.union([z.number(), list]),
        color: z.string().optional(),
        repeat: flag,
        size: list.optional(),
      })
      .optional(),
  })
  .refine((b) => BACKDROP_KINDS.filter((kind) => b[kind] !== undefined).length === 1, {
    message: 'a backdrop is one of { scene, glass? }, { color }, { gradient } or { vector }',
  })
  .refine((b) => b.glass === undefined || b.scene !== undefined, {
    message: 'glass goes with a scene',
  });

/** `colors` (`ThemeColors`, :16980): checked against the declarations, never read (J10.113). */
const colorsFields = z.looseObject({
  v: z.literal(1),
  label: z.string(),
  inputs: z.looseObject({
    light: z.record(z.string(), z.unknown()).optional(),
    dark: z.record(z.string(), z.unknown()).optional(),
  }),
  id: z.string().optional(),
  contrast: z.literal('more').optional(),
  codeTheme: z
    .looseObject({ light: z.string().optional(), dark: z.string().optional() })
    .optional(),
});
const passed = z.record(z.string(), z.unknown());

/**
 * The call's attributes: an attribute the build does not know is refused by name, and the call
 * names at least one (§10.25.6 item 3).
 */
const setArgs = z
  .strictObject({
    backdrop: backdropFields.optional(),
    overrideColorScheme: z.enum(['light', 'dark']).optional(),
    colors: colorsFields.optional(),
    sound: passed.optional(),
    style: passed.optional(),
  })
  .refine((args) => Object.values(args).some((value) => value !== undefined), {
    message: 'names no attribute: backdrop, colors, overrideColorScheme, sound or style',
  });

type Backdrop = z.infer<typeof backdropFields>;

/** The glass as the site's props carry it: the defaults filled in, a `tint` only when one was given. */
function glassOf(glass: Backdrop['glass']): BackdropGlass {
  if (glass === undefined) return DEFAULT_GLASS;
  if (glass === false) return false;
  const { blur, tint, opacity } = glass;
  return { blur, ...(tint === undefined ? {} : { tint }), opacity };
}

/** The backdrop as the session keeps it: a scene with its glass, or the one ground given. */
function backdropOf(backdrop: Backdrop): ThemeBackdrop {
  const { scene, color, gradient, vector } = backdrop;
  if (scene !== undefined) {
    const problem = nameProblem('scene', scene);
    if (problem !== undefined) throw new Error(`$.theme.set: ${problem}`);
    return { scene, glass: glassOf(backdrop.glass) };
  }
  if (color !== undefined) return { color };
  return gradient === undefined ? { vector: vector ?? {} } : { gradient };
}

/** The theme `set` was given, with the mod that set it, or the reason the call rejects. */
function themeOf(deps: Deps, event: unknown): SessionTheme {
  const { backdrop, overrideColorScheme, colors, sound, style } = parse(
    '$.theme.set',
    setArgs,
    event,
  );
  return {
    plugin: deps.plugin,
    ...(backdrop === undefined ? {} : { backdrop: backdropOf(backdrop) }),
    ...(overrideColorScheme === undefined ? {} : { overrideColorScheme }),
    ...(colors === undefined ? {} : { colors }),
    ...(sound === undefined ? {} : { sound }),
    ...(style === undefined ? {} : { style }),
  };
}

/**
 * The `Backdrop` site's props for `theme` (reference.md, "Render sites"): `scene`, `plugin`, `glass`
 * and `overrideColorScheme` when given; `undefined` when the theme has no scene, which no hook draws.
 */
export function sceneBackdrop(theme: SessionTheme | undefined): SessionBackdrop | undefined {
  const backdrop = theme?.backdrop;
  if (theme === undefined || backdrop === undefined || !('scene' in backdrop)) return undefined;
  return {
    scene: backdrop.scene,
    plugin: theme.plugin,
    glass: backdrop.glass,
    ...(theme.overrideColorScheme === undefined
      ? {}
      : { overrideColorScheme: theme.overrideColorScheme }),
  };
}

/**
 * Drawn, the answer carries `requestId` `backdrop`, "the `e.requestId` of every backdrop at
 * `ui.render`".
 */
async function set(deps: Deps, event: unknown): Promise<BackdropDrawn & { requestId?: string }> {
  const theme = themeOf(deps, event);
  deps.registries.theme.set(theme);
  deps.ports.theme.changed(theme);
  const drawn = await deps.ports.theme.drawn();
  return drawn.isDrawn ? { ...drawn, requestId: BACKDROP_REQUEST } : drawn;
}

/** Judgment: `clear` resolves `{ isDrawn: false, reason }`, the shape of `set`'s result, even with no theme set. */
function clear(deps: Deps): Promise<BackdropDrawn> {
  if (deps.registries.theme.clear()) deps.ports.theme.changed(undefined);
  return Promise.resolve({ isDrawn: false, reason: THEME_CLEARED });
}

export const themeApi = (deps: Deps): ModsNamespaceApi<'theme'> => ({
  set: async (...args) => {
    themeOf(deps, args[0]);
    return call(deps, 'theme.set', parse('$.theme.set', setArgs, args[0]), (e) => set(deps, e));
  },
  clear: async () => call(deps, 'theme.clear', {}, () => clear(deps)),
});
