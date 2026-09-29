// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The workspace's one strongly-connected-components implementation.
//!
//! Cycle detection over a caller-built index graph is a recurring need across
//! the stack — the slice linker collapses mutually dependent slices into link
//! units, the ShEx structural checker refuses reference-only cycles and
//! cyclic negation — and it is exactly the kind of non-trivial graph algorithm
//! that drifts when it is copied. There is therefore **one** transcription of
//! it, here, in the crate that is a common ancestor of those consumers. It is
//! pure `core`/`alloc` shapes over [`Vec`]: no dependencies, no
//! `std::fs`/`std::io`, wasm32-clean, and it recurses nowhere.
//!
//! The graph is an adjacency list over the dense node space `0..adjacency.len()`.
//! Mapping domain identities (slice IRIs, shape labels) onto that space, and
//! back, is the caller's job — this module knows nothing about RDF.

/// Tarjan's strongly-connected components over an adjacency list, iteratively.
///
/// `adjacency[n]` lists the nodes `n` has an edge to; the node space is
/// `0..adjacency.len()`. Runs in `O(V + E)` time and `O(V)` space.
///
/// # Recursion
///
/// The depth-first search carries its own explicit work stack, so traversal
/// depth costs heap, not call frames. That is deliberate and load-bearing: the
/// graphs are built from caller-supplied input (a slice catalog, a ShEx
/// schema), so a textbook recursive Tarjan would let an input of *n* chained
/// nodes drive *n* levels of recursion and abort the process on stack
/// exhaustion. A degenerate path of 100_000 nodes is covered by the tests.
///
/// # What the result guarantees
///
/// * Every node in `0..adjacency.len()` appears in **exactly one** component;
///   the components partition the node space.
/// * Two nodes share a component if and only if each is reachable from the
///   other. Components are maximal: a chain of mutually unreachable cycles
///   stays as many components, never merged.
/// * A node with a **self-loop** yields a singleton component `[n]` — the same
///   shape an isolated node yields. Component size alone therefore does not
///   distinguish a self-dependency from an independent node; a caller that
///   treats self-dependency as a cycle must consult the edges as well.
/// * The function is deterministic: the same `adjacency` always yields the
///   identical `Vec<Vec<usize>>`.
///
/// # What it does *not* guarantee
///
/// Neither ordering is part of the contract, and callers that need a stable
/// order must impose it themselves (both in-tree consumers sort):
///
/// * **Component order.** Components are emitted as their roots finish, which
///   happens to be a reverse topological order of the condensation, seeded from
///   ascending roots. Treat it as an artifact of the traversal, not an API.
/// * **Member order within a component.** Members come off the Tarjan stack in
///   the order they unwind, which is neither ascending nor insertion order.
///
/// # Panics
///
/// Panics (index out of bounds) if any adjacency entry names a node outside
/// `0..adjacency.len()`. The dense node space is the caller's invariant.
///
/// # Examples
///
/// ```
/// use purrdf_core::graph::tarjan_scc;
///
/// // 0 → 1 → 0 is a cycle; 2 stands alone.
/// let adjacency = vec![vec![1], vec![0], vec![]];
/// let mut components = tarjan_scc(&adjacency);
/// for component in &mut components {
///     component.sort_unstable();
/// }
/// components.sort();
/// assert_eq!(components, vec![vec![0, 1], vec![2]]);
/// ```
pub fn tarjan_scc(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut components: Vec<Vec<usize>> = Vec::new();
    // Members in the order they unwind off the Tarjan stack.
    tarjan(adjacency, |component| {
        components.push(component.iter().rev().copied().collect());
    });
    components
}

/// The strongly connected component of every node, as a dense component id:
/// `index[n]` is the id of `n`'s component, and the ids are `0..k` for `k`
/// components.
///
/// The same Tarjan walk as [`tarjan_scc`], over any adjacency whose rows
/// iterate node indices (`Vec<usize>`, `BTreeSet<usize>`, a slice), for a
/// caller that wants a node's component rather than each component's nodes.
///
/// The ids are a **reverse topological order of the condensation**: for every
/// edge `u → v` between two components, `index[v] < index[u]`. That is a
/// property of Tarjan's algorithm (a component is numbered when its root
/// finishes, after everything it reaches), so a caller propagating along edges
/// visits components in descending id order, and against them in ascending.
///
/// # Panics
///
/// As [`tarjan_scc`]: when an adjacency entry names a node outside
/// `0..adjacency.len()`.
///
/// ```
/// use std::collections::BTreeSet;
/// use purrdf_core::graph::scc_component_index;
///
/// // {0, 1} is a cycle that reaches 2.
/// let adjacency: Vec<BTreeSet<usize>> =
///     vec![BTreeSet::from([1]), BTreeSet::from([0, 2]), BTreeSet::new()];
/// let index = scc_component_index(&adjacency);
/// assert_eq!(index[0], index[1]);
/// assert!(index[2] < index[0]);
/// ```
pub fn scc_component_index<E>(adjacency: &[E]) -> Vec<usize>
where
    for<'e> &'e E: IntoIterator<Item = &'e usize>,
{
    let mut index = vec![usize::MAX; adjacency.len()];
    let mut next = 0;
    tarjan(adjacency, |component| {
        for &member in component {
            index[member] = next;
        }
        next += 1;
    });
    index
}

