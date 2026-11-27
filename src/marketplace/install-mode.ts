import type { PluginIssue } from '../format/issues.ts';
import type { MarketplaceSourceKind, PluginSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';

/**
 * How a plugin is installed: loaded where it lies, copied into the cache, or linked from the cache
 * entry (plugins/loading.md, "In-place and copied plugins"; plugins/marketplace-reference.md, "Copy
 * mode and link mode", "Output that fails the install or update"; plugins/host-marketplace.md,
 * "Edit plugins in place on a shared directory"). Pure: the origin and the source in, a decision or
 * the refusal out; the host copies, links or extracts.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_mode.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/install-mode');

export type InstallMode = 'in-place' | 'copy' | 'link-in-cache';

/** Where a plugin came from, as far as how it is installed goes. */
export type InstallOrigin =
  /** `--plugin-dir` with a directory, or a folder of `CLAUDE_CODE_PLUGIN_DIRS`. */
  | { readonly kind: 'session-directory' }
  /** `--plugin-dir` with a `.zip`, or `--plugin-url`. */
  | { readonly kind: 'session-archive' }
  | { readonly kind: 'skills-directory' }
  | {
      readonly kind: 'marketplace';
      /** How the marketplace was added; absent when the plugin has none (a copy of the entry is all there is). */
      readonly marketplace?: MarketplaceSourceKind;
      readonly source: PluginSource;
    };

export interface InstallDecision {
  readonly mode: InstallMode;
  /** Link mode: the cache entry holds links to the printed directory, which must stay where it is. */
  readonly linked: boolean;
  /** The plugin has a folder in the cache (a copy, or the links of link mode); a plugin loaded in place has none. */
  readonly cached: boolean;
  /** "A `--plugin-url` archive or a `--plugin-dir` `.zip` is extracted into a session temp directory first." */
  readonly extractFirst: boolean;
  /** Whether Claude Code installs the plugin's Node.js packages (only into a copied version directory). */
  readonly dependencyInstall: boolean;
  /**
   * Whether an edit to the source reaches a session only through an update: a copy is a snapshot,
   * a plugin loaded in place "take[s] effect at the next session start or `/reload-plugins`", and a
   * link-mode plugin is loaded in place (judgment: the docs say "loads the files in place").
   */
  readonly editsNeedUpdate: boolean;
}

/** What the command printed and where the session started, for the checks that need them. */
export interface PrintedDirectory {
  readonly directory: string;
  readonly sessionCwd?: string;
}

export interface ModeInput {
  readonly origin: InstallOrigin;
  readonly platform: 'posix' | 'windows';
  /** For a `command` source: its printed directory. */
  readonly printed?: PrintedDirectory;
}

/**
 * How the plugin is installed, or why it may not be: "Claude Code refuses to install a link-mode
 * plugin on Windows", and a printed directory that is a UNC path on Windows or "the one Claude Code
 * was started in, or one of its parents" fails the install.
 */
export const decideInstallMode = (input: ModeInput): InstallDecision | PluginIssue =>
  call('decideInstallMode', input);

/**
 * "Sessions started inside the directory": "a session started in the printed directory or anywhere
 * below it doesn't load" a link-mode plugin. Only link mode loads from the printed directory.
 */
export const linkedPluginLoads = (
  decided: InstallDecision,
  printed: PrintedDirectory,
  platform: ModeInput['platform'],
): boolean => call('linkedPluginLoads', decided, printed, platform);
