# T20 independent review (Addendum A Task 20, PurRDF #570 G-11)

Scope: uncommitted diff in /home/paudley/Active/purrdf/.worktrees/570-t20 (base 155cf3c3c): conformance-kit `outcome.rs`/`lib.rs`/`tests/grading.rs`/new golden `earl-two-subjects.nt`; shapes `tests/support/w3c_earl.rs`; sparql-conformance `community.rs`, `bin/community-conformance.rs`, `tests/community_conformance.rs`. No source modified.

## Evidence run (side worktree)
- `cargo test --locked -p purrdf-conformance-kit -p purrdf-sparql-conformance`: all suites ok, 0 failed (incl. new grading tests and `the_community_earl_asserts_each_subject_and_test_once`).
- `cargo test --locked -p purrdf-shapes --test w3c12_conformance --test w3c_conformance`: 29 and 12 passed.
- `cargo clippy --locked -p purrdf-conformance-kit -p purrdf-sparql-conformance -p purrdf-shapes --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- W3C EARL generated with PURRDF_W3C_EARL_DIR/COMMIT=155cf3c3...: purrdf-shacl-earl.ttl sha256 f48ce1f0fe14be85bfc4505d89ccb13b7200b06aaff0122685d3110f613c199e (120 earl:subject), purrdf-shacl12-earl.ttl sha256 dfd87e26fa9baecd57feaf9c2fcd6c6dbb397ce64a7dee912e66be6df5acd512 (382). Recorded for later comparison. I did NOT build a pristine base to diff hashes (would need a full second build); the byte-identity argument below is by code inspection plus the unchanged golden.
- Existing golden `earl-report.nt` is unmodified in `git status` and its test passes; only `earl-two-subjects.nt` is new.

## Findings by item
1. DuplicateAssertion: `with_subjects` sorts each subject's records by (case, profile, surface) and refuses adjacent equal `case` regardless of profile/surface/verdict. Typed `GradeError::DuplicateAssertion{subject,case}` with Display arm. Test asserts exact variant for same case under different profile/surface. Also refuses empty subject list and duplicate subject IRIs (typed Malformed). OK.
2. Determinism: subjects sorted by IRI, records sorted within subject, flattened with `owners`; assertion/result blank labels are `assertion{index}` on the global sorted index, so stable and independent of input order. Test builds ["b","a"] vs ["a","b"], asserts equal Reports and equal lines, plus golden. OK.
3. Single-subject output: `Report::new` delegates to `with_subjects` with one subject; ordering key, emit order (assertor, subject, then records) and blank labels are unchanged from the old code, so triples and numbering are identical; the W3C harness tests and the old golden pass. Behavioural change (intentional): a single subject can no longer hold the same case twice under different profile/surface; the one unit test that did so was adjusted, and the W3C adapter `.expect()`s (suites list each test once; verified by passing runs). OK.
4. Route IRI: `{base}#profile={enc}&surface={enc}` using `purrdf_lex::percent::encode(.., FRAGMENT)`. Valid fragment per RFC 3986; deterministic; `to_earl` interns via `intern_iri` (validation by the builder). Minor: FRAGMENT keeps `&` and `=` (sub-delims), so a pathological profile/surface containing `&surface=` could in theory yield the same IRI for two distinct routes. This fails closed (with_subjects refuses duplicate subject IRI with Malformed) and real profiles/surfaces are fixed identifiers (`native`, `native/parse`, ...), so not a blocker. A hardening note: encode `&`/`=` (or use a stricter set) if routes ever become free-form.
5. Community test: asserts routes>1, subjects().len()==routes, and reads back (earl:subject, earl:test) pairs from serialized N-Triples, requiring all counts ==1 and pair count == records. If routes collapsed into one subject the library would return DuplicateAssertion (or the pair/subject counts would fail). Sound.
6. DuplicateAssertion arms in `observe`/`grade_inferences` mapped to `OutcomeKind::Malformed`: these functions only produce comparison errors from `report::compare_graphs`/`compare_report`, which never construct DuplicateAssertion; the arm is exhaustive-match filler. Mapping to Malformed with the error text is the honest fail-visible choice (a recorded failing observation rather than a panic, consistent with the file's "no unreachable!" typed style). Acceptable; `unreachable!` would be a panic path in a library, worse. OK.
7. EARL 1.0: each route is an `earl:TestSubject` (typed) with `rdfs:label` (kit-neutral; label supplied by the binary, kit has only EARL/RDF terms). Multiple subjects with one assertor and one assertion per (subject,test) is valid EARL. OK.
8. API change `Report::subject()` -> `subjects()`: grep over crates/ shows no remaining callers of the old accessor; `Report::new` callers (grading.rs, shapes w3c_earl.rs, community binary now via `earl_report`) all updated; workspace crates compile under clippy `--all-targets`. OK.

## Non-blocking notes
- `to_earl` indexes `subjects[report.owners[index]]`; safe because fields are private and built together in `with_subjects`.
- Doc line in `to_earl` exceeds the usual wrap width (cosmetic; fmt passes).

VERDICT: PASS
