import { coreSchemaId } from '../../format/core/versions.ts';
import type { Plugin } from '../../loading/plugin-model.ts';
import { RUN_CLAUDE_CODE_HANDLER } from '../adapters/registration.ts';
import { ADAPTER_RUNTIME, RUN_CLAUDE_CODE_HANDLER_SOURCE } from '../adapters/sources.ts';
import { conversion, jsonFile } from '../notes.ts';
import type { Conversion, ConvertedFile, ConvertOptions } from '../types.ts';
import { componentNotes } from './components.ts';
import { CORE_HOOKS, coreHooksOf } from './hooks.ts';
import { CORE_MANIFEST, coreManifestOf } from './manifest.ts';
import { CORE_MCP, coreMcpOf } from './mcp.ts';
import { coreSkillNotes } from './skills.ts';

export interface ClaudeCodeToCoreOptions extends ConvertOptions {
  /**
   * The plugin's `hooks/hooks.json` declares `modules` (a mod), which `Plugin` does not carry; it is
   * reported as not converted. `convertPluginPackage` reads it.
   */
  readonly mods?: boolean;
}

/**
 * A Claude Code plugin as a core package (Agent Plugins Extended §18.4.3), pure over the loaded
 * plugin: `plugin.json`, `mcp.json` and `hooks.json` to write beside it, making a dual package
 * (§18.4.1 item 4), and a note for everything §18.4.5 lists as having no core counterpart. A name
 * core §5.5 refuses stops the conversion.
 */
export function claudeCodeToCore(
  source: Plugin,
  options: ClaudeCodeToCoreOptions = {},
): Conversion {
  const manifest = coreManifestOf(source);
  const mcp = coreMcpOf(source.mcpServers, new Set(Object.keys(source.mcpBundles)));
  const hooks = coreHooksOf(source.hooks, options.hookAdapter ?? true);
  const files: ConvertedFile[] = [jsonFile(CORE_MANIFEST, manifest.value)];
  if (Object.keys(mcp.servers).length > 0) {
    files.push(
      jsonFile(CORE_MCP, { $schema: coreSchemaId('1.1.0', 'mcp'), mcpServers: mcp.servers }),
    );
  }
  if (Object.keys(hooks.hooks).length > 0) {
    files.push(
      jsonFile(CORE_HOOKS, { $schema: coreSchemaId('1.1.0', 'hooks'), hooks: hooks.hooks }),
    );
  }
  if (hooks.adapted)
    files.push({ path: RUN_CLAUDE_CODE_HANDLER, text: RUN_CLAUDE_CODE_HANDLER_SOURCE });
  return conversion(
    'core',
    files,
    [
      ...manifest.notes,
      ...coreSkillNotes(source.skills),
      ...mcp.notes,
      ...hooks.notes,
      ...componentNotes(source, options.mods === true),
    ],
    hooks.adapted ? ADAPTER_RUNTIME : undefined,
  );
}
