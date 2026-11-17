/** The pure readers of the core Agent Plugins form (Agent Plugins 1.1.0, core §4–§10; docs/API.md, "Core packages"). */
export { commandProblem } from './command.ts';
export { checkDualManifests } from './dual.ts';
export { extensionsProblem, type CoreExtensions } from './extensions.ts';
export {
  CORE_HOOK_EVENTS,
  readCoreHooks,
  type CoreHookEvent,
  type CoreHookHandler,
  type CoreHooks,
  type CoreHookPositions,
  type CoreHooksResult,
} from './hooks.ts';
export { iconCandidates } from './icon.ts';
export {
  readCoreManifest,
  type CoreAuthor,
  type CoreManifest,
  type CoreManifestResult,
} from './manifest.ts';
export { readCoreMcp, type CoreMcpOptions, type CoreMcpResult } from './mcp.ts';
export {
  CORE_TRANSPORTS,
  readCoreServer,
  type CoreMcpServer,
  type CoreRemoteServer,
  type CoreStdioServer,
  type CoreTransport,
} from './mcp-server.ts';
export { coreNameProblems } from './name.ts';
export {
  CORE_VARIABLES,
  expandPlaceholders,
  resolveCwd,
  type CorePaths,
  type CoreVariable,
  type ResolvedCwd,
} from './placeholders.ts';
export {
  coreEnvironment,
  coreHandlerProcess,
  coreServerProcess,
  type CoreProcess,
  type CoreProcessInput,
} from './subprocess.ts';
export { headersProblem, urlProblem } from './remote.ts';
export { checkCoreSkill, type CoreSkillCheck } from './skill.ts';
export {
  CORE_DOCUMENTS,
  CORE_VERSIONS,
  coreSchemaId,
  coreSchemaVersion,
  type CoreDocument,
  type CoreVersion,
} from './versions.ts';
