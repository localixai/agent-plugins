//! The core's counterpart of `src/foundation/plain-data.ts` at sha256:ab47ddf53ba7, which stays TypeScript: a walk over a
//! host value that keeps its containers by identity (a cycle, a shared sub-structure), which no
//! value crossing the boundary can be (docs/RUST.md, "What stays TypeScript").
//! A value's depth, nodes and characters against limits, for the core's own values, which hold no
//! cycle and share nothing, so each container is reached once.

use serde::Serialize;

use crate::js::{JsValue, code_unit_length};

/// What a walk of a value found. `depth` counts nested arrays and objects: a scalar is 0, `[1]` is
/// 1, `[[1]]` is 2.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct PlainDataStats {
    /// Arrays and objects nested.
    pub depth: usize,
    /// Arrays, objects and scalars together.
    pub nodes: usize,
    /// Characters of every string, keys included.
    pub chars: usize,
}

/// The limits a value is held to; `None` is no limit.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlainDataLimits {
    /// `maxDepth`.
    pub max_depth: Option<usize>,
    /// `maxNodes`.
    pub max_nodes: Option<usize>,
    /// `maxChars`.
    pub max_chars: Option<usize>,
}

/// Which limit a value went past, first found.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlainDataProblem {
    /// `depth`.
    Depth,
    /// `nodes`.
    Nodes,
    /// `chars`.
    Chars,
}

fn limit_problem(stats: PlainDataStats, limits: PlainDataLimits) -> Option<PlainDataProblem> {
    let over = |limit: Option<usize>, value: usize| limit.is_some_and(|limit| value > limit);
    if over(limits.max_depth, stats.depth) {
        return Some(PlainDataProblem::Depth);
    }
    if over(limits.max_nodes, stats.nodes) {
        return Some(PlainDataProblem::Nodes);
    }
    over(limits.max_chars, stats.chars).then_some(PlainDataProblem::Chars)
}

/// Walks `value` without recursion and stops at the first limit it goes past: the problem, or
/// the stats of the whole value.
///
/// # Errors
///
/// The first limit `value` goes past.
pub fn walk_plain_data(
    value: &JsValue,
    limits: PlainDataLimits,
) -> Result<PlainDataStats, PlainDataProblem> {
    let mut stats = PlainDataStats::default();
    let mut stack: Vec<(&JsValue, usize)> = vec![(value, 0)];
    while let Some((item, above)) = stack.pop() {
        let container = matches!(item, JsValue::Array(_) | JsValue::Object(_));
        let own = above + usize::from(container);
        stats.depth = stats.depth.max(own);
        stats.nodes += 1;
        if let Some(text) = item.as_str() {
            stats.chars += code_unit_length(text);
        }
        if let Some(problem) = limit_problem(stats, limits) {
            return Err(problem);
        }
        match item {
            JsValue::Array(items) => stack.extend(items.iter().rev().map(|child| (child, own))),
            JsValue::Object(members) => {
                stats.chars += members
                    .keys()
                    .map(|key| code_unit_length(key))
                    .sum::<usize>();
                stack.extend(members.values().rev().map(|child| (child, own)));
            }
            JsValue::Undefined
            | JsValue::Null
            | JsValue::Bool(_)
            | JsValue::Number(_)
            | JsValue::String(_) => {}
        }
    }
    Ok(stats)
}

/// The first limit `value` goes past, or `None` when it fits.
#[must_use]
pub fn plain_data_problem(value: &JsValue, limits: PlainDataLimits) -> Option<PlainDataProblem> {
    walk_plain_data(value, limits).err()
}

#[cfg(test)]
mod tests {
    use super::{PlainDataLimits, PlainDataProblem, PlainDataStats, walk_plain_data};
    use crate::js::JsValue;

    #[test]
    fn counts_depth_nodes_and_characters_keys_included() {
        let value = JsValue::Object(
            [("ab".to_owned(), JsValue::Array(vec![JsValue::from("xyz")]))]
                .into_iter()
                .collect(),
        );
        let stats = walk_plain_data(&value, PlainDataLimits::default());
        assert_eq!(
            stats,
            Ok(PlainDataStats {
                depth: 2,
                nodes: 3,
                chars: 5
            })
        );
        let limits = PlainDataLimits {
            max_depth: Some(1),
            ..PlainDataLimits::default()
        };
        assert_eq!(
            walk_plain_data(&value, limits),
            Err(PlainDataProblem::Depth)
        );
    }
}
