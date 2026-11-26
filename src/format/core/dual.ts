import { coreModule } from '../../wasm/bridge.ts';
import type { ValidationIssue } from '../validate/finding.ts';

/**
 * The manifest half of a dual package (Agent Plugins Extended §18.3.1): `plugin.json` (core) and
 * `.claude-plugin/plugin.json` (Claude Code) with the same values in the fields both define, chosen
 * so that they load in both. The check reports, per file, what loads in one form only: a core
 * manifest that fails core §5; a field whose two values differ; a name that is not kebab-case (a `.`
 * draws Claude Code's warning, J18.19); an `author` without `name` and a `homepage` that is no
 * absolute URL, which Claude Code refuses; a missing `description`, which its validator warns
 * about; a `version` that is not SemVer; `$schema` or `extensions` in the Claude Code manifest.
 * Claude Code's own rules on its manifest stay `validatePlugin`'s.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/core/dual.rs`, docs/RUST.md).
 */

const call = coreModule('format/core/dual');

/** Checks the two parsed manifests of a dual package (§18.3.1). */
export const checkDualManifests = (core: unknown, claude: unknown): readonly ValidationIssue[] =>
  call('checkDualManifests', core, claude);
