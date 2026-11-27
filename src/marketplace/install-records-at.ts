import { coreModule } from '../wasm/bridge.ts';
import type { InstalledRecords } from './installed-plugins.ts';

/**
 * Which install records apply in one project folder (plugins/install.md, "Choose an install
 * scope"; plugins/overview.md, "Understand install scopes"): a user-scope install is "enabled for
 * you in every project on this machine", a project- or local-scope one in "this repository" only,
 * the folder its `projectPath` names ("Project the install belongs to", plugins/cli-reference.md).
 * Pure over the records `parseInstalledPlugins` read. Judgments (spec/judgments.md): a managed
 * record applies everywhere, like a user one; a project or local record without `projectPath`
 * belongs to no folder; the folders compare as `selectUninstall` compares them (`samePath`).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/marketplace/install_records_at.rs`,
 * docs/RUST.md).
 */
const call = coreModule('marketplace/install-records-at');

/** Where the session runs: the project folder whose `.claude/` settings it reads, if any. */
export interface InstallsAt {
  readonly projectPath?: string;
  readonly platform: 'posix' | 'windows';
}

/** The records that apply at `at`, by plugin id in file order; a plugin none of whose records apply is left out. */
export const installsAt = (records: InstalledRecords, at: InstallsAt): InstalledRecords =>
  call('installsAt', records, at);
