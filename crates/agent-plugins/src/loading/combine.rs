//! The core's counterpart of `src/loading/combine.ts` at sha256:c2f1619b4b4f, which stays TypeScript: its
//! table answers with the very values it was given (`add` keeps them in a `Set`, by identity), and
//! its callers find them again by identity, which no copy across the boundary can be (docs/RUST.md,
//! "What stays TypeScript").
//!
//! How a manifest key combines with its default location (plugins/manifest-reference.md, "How each
//! key combines with its default location"). `declared` is `None` when the manifest does not set
//! the key; the result is the entries to load, in loading order.
//!
//! - `replace`: a key that is set replaces the default, so the default is not scanned (an empty list
//!   included: the key is still set).
//! - `add`: the default and the listed entries load; one named twice loads once.
//! - `merge`: the default loads first, what the manifest declares after it, in the order declared;
//!   merging the content (a later server name replaces an earlier one) is the reader's.

use crate::format::layout::CombineMode;

/// `COMBINE[mode](defaults, declared)`.
#[must_use]
pub fn combine<T: Clone + PartialEq>(
    mode: CombineMode,
    defaults: &[T],
    declared: Option<&[T]>,
) -> Vec<T> {
    match mode {
        CombineMode::Replace => declared.unwrap_or(defaults).to_vec(),
        CombineMode::Add => {
            let mut out: Vec<T> = Vec::new();
            for item in defaults.iter().chain(declared.unwrap_or_default()) {
                if !out.contains(item) {
                    out.push(item.clone());
                }
            }
            out
        }
        CombineMode::Merge => defaults
            .iter()
            .chain(declared.unwrap_or_default())
            .cloned()
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::combine;
    use crate::format::layout::CombineMode;

    #[test]
    fn combines_as_each_mode_says() {
        let defaults = ["a", "b"];
        let declared = ["b", "c"];
        assert_eq!(
            combine(CombineMode::Replace, &defaults, Some(&declared)),
            ["b", "c"]
        );
        assert_eq!(combine(CombineMode::Replace, &defaults, None), ["a", "b"]);
        assert_eq!(
            combine(CombineMode::Add, &defaults, Some(&declared)),
            ["a", "b", "c"]
        );
        assert_eq!(
            combine(CombineMode::Merge, &defaults, Some(&declared)),
            ["a", "b", "b", "c"]
        );
    }
}
