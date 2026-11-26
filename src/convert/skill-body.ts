import { coreModule } from '../wasm/bridge.ts';

/**
 * What Claude Code expands in a skill's body and a core client shows as written (Agent Plugins
 * Extended §18.3.2 item 3, §18.4.2 item 2, §18.4.3 item 2): argument placeholders (`$ARGUMENTS`,
 * `$ARGUMENTS[N]`, `$N`, skills.md "Available string substitutions"), the `${CLAUDE_*}` and
 * `${user_config.*}` variables, and injected commands (`` !`cmd` `` and ```` ```! ````, "Inject
 * dynamic context"). A placeholder after a single backslash is escaped and stays text, as
 * `skills/substitute.ts` reads it; `$name` expands only for names a Claude Code `arguments` field
 * declares, which no core skill has.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/convert/skill_body.rs`, docs/RUST.md).
 */
const call = coreModule('convert/skill-body');

/** Each kind of expanded text the body holds, as a phrase that names it, in a fixed order. */
export const expandedFeatures = (text: string): readonly string[] => call('expandedFeatures', text);
