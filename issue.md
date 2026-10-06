# Issue #442: owl_dl: Graph::step scans every edge per neighbour query (O(nodes×edges) rounds)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Evidence
gmeow (#1757 / gmeow#1803) delegates its class-model consistency question to `Reasoner::consistency`. On its production class-model worlds, `purrdf consistency` behaves as follows:

| World | Triples | Result |
|---|---|---|
| `graph/imports` | 1,366 | decided in 26 ms |
| `graph/logic` | 21,317 | **no answer after 39 min** (single thread, under 10 MB RSS) |
| `rl-default`, with its derived `owl:Nothing` markers removed | 83,233 | **no answer after 10 min** |

A symbolized profile of the `graph/logic` run (90 s, `perf record -g`) attributes **97.4%** of samples to `owl_dl::graph::Graph::step`:
- 54% `BTreeSet<(u32, bool)>::contains` (the achiever probes);
- 29% `find` (union-find chain walks);
- 21% node-vector indexing.

## Cause
`Graph::neighbors` answers "which nodes does `x` reach via role `r`" through `step`, which **walks the whole global `State::edges` vector**. For every edge it calls `find` twice and probes the achiever set twice. Each neighbour query is therefore O(|edges|), and the rules issue one per node per role atom per round, so a round costs O(|nodes| × |edges|). Over an ABox with tens of thousands of role edges, that alone is the grind.

## Fix
Keep a per-node **adjacency index** of edge indices, the indexed-extension-table idea used in HermiT's hypertableau:
- `State::push_edge` appends to both endpoints' lists. Edge indices only grow, so each list stays sorted.
- The one merge site folds the discarded node's list into the kept root's with a sorted merge.
- `step` walks only `x`'s class's edge indices, **in ascending edge order**. That is exactly the order the global scan visits them in, so neighbour output order, search, verdicts, certificates and proof bytes are all unchanged.
- The `Work` charge stays `edges.len()` per call, so every pinned step/work ledger and cap is byte-identical. Re-basing the meter on degree would be a separate decision.

## Acceptance
- **Byte-identical behaviour:** the step ledgers, the 9,800-case differential oracle and the W3C conformance runs are unchanged.
- **Bench:** a new `consistency` bench family with a large role-edge ABox shows the per-round cost scaling with degree, not edge count.
- **gmeow's production worlds** decide, or answer `unknown` honestly on a cap, rather than grinding.

## Comments (0)

