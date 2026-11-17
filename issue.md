# Issue #1367: Correct three documentation passages after the switch to the labelling

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

Three passages no longer match the code after the switch to the nested-refinement labelling. Each is documentation only.

1. `docs/MIGRATION-5.0.md` does not list one changed signature. `Canonicalized::presentation()` returned `CanonPresentation` in 4.x and now returns `Option<CanonPresentation>` (`crates/rdf-core/src/ir/canon/result.rs`). Add it to the table of changed signatures, with the migration for a 4.x caller.
2. `docs/WASM_TESTING.md` (around lines 113 to 119) describes the selected case in `canonicalization-host-error.test.mjs` as one that transports an exact maximum `u64` policy as text, and says native Rust owns the reserved-vocabulary refusal. The case now sends the bound `0`, expects the labelling-limit refusal with its native cause, and checks that the unlimited neighbour succeeds. Rewrite the entry to describe that case.
3. The `PipelineBundle` rustdoc (`crates/rdf-core/src/ir/pipeline_bundle.rs`, line 36 and the `digest()` doc near line 1196) calls the dataset section "the canonical N-Quads hash". The fold absorbs the canonical document text. Correct the wording. The file is under `ir/`, so refresh the ledger constructors identity afterwards.

Acceptance: the three passages match the code, and `scripts/check-doc-claims.py` and the commit hooks pass.


## Comments (0)

