/**
 * permissions.md ("PowerShell"): "Common aliases are canonicalized before matching ... so
 * `PowerShell(Get-ChildItem *)` matches `gci`, `ls`, and `dir` as well." The docs name those three;
 * the rest are PowerShell's own built-in aliases (`Get-Alias` on Windows PowerShell 5.1 and
 * PowerShell 7), so a deny rule on a cmdlet is not passed by its short name (spec/judgments.md).
 * Keys are lower case because matching ignores case.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/rules/bash/powershell_aliases.rs`,
 * docs/RUST.md).
 */
import { coreModule } from '../../wasm/bridge.ts';

const call = coreModule('rules/bash/powershell-aliases');

export const POWERSHELL_ALIASES: ReadonlyMap<string, string> = call('POWERSHELL_ALIASES');
