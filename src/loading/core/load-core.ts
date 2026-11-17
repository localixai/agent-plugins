import { loadFiles } from '../../format/load-files.ts';
import { coreExtensionDirectories, coreIcons } from './assets.ts';
import { readCoreHooksFile, readCoreMcpFile } from './config.ts';
import { coreLocate } from './locations.ts';
import { coreManifestStep } from './manifest-step.ts';
import type { LoadCorePackageInput, LoadedCorePackage } from './model.ts';
import { readCoreSkills } from './skills.ts';

/**
 * Loads a core package (Agent Plugins 1.1.0, core §4–§10; docs/API.md, "Core packages"): the root
 * `plugin.json` first, which rejects the package when it fails (core §5.1–§5.3); then each portable
 * component type from its fixed location, each failing on its own narrowest boundary (core §4.1,
 * §6.2, §7.1, §7.2.2, §7.3.2, §11.3): an invalid type, server, event member, handler or skill is
 * reported and skipped while the rest loads. Nothing of the Claude Code layout is read: a core
 * package has no other component location (core §6.1). Never throws.
 */
export async function loadCorePackage(input: LoadCorePackageInput): Promise<LoadedCorePackage> {
  const { signal } = input;
  const context = coreLocate(loadFiles(input.files, signal), signal);
  const implemented = input.extensions ?? [];
  const step = await coreManifestStep(context, implemented);
  if (step.manifest === undefined || step.version === undefined) {
    return { package: undefined, issues: step.issues };
  }
  const { manifest, version } = step;
  const [skills, mcp, hooks, icons, extensionDirectories] = await Promise.all([
    readCoreSkills(context, manifest.name),
    readCoreMcpFile(context, version, {
      implemented,
      ...(input.transports && { transports: input.transports }),
    }),
    readCoreHooksFile(context, version, implemented),
    coreIcons(context, input.iconFormats ?? []),
    coreExtensionDirectories(context, implemented),
  ]);
  return {
    package: {
      name: manifest.name,
      version,
      manifest,
      skills: skills.skills,
      mcpServers: mcp.servers,
      hooks: hooks.hooks,
      icons,
      extensionDirectories,
    },
    issues: [...step.issues, ...skills.issues, ...mcp.issues, ...hooks.issues],
  };
}
