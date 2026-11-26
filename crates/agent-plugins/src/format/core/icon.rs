//! Port of `src/format/core/icon.ts`, switched.
//!
//! Plugin icon candidates (core §4.3.1): a regular file directly in the root `assets/` directory whose
//! whole name is lowercase, `icon` and one image-format extension the client recognises. PNG is the
//! one format every displaying client supports, so it is always recognised. The kit only finds the
//! candidates: reading, decoding and rendering them, and making SVG inert, are the host's, and an
//! unusable candidate never affects the package (core §4.3.1).

use crate::ports::files::DirectoryEntry;

/// `^icon\.([a-z0-9]+)$`: the extension of a candidate's name.
fn candidate_extension(name: &str) -> Option<&str> {
    let extension = name.strip_prefix("icon.")?;
    (!extension.is_empty()
        && extension
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit()))
    .then_some(extension)
}

/// The candidates among the entries of `assets/`, as root-relative paths: PNG first, then the other
/// recognised formats in the order `formats` gives them (judgment: a client may pick any).
#[must_use]
pub fn icon_candidates(entries: &[DirectoryEntry], formats: &[String]) -> Vec<String> {
    let mut order = vec!["png".to_owned()];
    order.extend(
        formats
            .iter()
            .map(|format| format.to_lowercase())
            .filter(|one| one != "png"),
    );
    let mut found: Vec<(usize, String)> = entries
        .iter()
        .filter_map(|entry| {
            let extension = if entry.is_file {
                candidate_extension(&entry.name)
            } else {
                None
            };
            let rank = extension.and_then(|one| order.iter().position(|known| known == one))?;
            Some((rank, format!("assets/{}", entry.name)))
        })
        .collect();
    found.sort_by_key(|(rank, _)| *rank);
    found.into_iter().map(|(_, path)| path).collect()
}

crate::registry::exports! {
    "format/core/icon" {
        "iconCandidates" => ok |entries: Vec<DirectoryEntry>, formats: Option<Vec<String>>|
            icon_candidates(&entries, &formats.unwrap_or_default());
    }
}
