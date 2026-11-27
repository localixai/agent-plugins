//! Layer 4: the marketplace policy: command sources, repository identity, refusals and the kinds
//! of source (`src/settings/marketplaces/` in the npm package).

pub mod command_sources;
pub mod facts;
pub mod git_url;
pub mod host_of;
pub mod match_source;
pub mod refusal_messages;
pub mod source_kinds;
pub mod source_validate;
