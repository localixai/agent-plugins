//! The core's counterpart of `src/rules/memo.ts` at sha256:35757834d654, which stays TypeScript: its
//! answer is a function that keeps the host's own values (a compiled `RegExp`, a glob set) by key
//! and hands back the very object it made, which no value crossing the boundary can be
//! (docs/RUST.md, "What stays TypeScript"). The core keeps this copy for its own readings
//! (`bash/split.rs`).
//!
//! A function of a text key that keeps its last `limit` answers, so a pattern, a glob set or a
//! command line read once is not read again for the next rule or the next gate of the same call.
//! The answers must be pure functions of the key.
//!
//! Decision: the cache is behind a `Mutex`, as the crate's values are `Send + Sync`; a lock some
//! panic left poisoned is not trusted, and the key is read again without it. Since the answers are
//! pure, the cache changes no answer, only how often one is made.

use std::sync::Mutex;

use indexmap::IndexMap;

/// The default `limit` of `memoize`.
pub const LIMIT: usize = 256;

/// The function `memoize` returns: `get` is its call.
#[derive(Debug)]
pub struct Memo<V> {
    make: fn(&str) -> V,
    limit: usize,
    cache: Mutex<IndexMap<String, V>>,
}

/// A function of a text key that keeps its last `limit` answers (the module comment).
#[must_use]
pub fn memoize<V: Clone>(make: fn(&str) -> V, limit: usize) -> Memo<V> {
    Memo {
        make,
        limit,
        cache: Mutex::new(IndexMap::new()),
    }
}

impl<V: Clone> Memo<V> {
    /// The answer for `key`: the one kept, else made and kept, the oldest kept one dropped first
    /// when `limit` are kept already.
    pub fn get(&self, key: &str) -> V {
        let Ok(mut cache) = self.cache.lock() else {
            return (self.make)(key);
        };
        if let Some(hit) = cache.get(key) {
            return hit.clone();
        }
        if cache.len() >= self.limit {
            cache.shift_remove_index(0);
        }
        let value = (self.make)(key);
        cache.insert(key.to_owned(), value.clone());
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn length(key: &str) -> usize {
        key.len()
    }

    #[test]
    fn keeps_the_last_answers_and_drops_the_oldest() {
        let memo = memoize(length, 2);
        assert_eq!(memo.get("a"), 1);
        assert_eq!(memo.get("bb"), 2);
        assert_eq!(memo.get("a"), 1);
        assert_eq!(memo.get("ccc"), 3);
        let kept: Vec<String> = memo
            .cache
            .lock()
            .map(|cache| cache.keys().cloned().collect())
            .unwrap_or_default();
        assert_eq!(kept, ["bb", "ccc"]);
        assert_eq!(LIMIT, 256);
    }
}
