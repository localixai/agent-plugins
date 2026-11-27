import { coreModule } from '../../wasm/bridge.ts';

/**
 * permissions.md ("Exec wrappers and `find` actions"): "A prefix rule like `Bash(watch *)` or
 * `Bash(find *)` can't auto-approve" the exec wrappers `watch`, `setsid`, `ionice` and `flock`,
 * nor `find` "with an action that runs commands, deletes files, or writes files, such as `-exec`,
 * `-delete`, `-rm`, or `-fprint`, or with `-files0-from`".
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/exec_wrappers.rs`,
 * docs/RUST.md).
 */
const call = coreModule('rules/bash/exec-wrappers');

/** `promptsDespitePrefixRule` for a subcommand's text. */
export const requiresPromptDespitePrefixRule = (subcommand: string): boolean =>
  call('requiresPromptDespitePrefixRule', subcommand);
