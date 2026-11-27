import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * The frontmatter checks of `claude plugin validate` on a skill, agent or command file (oracle
 * 2.1.319, 2.1.321 corpus and 2.1.325; spec §4.10, §5.4.4): no block, a block that does not parse, a block that is not a mapping, a missing
 * description and the few fields whose type it reads. Anything else in the block is left alone.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/component_frontmatter.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/validate/component-frontmatter');

export const COMPONENT_KINDS: readonly ['skill', 'agent', 'command'] = call('COMPONENT_KINDS');
export type ComponentKind = (typeof COMPONENT_KINDS)[number];

/** Where the component file sits: a plugin's folder, or a project's `.claude/` folder. */
export type ComponentPlace = 'plugin' | 'project';

/** Findings of one component file's text, with the paths the oracle gives them. */
export const frontmatterFindings = (
  kind: ComponentKind,
  text: string,
  place: ComponentPlace = 'plugin',
): Finding[] => call('frontmatterFindings', kind, text, place);
