/** Implemented by the Rust core (`crates/agent-plugins/src/settings/managed/wsl_chain.rs`, docs/RUST.md). */
import { coreModule } from '../../wasm/bridge.ts';
import type { PluginIssue } from '../../format/issues.ts';
import type { AdminDocument } from './admin-source.ts';

const call = coreModule('settings/managed/wsl-chain');

/**
 * `wslInheritsWindowsSettings` (settings-reference.md, `wslInheritsWindowsSettings`; managed-settings.md,
 * "How Claude Code combines managed sources", "When an admin document counts as present"): which
 * documents a WSL session composes. The host reads the Windows `HKLM` value and the Windows
 * managed settings file (through `C:\Program Files\ClaudeCode`), `/etc/claude-code` and `HKCU`;
 * this decides which of them count.
 */
export interface WslDocuments {
  /** The `HKLM` registry value. */
  readonly hklm?: AdminDocument;
  /** The managed settings file and drop-ins under `C:\Program Files\ClaudeCode`. */
  readonly windowsFile?: AdminDocument;
  /** The managed settings file and drop-ins under `/etc/claude-code`. */
  readonly etcFile?: AdminDocument;
  /** The `HKCU` registry value. */
  readonly hkcu?: AdminDocument;
}

export interface WslChain {
  /** The documents that compose, with the sources `composeManaged` ranks them as. */
  readonly documents: readonly AdminDocument[];
  /** The Windows policy chain is on. */
  readonly chainOn: boolean;
  readonly issues: readonly PluginIssue[];
}

/**
 * The documents a WSL session composes. "Claude Code honors it only when set in the HKLM registry
 * key or in a managed settings file or drop-in under `C:\Program Files\ClaudeCode\`". With the
 * chain on, the Windows documents stand above `/etc/claude-code`, which is read "only when no
 * Windows admin document is present", and `HKCU` joins "only when HKCU also sets the key to `true`".
 * With it off, WSL reads `/etc/claude-code` alone; an `HKLM` value or Windows-folder file that
 * exists but cannot be read keeps `/etc/claude-code` from applying either way.
 */
export const wslChain = (documents: WslDocuments): WslChain => call('wslChain', documents);
