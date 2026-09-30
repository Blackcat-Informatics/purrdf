// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The one breadth-first shortest-path search over a dependency graph.
//!
//! Both the rule scheduler (rule indices) and the semi-naive stratifier (predicate
//! symbols) report a shortest cycle in their diagnostics, so the path must be a pure
//! function of the graph. Adjacency is an ordered set and every container is ordered, so
//! among equal-length paths the one whose earlier hops are lexically smallest wins.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The shortest `from -> … -> to` path through `depends`, inclusive of both ends.
///
/// A breadth-first search over lexically ordered adjacency, so the path is a pure function
/// of the graph rather than of a traversal accident. `from == to` yields the one-node path
/// `[from]`; an unreachable `to` yields `None`.
pub(crate) fn shortest_path<K: Ord + Clone>(
    depends: &BTreeMap<K, BTreeSet<K>>,
    from: &K,
    to: &K,
) -> Option<Vec<K>> {
    let mut parent: BTreeMap<K, K> = BTreeMap::new();
    let mut queue: VecDeque<K> = VecDeque::from([from.clone()]);
    let mut seen: BTreeSet<K> = BTreeSet::from([from.clone()]);
    while let Some(node) = queue.pop_front() {
        if node == *to {
            // Walk the parent chain back to `from`, then reverse it.
            let mut path = vec![node.clone()];
            let mut cursor = node;
            while cursor != *from {
                cursor = parent[&cursor].clone();
                path.push(cursor.clone());
            }
            path.reverse();
            return Some(path);
        }
        for next in depends.get(&node).into_iter().flatten() {
            if seen.insert(next.clone()) {
                parent.insert(next.clone(), node.clone());
                queue.push_back(next.clone());
            }
        }
    }
    None
}

/// The stratum relaxation every stratification in this crate runs: for each edge
/// `(head, body, closed)` the head's stratum must reach the body's, plus one when the edge
/// is `closed` (negative or aggregating), repeated until no stratum moves.
///
/// `get` and `set` read and write the caller's stratum store (a vector over rule indices,
/// a map over predicate symbols), so the one loop serves both. `max_passes` bounds the
/// passes: `None` runs to the fixpoint (the caller has proved it terminates), `Some(n)`
/// gives up after `n` passes that still moved something. Returns whether it reached the
/// fixpoint, so `false` means a closed edge sits inside a cycle.
pub(crate) fn relax_strata<S, K, I>(
    state: &mut S,
    edges: impl Fn() -> I,
    get: impl Fn(&S, &K) -> usize,
    mut set: impl FnMut(&mut S, &K, usize),
    max_passes: Option<usize>,
) -> bool
where
    I: IntoIterator<Item = (K, K, bool)>,
{
    let mut passes = 0usize;
    loop {
        let mut changed = false;
        for (head, body, closed) in edges() {
            let need = get(state, &body) + usize::from(closed);
            if get(state, &head) < need {
                set(state, &head, need);
                changed = true;
            }
        }
        if !changed {
            return true;
        }
        passes += 1;
        if max_passes.is_some_and(|limit| passes > limit) {
            return false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(edges: &[(u32, u32)]) -> BTreeMap<u32, BTreeSet<u32>> {
        let mut g: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for &(a, b) in edges {
            g.entry(a).or_default().insert(b);
        }
        g
    }

    #[test]
    fn finds_the_shortest_path_not_the_first_found() {
        // 1 -> 2 -> 3 -> 4 is longer than 1 -> 5 -> 4.
        let g = graph(&[(1, 2), (2, 3), (3, 4), (1, 5), (5, 4)]);
        assert_eq!(shortest_path(&g, &1, &4), Some(vec![1, 5, 4]));
    }

    #[test]
    fn no_path_is_none() {
        let g = graph(&[(1, 2), (3, 4)]);
        assert_eq!(shortest_path(&g, &1, &4), None);
        assert_eq!(shortest_path(&g, &9, &1), None);
    }

    #[test]
    fn start_equal_goal_is_the_single_node_path() {
        let g = graph(&[(1, 1), (1, 2)]);
        assert_eq!(shortest_path(&g, &1, &1), Some(vec![1]));
        assert_eq!(shortest_path(&graph(&[]), &7, &7), Some(vec![7]));
    }

    #[test]
    fn equal_length_ties_break_on_the_lexically_smallest_hop() {
        // Two length-2 routes 1 -> 2 -> 9 and 1 -> 3 -> 9: the smaller middle hop wins,
        // however the edges were inserted.
        let a = graph(&[(1, 3), (3, 9), (1, 2), (2, 9)]);
        let b = graph(&[(1, 2), (2, 9), (1, 3), (3, 9)]);
        assert_eq!(shortest_path(&a, &1, &9), Some(vec![1, 2, 9]));
        assert_eq!(shortest_path(&b, &1, &9), Some(vec![1, 2, 9]));
    }

    #[test]
    fn works_over_string_keys() {
        let mut g: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        g.entry("p".into()).or_default().insert("q".into());
        g.entry("q".into()).or_default().insert("p".into());
        assert_eq!(
            shortest_path(&g, &"p".to_owned(), &"p".to_owned()),
            Some(vec!["p".to_owned()])
        );
        assert_eq!(
            shortest_path(&g, &"q".to_owned(), &"p".to_owned()),
            Some(vec!["q".to_owned(), "p".to_owned()])
        );
    }
}
