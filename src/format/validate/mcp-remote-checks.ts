import { coreModule } from '../../wasm/bridge.ts';
import type { Finding } from './finding.ts';

/**
 * The checks of `claude plugin validate` on a remote server's `url` and `headers` (plugins/
 * manifest-reference.md, "Validate the manifest"; spec §8 item 3): an error for a `url` that is not
 * a valid absolute URL once its references are set aside, warnings for an `http://` or `ws://` URL
 * to a host that is not loopback and for a header value that looks like a literal credential. The
 * wording and the credential shapes are oracle 2.1.325's (spec/judgments.md).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/validate/mcp_remote_checks.rs`,
 * docs/RUST.md).
 */
const call = coreModule('format/validate/mcp-remote-checks');

/** The url and header findings of one server at `path`. */
export const remoteFindings = (
  path: string,
  server: Readonly<Record<string, unknown>>,
): Finding[] => call('remoteFindings', path, server);
