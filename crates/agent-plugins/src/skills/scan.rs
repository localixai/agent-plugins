//! How the skills of a folder are found (`src/skills/scan/` in the npm package).

pub mod container;
pub mod file_location;
pub mod head;
pub mod locations;
pub mod names;
pub mod nested;
pub mod read;
pub mod resolve;
#[expect(
    clippy::module_inception,
    reason = "the port of `src/skills/scan/scan.ts` sits at its source's path"
)]
pub mod scan;
pub mod scan_types;
