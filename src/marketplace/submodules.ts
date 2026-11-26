import type { MarketplaceSource } from '../format/marketplace/types.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { SubmoduleRule } from './types.ts';

/**
 * Which git submodules of the marketplace repository a relative-path plugin needs (plugins/
 * marketplace-reference.md, "Git submodules in the marketplace repository", Claude Code v2.1.295
 * and later): pure, the paths in `.gitmodules` and what the plugin's files say in, the paths to
 * fetch out. The host reads the files and runs the fetch; the kit only decides.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/submodules.rs`, docs/RUST.md).
 */
const call = coreModule('marketplace/submodules');

/**
 * The rule for a plugin at `path` ("." at the root) of a marketplace added as `origin`: a sparse
 * checkout "fetches no submodule", a marketplace that was not cloned has none to fetch, a plugin
 * at the root gets every submodule or none, any other plugin only the ones it touches.
 */
export const submoduleRuleFor = (
  path: string,
  origin: MarketplaceSource | undefined,
): SubmoduleRule => call('submoduleRuleFor', path, origin);

/** What the plugin's repository and files say about its submodules; every path is relative to the repository root. */
export interface SubmoduleEvidence {
  /** The `path` of every submodule in `.gitmodules` (`submodulePaths` reads them). */
  readonly submodules: readonly string[];
  /** The plugin's directory: `.` at the root. */
  readonly plugin: string;
  /** Where each symbolic link committed to the repository points. */
  readonly linkTargets: readonly string[];
  /** Root plugin: the directories at the root that hold plugin components (`skills/`, `hooks/`, `bin/`). */
  readonly componentDirectories: readonly string[];
  /** Root plugin: the paths written in the marketplace entry, `plugin.json` and config files (`.mcp.json`, `hooks/hooks.json`). */
  readonly configPaths: readonly string[];
  /** Root plugin: the text of the config files and of the files under the component directories. */
  readonly texts: readonly string[];
  /** The kit "can't tell which ones hold plugin files": a `SKILL.md` at the root, or an unreadable marketplace file, config file or `.gitmodules`. */
  readonly undecidable: boolean;
}

/**
 * The submodules to fetch, in `.gitmodules` order. A root plugin gets all of them or none; when the
 * kit cannot tell, every one (unless the rule is `none`). A path counts when it is the submodule or
 * inside it, and also when it holds the submodule (spec/judgments.md): the rule errs toward fetching.
 */
export const selectSubmodules = (
  rule: SubmoduleRule,
  evidence: SubmoduleEvidence,
): readonly string[] => call('selectSubmodules', rule, evidence);

/** The `path` of each submodule in the text of a `.gitmodules` file, in file order. */
export const submodulePaths = (gitmodules: string): readonly string[] =>
  call('submodulePaths', gitmodules);
