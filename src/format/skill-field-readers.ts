import { coreModule } from '../wasm/bridge.ts';

/**
 * Readers of the skill frontmatter fields (skills.md, "Frontmatter reference"); `FIELD_READERS` in
 * `skill-fields.ts` holds them, and only the limit they report against is read here.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/skill_field_readers.rs`,
 * docs/RUST.md).
 */

const call = coreModule('format/skill-field-readers');

/** The Agent Skills spec's limit, quoted by skills.md: "a string of up to 500 characters". */
export const COMPATIBILITY_LIMIT: number = call('COMPATIBILITY_LIMIT');
