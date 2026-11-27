//! Layer 4: the marketplace policy: command sources, repository identity, refusals and the kinds
//! of source (`src/settings/marketplaces/` in the npm package).

pub mod command_sources;
pub mod facts;
pub mod gate;
pub mod git_url;
pub mod headers_helper;
pub mod host_of;
pub mod match_source;
pub mod official_registration;
pub mod refusal_messages;
pub mod registry;
pub mod source_kinds;
pub mod source_validate;
