/**
 * The arguments a user typed after a skill's name (skills.md, "Available string substitutions":
 * "Indexed arguments use shell-style quoting, so wrap multi-word values in quotes").
 *
 * The docs give one example, `"hello world" second`. The rest is the kit's judgment
 * (spec/judgments.md): single quotes are literal; double quotes take a backslash before `"` and
 * `\`; outside quotes a backslash escapes whitespace, a quote or a backslash and stays as written
 * before anything else (so a Windows path `C:\Users\me` survives); quoted and bare parts of one
 * word join; `""` is an empty argument; a quote never closed takes the rest of the text.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/skills/arguments.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('skills/arguments');

/** `raw` split into arguments the way a shell would: `"hello world" second` is two. */
export const parseArguments = (raw: string): readonly string[] => call('parseArguments', raw);
