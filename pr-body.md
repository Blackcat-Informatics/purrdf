Dataset updates can now use `TextIndex::with_delta(next, added, removed)` instead
of rebuilding the complete index. It validates the effective literal multiset
against the indexed generation, analyzes changed documents and merges their
postings into retained lexical and surface tables. Identifiers, source evidence,
fingerprints and exact BM25F statistics match fresh construction. Unchanged
token, gram, pronunciation and alignment buffers are reused. Preserving an older
index requires an explicit clone; source scans, sorting, hashing and table merges
still visit retained data.

Integration exposed a remaining native anyURI consumer defect: the newly
recognized primitive bypassed its old cast classification. Both cast directions
now preserve the primitive table, URI-to-string conversion reads the native
collapsed value, and malformed URI values refuse conversion. The existing native
parser owns whitespace mapping. Stale unsupported-URI descriptions are corrected.
The native datatype/range implementation already landed in the linked XSD work;
the actual downstream replacement and deletion of URI special cases landed in
[GMEOW PR 1838](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1838)
and [PR 1839](https://github.com/Blackcat-Informatics/gmeow-ontology/pull/1839).

Validation:

- Text: 353 tests in 33 suites, including 195 chained comparisons with fresh
  builds, exact analysis counts, retained allocations and public native/WASM
  ranking and surface execution.
- Cast repair: all 23 native operator-table tests and three governed cases on
  native and actual WASM pass; the original numeric transcript is unchanged.
  Strict affected all-target compilation passes with warnings denied.
- The original full local run passed every workspace target and doc test except
  the old cast refusal. Its corrected target, the separate JSON consumer, core
  hygiene and release WASM build now pass; the failed original command is not
  represented as a pass. Normal hooks and publication pass through `ede07f859`.
- Final hosted CI has 50 successful checks and five configured skips. Material
  review is resolved. Its tested merge exactly matches the predicted combination
  of the final head and current main. The inventory reflects that combined tree,
  including main's additional role case: 138 named cases and 143 executions.
  No timing measurements were used.

Closes #538.
Closes #571.
