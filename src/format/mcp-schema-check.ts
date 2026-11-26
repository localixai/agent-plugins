/**
 * The check mcp.md ("Tools with invalid input schemas") names: "The schema must be valid against the
 * JSON Schema draft 2020-12 meta-schema", and "Top-level property names must be 1 to 64 characters
 * long and use only ASCII letters and digits, `_`, `.`, and `-`". The kit has no JSON Schema
 * validator, so the meta-schema is carried as a table: each keyword of its vocabularies (core,
 * applicator, unevaluated, validation, format, content, meta-data, and the deprecated `definitions` and
 * `dependencies`) with the shape its value must have. A keyword the vocabularies do not define is
 * allowed, as in the meta-schema.
 *
 * Implemented by the Rust core (`crates/agent-plugins/src/format/mcp_schema_check.rs`, docs/RUST.md).
 */
import { coreModule } from '../wasm/bridge.ts';

const call = coreModule('format/mcp-schema-check');

/** The places in `value` that break the draft 2020-12 meta-schema, as `path: message`. */
export const metaSchemaProblems = (value: unknown): string[] => call('metaSchemaProblems', value);

/** The top-level properties whose names the API would refuse. */
export const propertyNameProblems = (value: unknown): string[] =>
  call('propertyNameProblems', value);

export const checksMetaSchema = (value: unknown): boolean => call('checksMetaSchema', value);
