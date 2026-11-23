import { ENGINE_ORIGIN, type ModTier } from '../../protocol/mods-events.ts';
import { GUARD_ID } from '../../protocol/mods-settings.ts';
import type { HookAccess } from '../types.ts';
import { once } from './guard.ts';
import type { LoadedMod } from './loader.ts';
import { protectedHookNotes } from './guard-notes.ts';
import { GUARDED_DOORS, GUARDED_EVENTS } from './guard-surfaces.ts';
import { isRecord } from '../../protocol/is-record.ts';
import { offersManagedTool, type ManagedToolInfo } from './guard-tools.ts';

/** Identity facts the embedding host knows before it emits an event. */
export interface ManagedModData {
  /** True for a descriptor of a managed MCP tool. No private descriptor fields are assumed. */
  readonly isManagedTool?: (input: unknown) => boolean;
  /** Additional protected conversation rows, such as the stored text of a skill. */
  readonly isProtectedRow?: (input: unknown) => boolean;
  /**
   * The tools of the managed MCP servers connected now, as the host lists them in the client's tool
   * pool (`mcp__<server>__<tool>`): the guard keeps them in the pool the client's own `tool.list`
   * resolves, and a user's `tool.offer` hook does not run for one (§14.9.2 item 3).
   */
  readonly managedTools?: () => readonly ManagedToolInfo[];
}

const USER_TIER: Record<ModTier, boolean> = {
  core: false,
  builtin: false,
  prepend: false,
  user: true,
  append: false,
};

function protectedRow(input: unknown, given: ManagedModData): boolean {
  const door = isRecord(input) && typeof input.door === 'string' ? input.door : '';
  return GUARDED_DOORS[door] === true || (given.isProtectedRow?.(input) ?? false);
}

export interface GuardProtection {
  readonly admit: (access: HookAccess) => boolean;
  readonly bind: (mod: LoadedMod) => LoadedMod;
  readonly notes: (mod: LoadedMod) => readonly string[];
}

/** admin.md § Know which hooks of a user's mod don't run; Enforce on the original event. */
export function createGuardProtection(given: ManagedModData): GuardProtection {
  let current: object | undefined;
  const conditional: Readonly<Record<string, ((access: HookAccess) => boolean) | undefined>> = {
    'session.append': ({ input }) => protectedRow(input, given),
    'tool.describe': ({ input }) => given.isManagedTool?.(input) ?? false,
    'tool.offer': ({ input }) => offersManagedTool(input, given.managedTools?.() ?? []),
    // The client's own requests reach only the organization's mods (§10.19.7, 2.1.321 on).
    'http.fetch': ({ origin }) => origin.plugin === ENGINE_ORIGIN.plugin,
  };
  return {
    admit: (access) => {
      if (!current || access.isBuiltin || !USER_TIER[access.tier]) return true;
      if (GUARDED_EVENTS[access.event] === true) return false;
      try {
        const check = Object.hasOwn(conditional, access.event)
          ? conditional[access.event]
          : undefined;
        return !(check?.(access) ?? false);
      } catch {
        // Judgment: an unavailable identity fact cannot establish that an untrusted hook is safe.
        return false;
      }
    },
    bind: (mod) => {
      const token = {};
      current = token;
      return {
        ...mod,
        id: GUARD_ID,
        unload: once(async () => {
          if (current === token) current = undefined;
          await mod.unload();
        }),
      };
    },
    notes: (mod) =>
      current !== undefined && USER_TIER[mod.tier] && !mod.isBuiltin
        ? protectedHookNotes(mod.hooks)
        : [],
  };
}
