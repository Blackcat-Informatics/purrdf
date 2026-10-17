# Exact text-index delta updates

The complete contract is an actual TextIndex delta path, with added and removed
literal rows, whose results match a fresh build. BM25F, Fixed arithmetic,
partitioning, source evidence and ranking profiles keep their existing laws.
The corpus includes both RDF layers and both physical carrier profiles.
No partial surface-only PR or closure satisfies this contract.

The current implementation discards raw rows and lexical analysis after build.
SurfaceIndex stores sorted flat gram/phonetic postings and source-aligned
evidence. Re-running assemble over all retained analyses would repeat the
reported expensive computation and is not an incremental implementation.

Use the existing analyzer/scorer once, and implement ordered updates at the
existing physical homes. An owned update may move buffers; it must not retokenize
unchanged documents, regenerate their grams or rerun their phonetic encodings.
Canonical identifiers, both fingerprints, phrase and auxiliary answers must
agree with a fresh index. The current canonical fingerprint requires traversing
retained source/index data; distinguish this cost from rebuilding projections.
Do not claim sublinear total work without evidence. Preserve older snapshots.

## Task 1: Update surface postings and evidence

Extend FlatPostings at its existing home with an ordered remap/merge. Reuse
unchanged source text, term spelling, span shapes, codes and gram positions.
Surface document replacement removes every old source for an affected document,
merges the analyzed additions and retains canonical ordering/evidence. Validate
analyzer compatibility and ID maps. Compile this coherent production slice
before broader integration. Compare actual substring/phonetic results and
generation against fresh construction, including deletions and overlaps.

## Task 2: Wire the actual TextIndex delta path

Admit added/removed effective literal rows, validate their relation to the source
generation, and analyze only affected documents, including their retained rows.
Merge their lexical postings, token streams and Han occurrences into the existing
index. Apply exact subtraction/addition to partition and field statistics.
Wire the surface update through the same document remaps and rebind the final
generation. Keep graph/language partitioning, routing, stable scoped term identity,
coverage, auxiliary-only documents and typed failures. No default corpus cache
or new runtime dependency. Complete the chosen source/delta API against the
actual DatasetView and existing binding/digest implementation; do not accept a
delta that silently certifies a mismatched source.

The working source API is an owned update against the next immutable dataset
plus effective added/removed DatasetView inputs. Reconstruct the selected prior
source rows by reversing the delta and compare their digest with the index's
source_fingerprint before modifying it. Reuse collect_rows and the existing
digest recipe; factor that recipe for borrowed ordered rows rather than writing
another hash preimage. Coverage comes from the actual next source, including
nonliteral statements and all three RDF layers. This requires a source walk and
canonical hashing, but no permanent copy of raw literals in every TextIndex.

Analyze the complete next-source rows of affected document keys only. Assemble
that partial index through the existing analyzer and ranking profile, then merge
it into the retained core tables with canonical document/partition/term maps.
Postings compare by (partition, document), not document alone. Retained token
streams need only term-ID rebasing; Han occurrences need document-ID rebasing.
Sum document/field counts and lengths from the retained and replacement document
facts through the existing partition/field homes, preserving auxiliary-only
partitions. This metadata walk is deliberate: it avoids a second implementation
of ranking statistics without reanalyzing retained text. Surface document IDs
must follow the same monotone
maps before the ordered addition path is used. Recompute the existing canonical
index digest and bind the final generation once. This route is not implemented
or qualified by the surface tests; complete it before creating the PR.

## Task 3: Qualify and deliver the whole contract

Fresh-build oracle tests cover insert/retract/replace, empty/repeated/no-op
changes, multi-literal documents, annotations, named/default/graph-free carriers,
language/direction, ranking fields, lexical/Han/substring/phonetic/phrase evidence,
identities and invalid deltas. Fixed-work controls prove unchanged documents are
not analyzed and unchanged grams/codes are not rebuilt. Record allocation/layout
costs, source scans and old-snapshot ownership explicitly. No timing measurement.

Use standard host tools, eight Cargo jobs/four test threads and 16 GiB per job;
one Cargo invocation at a time. Focused checks during development, affected native
and wasm execution, then the required final gate on settled source. No agents or
fabricated independent review. Obtain hosted material review for the complete
PR, resolve findings, satisfy CI, merge only with ghprsq, verify closure/mirror,
and remove the delivered branch. Update the root REWORK_LIST.md at transitions.

## Evidence and ownership

Stage brief reports no earlier delivery branch, commit or PR for this issue.
The source inspection confirms the all-corpus construction described above.
Related scoring/dictionary fixes are already merged and provide existing homes,
not delta completion. Prior art and plan judgment are owner-performed because
subagents are disabled. Independent implementation acceptance remains pending.
