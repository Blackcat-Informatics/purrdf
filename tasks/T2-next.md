# Next production unit

Historical handoff at 078f9c973; superseded by T2-owner-review.md and the current
validation.md. The cyclic-order suggestion below was rejected after inspecting
the native frozen-order certificate; the current implementation uses masks.

Current source is clean and pushed at 078f9c973. Both published units pass their
scoped checks; no full suite was launched. Plan Task 2 is still incomplete.

Implement the actual component and annotation orders, not another standalone
lookup. Reuse TripleOrder's three cyclic key orders and the existing ordinal
builder. FrozenTerms holds checked InternedTerm::Triple component IDs; shared
ordering logic should serve both two-field and graph-bearing reifier rows.
All three bound-axis subsets must address matching ranges, with no empty-table
payload allocation. Include every added order in native warming/certificates.

Expose conservative graph-pruned candidate defaults plus actual resident
overrides through DatasetView, and forward through Arc. Partial nested triple
positions retain residual bind_row validation; use existing query_id to read
bound components from each driver, without mint/remap. The exact-target path
is already wired and must be preserved. PathGraph has another StatementProbe
constructor and supplies no component restriction.

Planner currently ignores statement rows: visit_access_costs and
base_cardinality in bgp.rs consult quad statistics. Task 2 must add the new
statement access estimates and honest indexed-versus-scan EXPLAIN. Do not
pretend a filtered callback's low count proves no physical scan. Components
remove candidates before bind_row fuel charging, unlike the already-filtered
exact-target unit; reconcile the versioned governor schedule/receipts.

Then prove the actual ten-million/six-annotation population, portable runtime,
RDF 1.2 parity and final gates. Current API/code coverage is not whole-issue
qualification. No independent review has been fabricated; actual forge source
review remains a publication requirement. The separate docs branch is clean,
published at e42de2c7f and has remaining meaning vetting recorded in its own Stage.
