/** Conversion between the two plugin forms (Agent Plugins Extended §18.4; docs/API.md, "Converting between the forms"). */
export { convertPluginPackage, type ConvertPackageInput } from './convert-package.ts';
export { coreToClaudeCode } from './to-claude-code/index.ts';
export { claudeCodeToCore, type ClaudeCodeToCoreOptions } from './to-core/index.ts';
export {
  CONVERSION_OUTCOMES,
  type Conversion,
  type ConversionNote,
  type ConversionOutcome,
  type ConvertedFile,
  type ConvertOptions,
} from './types.ts';
