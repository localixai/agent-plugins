import type { CorePackage } from '../../loading/core/model.ts';
import { RUN_CORE_HANDLER } from '../adapters/registration.ts';
import { ADAPTER_RUNTIME, RUN_CORE_HANDLER_SOURCE } from '../adapters/sources.ts';
import { conversion, jsonFile, notConverted } from '../notes.ts';
import type { Conversion, ConvertedFile, ConvertOptions } from '../types.ts';
import { CLAUDE_HOOKS, claudeCodeHooks } from './hooks.ts';
import { CLAUDE_MANIFEST, claudeCodeManifest } from './manifest.ts';
import { CLAUDE_MCP, claudeCodeMcp } from './mcp.ts';
import { claudeCodeSkillNotes } from './skills.ts';

/**
 * A core package as a Claude Code plugin (Agent Plugins Extended §18.4.2), pure over the loaded
 * package: the files to write beside it, which make a dual package (§18.4.1 item 4), and a note for
 * everything not carried over (item 1). The extension data the package was loaded with is reported,
 * so a host loads it with every namespace it holds (`convertPluginPackage` does) to see all of it.
 */
export function coreToClaudeCode(source: CorePackage, options: ConvertOptions = {}): Conversion {
  const manifest = claudeCodeManifest(source.manifest);
  const mcp = claudeCodeMcp(source.mcpServers);
  const hooks = claudeCodeHooks(source.hooks, options.hookAdapter ?? true);
  const files: ConvertedFile[] = [jsonFile(CLAUDE_MANIFEST, manifest.value)];
  if (Object.keys(mcp.servers).length > 0)
    files.push(jsonFile(CLAUDE_MCP, { mcpServers: mcp.servers }));
  if (Object.keys(hooks.hooks).length > 0)
    files.push(jsonFile(CLAUDE_HOOKS, { hooks: hooks.hooks }));
  if (hooks.adapted) files.push({ path: RUN_CORE_HANDLER, text: RUN_CORE_HANDLER_SOURCE });
  const directories = source.extensionDirectories.map((directory) =>
    notConverted(
      `${directory}/`,
      undefined,
      'an extension directory has no Claude Code counterpart',
    ),
  );
  return conversion(
    'claude-code',
    files,
    [
      ...manifest.notes,
      ...claudeCodeSkillNotes(source.skills),
      ...mcp.notes,
      ...hooks.notes,
      ...directories,
    ],
    hooks.adapted ? ADAPTER_RUNTIME : undefined,
  );
}