/// Iterative Tarjan over any adjacency, handing each component to `emit` as
/// the slice of the Tarjan stack it occupies (its root first), in the order
/// the components finish.
fn tarjan<'g, E>(adjacency: &'g [E], mut emit: impl FnMut(&[usize]))
where
    &'g E: IntoIterator<Item = &'g usize>,
{
    const UNSET: usize = usize::MAX;
    let n = adjacency.len();
    let mut index = vec![UNSET; n];
    let mut low = vec![0usize; n];
    let mut on_stack = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut next_index = 0usize;
    // Work frames: a node and the rest of its out-edges.
    let mut work: Vec<(usize, <&'g E as IntoIterator>::IntoIter)> = Vec::new();

    for root in 0..n {
        if index[root] != UNSET {
            continue;
        }
        let mut enter = Some(root);
        loop {
            if let Some(node) = enter.take() {
                index[node] = next_index;
                low[node] = next_index;
                next_index += 1;
                stack.push(node);
                on_stack[node] = true;
                work.push((node, adjacency[node].into_iter()));
            }
            let Some((node, edges)) = work.last_mut() else {
                break;
            };
            let node = *node;
            for &child in edges.by_ref() {
                if index[child] == UNSET {
                    enter = Some(child);
                    break;
                }
                if on_stack[child] {
                    low[node] = low[node].min(index[child]);
                }
            }
            if enter.is_some() {
                continue;
            }
            // Node finished.
            work.pop();
            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == index[node] {
                let start = stack
                    .iter()
                    .rposition(|&member| member == node)
                    .expect("a finishing root is on the stack");
                for &member in &stack[start..] {
                    on_stack[member] = false;
                }
                emit(&stack[start..]);
                stack.truncate(start);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{scc_component_index, tarjan_scc};

    /// Normalize to the one shape assertions can compare: members ascending,
    /// components ordered by their smallest member. The function guarantees
    /// neither, by contract, so every test sorts before it asserts.
    fn normalized(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
        let mut components = tarjan_scc(adjacency);
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort();
        components
    }

    /// Every node lands in exactly one component — the partition property.
    fn assert_partitions(adjacency: &[Vec<usize>], components: &[Vec<usize>]) {
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut total = 0usize;
        for component in components {
            for &member in component {
                assert!(member < adjacency.len(), "component member out of range");
                assert!(seen.insert(member), "node {member} in two components");
                total += 1;
            }
        }
        assert_eq!(total, adjacency.len(), "node count mismatch");
        assert_eq!(seen.len(), adjacency.len(), "not every node was covered");
    }

    #[test]
    fn empty_graph_has_no_components() {
        let adjacency: Vec<Vec<usize>> = Vec::new();
        assert_eq!(tarjan_scc(&adjacency).len(), 0);
    }

    #[test]
    fn isolated_node_is_its_own_component() {
        let adjacency = vec![vec![]];
        let components = normalized(&adjacency);
        assert_eq!(components, vec![vec![0]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn isolated_nodes_do_not_merge() {
        let adjacency = vec![vec![], vec![], vec![]];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 3);
        assert_eq!(components, vec![vec![0], vec![1], vec![2]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn self_loop_is_a_singleton_component() {
        // Matches petgraph: a self-edge does not make the component bigger.
        // The node is its own SCC of size one, which is exactly why a caller
        // that calls self-dependency a "cycle" cannot decide it from size.
        let adjacency = vec![vec![0], vec![]];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 2);
        assert_eq!(components, vec![vec![0], vec![1]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn simple_cycle_and_an_isolated_node() {
        // 0 → 1 → 2 → 0, 3 isolated.
        let adjacency = vec![vec![1], vec![2], vec![0], vec![]];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 2);
        assert_eq!(components, vec![vec![0, 1, 2], vec![3]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn two_disjoint_non_trivial_components() {
        // {0,1} and {2,3} are separate 2-cycles with no edge between them.
        let adjacency = vec![vec![1], vec![0], vec![3], vec![2]];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 2);
        assert_eq!(components, vec![vec![0, 1], vec![2, 3]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn chained_components_are_not_merged() {
        // {0,1} → {2,3} → 4: a one-way edge between two cycles must NOT fuse
        // them, and the acyclic tail stays its own singleton.
        let adjacency = vec![
            vec![1],    // 0 → 1
            vec![0, 2], // 1 → 0 (cycle) and 1 → 2 (one-way into the next SCC)
            vec![3],    // 2 → 3
            vec![2, 4], // 3 → 2 (cycle) and 3 → 4 (one-way tail)
            vec![],     // 4 has no outgoing edges
        ];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 3);
        assert_eq!(components, vec![vec![0, 1], vec![2, 3], vec![4]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn back_edge_to_a_popped_component_does_not_merge() {
        // 0 → 1 → 0 and 2 → 0: node 2 reaches the {0,1} cycle but nothing
        // returns to it, so the `on_stack` guard must keep 2 on its own.
        let adjacency = vec![vec![1], vec![0], vec![0]];
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 2);
        assert_eq!(components, vec![vec![0, 1], vec![2]]);
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn whole_graph_is_one_component() {
        // A 64-node ring plus every chord: everything reaches everything.
        const N: usize = 64;
        let mut adjacency: Vec<Vec<usize>> = (0..N).map(|n| vec![(n + 1) % N]).collect();
        adjacency[0].push(N / 2);
        adjacency[N / 2].push(0);
        let components = normalized(&adjacency);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0], (0..N).collect::<Vec<usize>>());
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn deep_path_does_not_grow_the_call_stack() {
        // The reason this implementation is iterative. A recursive Tarjan would
        // drive 100_000 stack frames here and abort the process; the explicit
        // work stack spends heap instead. Every node is its own component.
        const N: usize = 100_000;
        let mut adjacency: Vec<Vec<usize>> = (0..N - 1).map(|n| vec![n + 1]).collect();
        adjacency.push(Vec::new());
        assert_eq!(adjacency.len(), N);

        let components = tarjan_scc(&adjacency);
        assert_eq!(components.len(), N);
        assert!(components.iter().all(|c| c.len() == 1));
        assert_partitions(&adjacency, &components);
    }

    #[test]
    fn deep_cycle_does_not_grow_the_call_stack() {
        // The same depth, but closed into one giant SCC, so the unwind path
        // (which pops 100_000 members off the Tarjan stack) is exercised too.
        const N: usize = 100_000;
        let adjacency: Vec<Vec<usize>> = (0..N).map(|n| vec![(n + 1) % N]).collect();

        let components = tarjan_scc(&adjacency);
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].len(), N);
        assert_partitions(&adjacency, &components);
    }

    /// Whether `to` is reachable from `from` (a path of zero or more edges).
    fn reaches(adjacency: &[Vec<usize>], from: usize, to: usize) -> bool {
        let mut seen = vec![false; adjacency.len()];
        let mut pending = vec![from];
        while let Some(node) = pending.pop() {
            if node == to {
                return true;
            }
            if !std::mem::replace(&mut seen[node], true) {
                pending.extend(&adjacency[node]);
            }
        }
        false
    }

    #[test]
    fn the_component_index_is_mutual_reachability_in_reverse_topological_order() {
        let mut state = 0x5CC0_0000_0000_0001_u64;
        for n in 0..24_usize {
            for _ in 0..20 {
                let adjacency: Vec<Vec<usize>> = (0..n)
                    .map(|_| {
                        (0..n)
                            .filter(|_| {
                                purrdf_testkit::rng::splitmix64_next(&mut state).is_multiple_of(7)
                            })
                            .collect()
                    })
                    .collect();
                let index = scc_component_index(&adjacency);
                let count = tarjan_scc(&adjacency).len();
                assert!(index.iter().all(|&c| c < count));
                for u in 0..n {
                    for v in 0..n {
                        let mutual = reaches(&adjacency, u, v) && reaches(&adjacency, v, u);
                        assert_eq!(index[u] == index[v], mutual, "{adjacency:?} {u} {v}");
                    }
                    for &v in &adjacency[u] {
                        assert!(index[v] <= index[u], "{adjacency:?} {u} -> {v}");
                    }
                }
            }
        }
    }

    #[test]
    fn the_component_index_reads_set_rows_and_deep_paths() {
        let sets: Vec<BTreeSet<usize>> =
            vec![BTreeSet::from([1]), BTreeSet::from([0, 2]), BTreeSet::new()];
        let index = scc_component_index(&sets);
        assert_eq!(index, [1, 1, 0]);
        const N: usize = 100_000;
        let path: Vec<Vec<usize>> = (0..N)
            .map(|n| if n + 1 < N { vec![n + 1] } else { vec![] })
            .collect();
        let index = scc_component_index(&path);
        // The tail finishes first.
        assert_eq!(index[N - 1], 0);
        assert_eq!(index[0], N - 1);
    }
}
