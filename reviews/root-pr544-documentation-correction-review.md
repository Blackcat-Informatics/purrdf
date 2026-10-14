# Hosted documentation correction review

Scoped source PASS. The actual hosted doc job failed on two unresolved IriError
links in GlobalDictionary methods that return imported IriReadError. Their two
error paragraphs now name its Lexical and Storage variants. The first local
standard make doc cleared core and exposed the previously unreached Datalog
plan's unresolved TermId link; that link now explicitly names crate::id::TermId,
matching the neighboring PartitionId link. No implementation body changed.

The corrected original documentation scope passed standard make doc in48944,
terminal0, producing all49 workspace documentation outputs. The original hosted
and first local failures remain retained. Runtime behavior and the full local
gate on ab743e1d1 remain applicable by the body-preserving delta; no redundant
runtime test or full-suite execution is required for these documentation edits.

The actual stale index reports40 errors with the same149 constructor-closure
paths. Its demonstrated metadata changes are constructor aggregate
60cd43cc38bd7131 to49c7c48bf647382f; seven global.rs authority references
ee49b9f69b8a6429 tobbf736546130634d; and value_index's unchanged sink
921e8adc54613316 moving from888 to890. find remains432. The Datalog plan doc
link is not a proof authority. Root approved only these identities and locations
for refresh; every theorem, class, sink identity and owning constructor body must
remain unchanged. The failed index is preserved in hosted-doc-stale-index.json.

Final refreshed index/helper24646 passed, errors[] and unchanged82/118. Normal
hook78382 and push76582 passed at10316f2c1. Root's final ledger diff readback
confirms exactly40 aggregate strings, seven global source references and one
888-to890 location update, with no other proof-field change. Required
current-head hosted acceptance remains open; earlier failed jobs retain their
original verdicts.
