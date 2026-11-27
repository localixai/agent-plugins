//! Layer 2: the marketplace catalog's readers and the small owners they share
//! (`src/format/marketplace/` in the npm package).

pub mod archive_url;
pub mod blocked_hosts;
pub mod display_fields;
pub mod entries;
pub mod entry_append;
pub mod entry_checks;
pub mod entry_merge;
pub mod entry_schema;
pub mod header_names;
pub mod ids;
#[expect(
    clippy::module_inception,
    reason = "the package's file `src/format/marketplace/marketplace.ts`, file for file (RS1)"
)]
pub mod marketplace;
pub mod marketplace_name_lists;
pub mod marketplace_names;
pub mod marketplace_schema;
pub mod marketplace_source;
pub mod plugin_source;
pub mod plugin_source_fields;
pub mod plugin_source_kinds;
pub mod relevance;
pub mod renames;
pub mod schema_issues;
pub mod settings_source_plugins;
pub mod source_failures;
pub mod types;
