# Decisions (lane 1351)

Each is reversible and in scope. Format: decision, reason.

1. Stacked on unmerged engine commit da17e2c1219ac918dcbe9e2ee00343b08fa46e58, merged as 6753a63eb.
   The only conflicts were `helpers-ledger.toml` capacity-proof hashes (main side kept, refreshed last
   in step 12) and two additive CHANGELOG entries (both kept).
2. Legacy family is a module `purrdf_core::rdfc12` (re-exported as `purrdf_rdf::rdfc12`) that names the
   existing functions, constants and types. The definitions stay in `ir/canon.rs`; moving 9,000 lines
   would change nothing observable and the file is the oracle for stored v4 digests. Reason: no byte moves
   in step 1; step 4 removes the legacy names from the crate root so the root family can be the new engine.
3. Step 1 also repoints `rdf12_canon_profile.rs` (the legacy profile's frozen corpus test) at `rdfc12`,
   since by D3 `vectors/rdf12-canon` stays frozen as the legacy profile's corpus.
4. Step 1 shared-helpers gate: `scripts/check-shared-helpers.py` fails (80 stale capacity-proof source hashes) after the
   engine merge; not refreshed until step 12 (AGENTS: refresh last). Commit hook does not run it.
5. Step 2 added `label_view`, `label_view_with_memory`, `Scope`, `Labelling` to the engine's view.rs (additive; graph scope
   needed lowering control). Possible merge friction with the engine branch; tell the engine reviewer.
6. D6 document: statement lines only, in canonical-sequence order; `empty-graph` and `record` lines have no N-Quads
   spelling and are not written (digest binds them); annotation count is not shown in text; two lines of different
   kinds that spell one statement both appear. Golden: crates/rdf-core/tests/golden/canonical-document.nq.
7. New default family naming (step 4): no `CanonHash` argument; `try_canonicalize`, `try_canonicalize_view`,
   `try_canonicalize_graph_view`, `try_canonical_digest_view`, `try_graph_digest_view`, `try_canonicalize_digest_view`,
   checked (FallibleDatasetView) variants `*_checked_*` replace the `flat` names. Digest type: `purrdf_hash::hex::Digest32`
   (not ContentDigest: it is not a SHA-256 content identity). Legacy names stay only under `rdfc12`.
8. `Canonicalized::presentation()` returns `Option<CanonPresentation>`: `None` for the line-sequence document; legacy
   functions report Some(Overlay|FlatAssertion). New presentation constants: `line-sequence`, version 1.
   "The flat presentation's version changes" is read as: the pin (purrdf-rdfc12, 3, flat-assertion, sha256) is replaced by
   (nested-refinement-labelling, 1, line-sequence 1); legacy constants keep their values under rdfc12 (the legacy
   algorithm is unchanged). Flag to coordination.
9. Host bound (D5): SearchPolicy::Finite(n) -> engine Limit::new(n); Unlimited -> Limit::ENGINE (the engine's own
   documented limit, 2^34 units). Receipt work records the engine's explored limited units under SearchEvent::LabellingUnit
   (SearchWork events array grows from 5 to 6 entries, host JSON follows).
10. Between steps 4 and 7 downstream crates (rdf, entail, shapes, slice, hosts) do not build: step 4 changes purrdf-core's
    root names; steps 6 and 7 update dependents (as the plan orders). purrdf-core checks are the evidence of step 4.
11. Root `CANON_CORPUS_POLICY = SearchPolicy::Finite(1_000_000)` replaces the root `RDFC_V2_CORPUS_POLICY`, which now exists only
    under `rdfc12`.
12. `tests/canon_search_differential.rs` is deleted: it pinned the old search, which no root entry point reaches.
13. PACK keeps `rdfc12::try_canonical_sha256_view` (format v1) until step 6 adds format version 2.
14. Engine invariant mappings: `DocumentError` -> `CanonError::Refused(Refusal::Input{reason: InputReason::Kind})`;
    `CanonError::ForeignId` -> `SourceInconsistent`.
15. A composite (`cdt:List`/`cdt:Map`) literal is a literal in the new family: the canonical line sequence carries it as one
    literal term, so a blank label spelled in its text is text, not a node. `canonical_relabel*` copy the text as written
    (`TermMapper::composite_text_is_verbatim`), a blank no line holds keeps its label, and `datasets_isomorphic` no longer
    pre-rejects on the dictionary blank count (that count includes embedded labels and would give a false negative).
    Skolemization still rewrites inside composites (a separate API). RDFC-1.0 behaviour under `rdfc12` is unchanged.
    Must be stated in the migration guide and the profile document.
16. A reserved-namespace IRI is ordinary data in the new family: `check_admissible*` only lowers; the refusal tests
    now use a latched stop signal (refusal) plus the neighbouring valid run, and the legacy refusal is asserted under `rdfc12`.
17. Digest-only entries (`*_digest_*`, graph digest) build no document and need no owned workspace; their stop at any poll reports
    a phase before Output. `canonical_output_accounts.rs` was rewritten from the new engine's own counts (not copied).
18. Frozen identities that moved by design: the pipeline-bundle root fixture (a04da97c... -> bca649dd...); the absent-graph digest
    is the digest of the empty canonical line sequence, not SHA-256 of empty text. Bundle graph pins stay `ContentDigest`
    (public API), filled from the 32 bytes of the graph digest.
