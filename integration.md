# Current numeric integration assessment

Fetched origin/main through normal git fetch; merge-tree --write-tree
--no-messages origin/main HEAD exits0. Current draft candidate combines main
6f1199b1d and head007584c86, treef9e6d39ccfe3cf46c0880e21ee103bc53e53fefc.
Refresh before actual landing; no input-lock claim.

Branch began at09e344f733. Main's subsequent delivery674 changes columnar
source-error transport, a columnar dev-only allocator edge/lock entry, native
columnar tests/docs and a disjoint WASM test selection. It changes no XSD exact
arithmetic, prepared text, float kernels, counter/allocator implementation,
governors, shared hashing, compiler or profiles. Numeric branch changes its
existing XSD methods, benches/governance and numeric documentation, plus a
separate WASM allocation case. The Makefile hunks combine cleanly and route both
sets of tests. No shared producer/API/dependency closure is newly changed by
the combination. This is source-based integration assessment, not a fresh
combined-runtime result.

Hosted run39712077142 and docs39712077158 are terminal on current head;
all50checks pass with fiveconfiguredskips, no pending/failing result.
Completed msrv job119605859959 logs explicitly checkout test mergead70fc52d12
and run cargo check --workspace --lib --locked. Immutable forge commit data shows
parents6f1199b1d/007584c86 and treef9e6d39c, matching the assessed candidate.
raw/ci-checkout-projection.log retains the exact relevant lines from that job,
obtained through gh api --allow-escape-sequences after the response guard refused
ANSI without it; failed first log-fetch attempts are not qualification.
That actual compiler check identifies the combined tree; the final hosted matrix
now also completes on its current PR association. Fresh base fetch and merge-tree
assessment still producef9e6d39c with head007584c86. Full paginated feedback is
unchanged/resolved and the final PR-body/check refresh remains green. Actual
ghprsq inputs/result and durable archive must be verified after publication.
