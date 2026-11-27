/**
 * What a manifest path of each kind must be (plugins/manifest-reference.md, "Path-only fields":
 * "`agents` entries must be `.md` files, and `skills` entries must be directories. The other three
 * accept a directory or a file"; `commands` takes a flat file or a directory). The message of the
 * mismatch, or `undefined` when the path is the right shape. The wording is the kit's (judgment).
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/loading/declared_shape.rs`, docs/RUST.md).
 */
import type { FileStat } from '../ports/index.ts';
import { coreModule } from '../wasm/bridge.ts';
import type { ListedKind } from './walk.ts';

const call = coreModule('loading/declared-shape');

type Shape = (stat: FileStat, written: string) => string | undefined;

export const DECLARED_SHAPE: { readonly [K in ListedKind]: Shape } = call('DECLARED_SHAPE');
