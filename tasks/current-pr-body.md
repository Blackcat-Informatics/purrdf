Closes #492

Add an explicit graph-free RDF 1.2 carrier with 12-byte triple/annotation rows,
8-byte reifier rows and SPO/POS/OSP access. Its named-graph requests fail with a
typed diagnostic, including empty and nested query paths. The existing carrier
keeps its named-graph behavior.

The real rule compiler, store, scheduler, chase and proof paths use arity-three
atoms and predicate-only partitions. Public RL inference returns the compact
model. Canonical output agrees with the corresponding default-graph model;
text keys and partitions use a distinct profile without a graph coordinate.
All paths share the original term, scoring and query implementations.

Current main is integrated. Five public carrier/canonical controls, three real
queries, four triple rule controls and strict all-target checks for nine packages
pass. Fixed-work layout controls retain 32N base and 20N lazy-index bytes.
Official default-graph query replay retains 590 passes and 45 typed named-graph
refusals, with unchanged inputs. Actual portable controls retain their scope,
including the current canonical relabeling case. Source proofs and the unchanged
helper gate, metadata, English/Chinese books and six rendering gates pass.

The final `make check` passes, including all publishable release WASM libraries.
All fifteen current portable controls pass through the original runner. The
three findings from the earlier gate are repaired: stable rule-head errors,
complete source inventory and the text identity's surface-presence field. Both
surface modes preserve full query answers and source identity. Normal commit
hooks and publication pass. Scoped independent source reviews are retained,
with current consumer evidence and an owner assessment of the integration.

Hosted checks and final integration review remain required. No throughput or
latency claim is made; physical measurements count bytes and allocations.
