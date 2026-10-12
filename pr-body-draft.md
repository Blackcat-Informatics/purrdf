The declared compiler's Shapes check cannot infer the SCC adjacency type, and
the Arm numerical replay refuses its existing `numeric` backend family before
executing any vectors. Make the known adjacency type explicit and register the
existing family in the shared dispatch registry. Arithmetic, traversal, resource
limits and backend availability checks retain their existing behavior.

The mandatory workspace gate also encounters an eight-argument geographic scan.
Document its distinct inputs with a function-local Clippy expectation, preserving
the scan and both production callers. Adapt the retrieval test wrapper to the
current admitted cursor API, retaining both inner and outer native owners.
Declare the identical admitted-to-resident cursor bridge once at its existing
property-function home and use it in HNSW and that test wrapper.

Closes #536.

Verified locally with supplied tools and standard directories:

- Numeric registry admission and unknown-path refusal: 1 passed.
- Original certified numerical replay: 9 passed, frozen digests unchanged.
- Original forced-path product/endpoint control: 1 passed.
- Original Shapes chain/cycle admission control: 1 passed.
- Strict all-target Clippy for hash, XSD, Shapes and geo: passed.
- Original retrieval real-producers target after its API adaptation: 21 passed.
- Unchanged helper census: passed.
- Strict evaluator/HNSW/retrieval all-target checks: passed.
- Original HNSW library controls: 77 passed.
- Both normal commit hooks and source publication through `ae0d71918`: passed.

Publication draft only. The first full local gate failed on the retrieval
adapter's old cursor type; that correction passes focused checks. The shared bridge passes census, strict and original runtime checks. The required
full-gate rerun is live; its result remains pending. The unchanged hosted MSRV 1.98 and actual Arm portable/NEON executions
remain required; native results do not establish either. No tooling, workflow,
compiler, manifest, cache or acceptance-threshold change is included.