19. Step 5 split: the in-crate identity and receipt test for the labelling lands in `purrdf-core` (step 5). The host JSON in
    `validate/src/canonical.rs` (the `Labelling` configuration, the `labelling_unit` event, the optional presentation) lands with
    step 7, because `purrdf-validate` depends on `purrdf-rdf`, `purrdf-entail`, `purrdf-shapes` and the evaluator, none of which
    builds until steps 6 and 7 (decision 10). The refusal text and the legacy constants are untouched: they belong to `rdfc12`.
20. GTS compaction certificates (`gts_certify`: `refold_digest`, `effective_digest`, `CompactionCertificate`) and the
    `stream:contentRefoldDigest` that compaction writes stay on RDFC-1.0 SHA-256, through `rdfc12`. The GTS vectors are frozen
    and compared byte for byte by `dict_vectors` and `streamable_vectors`, and the GTS wire format is governed in gmeow-gts, so
    a changed digest domain there is not this lane's to make. This is the one place a writer records an RDFC-1.0 digest in a new
    artifact (D1 says none should): flag to coordination for a ruling with the gmeow-gts owner.
21. PACK: `FORMAT_VERSION = 2` writes the canonical line-sequence digest (BLAKE3, 32 bytes) in the `rdfc_digest` header field;
    version 1 is still read, and `verify_pack` recomputes it with `rdfc12::try_canonical_sha256_view`. Added
    `PackView::format_version()`. The field name `rdfc_digest` is kept (public API). The v1 golden `pack_small.bin` stays as the
    read fixture; the v2 writer golden is the new `pack_small_v2.bin`.
22. `canonical_flat_nquads` (rdf crate) now returns the new document; `canonical_flat_nquads_with(hash)` is the RDFC-1.0 entry
    by name (delegates to `rdfc12::try_canonicalize_flat_view`). Tests of the RDFC-1.0 flat presentation (differential and
    sentinel corpus in `content_id_byte_determinism.rs`, two properties in `property_roundtrip.rs`) import from `rdfc12`.
    `tests/canonical_blake3_stream.rs` is deleted: its premise (a streamed BLAKE3 of the document bytes equals the GTS content
    label) no longer holds, the digest being of the line sequence. The two flat-view benches keep measuring the RDFC-1.0 path
    through `rdfc12`.

Coordination rulings (09:39 CST), binding:

R1. Decision 20 is accepted: GTS compaction certificates and `stream:contentRefoldDigest` stay on RDFC-1.0 SHA-256. This is the one
    stated exception to D1. The migration section names it and says the next GTS format's own specification decides its digest.
R2. Decision 15 is accepted (a composite literal is a literal). Conditions: (a) the migration section states that a blank label written
    inside a `cdt:List`/`cdt:Map` lexical form is text and does not co-refer with a graph blank node for canonicalization or isomorphism,
    while skolemization still rewrites inside composites; (b) one test, as a pair: datasets differing only in a blank label inside a
    composite literal are not isomorphic and digest differently, and the neighbouring pair differing only in a graph-level blank label is
    isomorphic; (c) check the dropped blank-count pre-check of `datasets_isomorphic` against a case where it gave a false negative and
    keep that case as the test unless the (b) pair covers it.
R3. Decision 8 is accepted: the pin is (nested-refinement-labelling, 1) with presentation `line-sequence` version 1; legacy constants keep values.
R4. No move into the engine branch. After the engine lands on main, sync this branch onto main once. Keep `label_view`,
    `label_view_with_memory`, `Scope`, `Labelling` here (only shared file: `canonical_labelling/mod.rs`, a doc comment on `Labelled.explored`).

23. The lowering of a view admits each quad and each reifier binding once: a `DatasetView` may visit a physical row repeatedly (the
    entail positive-world contract), and a statement is a set member. Annotation rows keep their repeat as the count.
24. `screen_comparison_terms_with_memory` no longer refuses the retired reserved namespace (it only validates and reports blanks);
    `ReservedVocabulary::retained_bytes` is always `Some(0)`; the caller-account payload variant was removed.
25. `CanonError::BudgetExceeded` and `CanonError::Refused` carry boxed payloads: the sixth search event grew `SearchWork`, which pushed
    `CanonError` and every error wrapping it past clippy's large-error bound.
26. Step 7 commits compile and semantic fixes only. Crate-owned golden vectors that move by design (purremb golden, entail oracle goldens and
    proof digests, shapes product bytes, validate regime/DL-proof/entail-repoint vectors, evaluator trace, governor corpus, slice cache keys)
    are regenerated in step 8. Slice tempdir tests cannot run here: the managed build directory has no CACHEDIR.TAG (environment, not this change).
27. Step 8 regenerated only crate-owned goldens (entail oracle goldens, owlrl corpus inputs and goldens via the maintained script, purremb,
    shapes product, validate regime and DL-proof vectors, evaluator traces) and updated in-source proof digests. Each diff was checked to be
    reorder/relabel/digest only (entail goldens, regime vectors: identical line multisets; traces: identical fuel and charges).
    `vectors/sparql-governors` stays frozen: its harness renders certified answers through `rdfc12::try_canonicalize` (the form the corpus pins)
    instead of regenerating the corpus in place (release plan D3). The governor harness's `GOVERNOR_CORPUS_DIGEST` is unchanged.
