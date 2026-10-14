Return typed capacity errors and compose sparse segmented snapshots

Closes #490
Closes #495

Core builders, importers, mutable views, merges, event scopes and provenance
tables return typed capacity diagnostics with the original space, attempted count
and limit. Duplicate-at-capacity and rollback preserve prior data. Global text
ranges and native Datalog IDs support 64-bit addresses; compact dataset-local
IDs keep their original layout and use checked ingress.

The layered segmented stack publishes authenticated replacement descriptors
atomically, pins snapshots without rebuilding a dictionary, and preserves
chronological effective-set semantics. Local rewrites remove lexical and datatype
bytes from persisted successor segments while old snapshots remain readable
until their original owners are released. Persistence stays caller-owned.

Host dictionary and identity interfaces carry the original tagged/high-bit/triple
IDs through real SPARQL joins and Datalog compilation, evaluation, chase and proof.
The original membership implementation compares both full identity columns; its
bounded count reduction does not narrow or remap either ID.

Validation covers every listed logical capacity boundary, duplicate/refusal/healthy
neighbors, wide offset arithmetic, eight branded ID spaces, public producers,
1,000 header-only sources, tenfold term/removal snapshot controls, generated
publication parity, 48 public queries and persisted lexical purge. Native,
actual scalar/SIMD and installed host results remain attributed to their inputs.
The original full local gate and reviewed affected checks qualify the settled
implementation. All 82 panic-census controls pass; forty source-backed theorem
rows retain their original sinks and authorities, with no weakened exemption.

The original complete hosted producer passed 112 sites on seven configurations.
Its generated document was imported without changing manual text or acceptance
floors, FMA limits or relaxed-instruction restrictions. All current hosted checks
passed after the latest host integration; declared optional jobs were skipped.

Latest host integration preserves both normalized WASM stack diagnostics and typed
reasoning context in its sole conflict. Evaluator/WASM/C all-target compilation
and six operation controls pass. The final combined package passed all 458 Node
tests, TypeScript, packed installation and the full-width identity ABI controls.
The current remote main is included in this branch. The clean integration tree
is `6febc4b849e3755a0b9832636275e4f9af1a92c7`; no additional base behavior
requires an unexecuted qualification.

Original independent implementation, source-proof and completion reviews are
retained with their scope. The PR review service skipped the oversized initial
diff; its success check is not called substantive approval. The owner maps current
execution and integration under the zero-subagent workflow. There are no submitted
reviews, inline findings or unresolved threads. The suggested split/base workaround
is declined because the pair shares its accepted identity and storage interfaces;
its original independent source reviews remain the substantive review evidence.
No requirement is
moved to another ticket or published as an incomplete precursor.

Normal installed tools, standard directories and signing/hooks were used.
No timing performance claim, exclusive campaign, reduced corpus, altered compiler
or cache workaround is introduced. The authoritative plan is
.stage/typed-capacity-errors-instead-of-u32/plan.md; select that Stage directory
for separate evidence archival.

Defect-Class: none
