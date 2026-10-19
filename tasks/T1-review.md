# T1 review: re-vendor the SHACL 1.2 suite and update harness bookkeeping

## Reviewed scope

Uncommitted worktree diff (`git diff`, `git status`) against plan Task 1 and G5, G6, G8, G9, G9a, G16, G17/U4 and the Binding documents. The review covered the vendor script, frozen manifests, the discovery module, the ledgered tests, `lint.rs`, the census, `vocab.rs`, the docs and the README edits.

## Evidence

| Check | Command | Result |
|---|---|---|
| Vendored tree equals upstream | `git archive 4cd4698... shacl12-test-suite/tests`, then `diff -r` against `vectors/shacl12/tests` | IDENTICAL (683 files each, untracked adds included) |
| Frozen corpora | `python3 scripts/check-corpus-frozen.py` | OK: 29 corpora byte-frozen |
| Hand edits in `vectors/` | `git diff --stat -- vectors` and spot checks | Only `vectors/shacl/README.md` (U4: 6 vendored plus 3 first-party = 9, total 129) is hand-edited. `PROVENANCE.md` differs only in the pinned commit and tarball URL, and the pin matches `COMMIT` in `scripts/vendor-shacl12.py`. Vocabularies and tests are verbatim upstream. |
| Grading test | `cargo test -p purrdf-shapes --test w3c12_conformance -- --nocapture` | 17 pass, 2 fail (`w3c_shacl12_conformance`, `every_planned_inventory_id_passes`). The scoreboard shows `W3C12 TOTAL: passed 571, non-canonical-expected-decimal 6, xfailed 0, ledger 0`. The failures are exactly the 14 named cases (12 in `w3c_shacl12_conformance`, which also lists the 2 stratification cases). The 6 decimal cases are reported. |
| Ledgered tests | `product_corpus_equivalence`, `vocabulary_import_invariance`, `shacl_shacl_differential`, `spec_census`, `host_entry_points` | all pass. Output: `PRODUCT-EQUIVALENCE: passed 395 ledgered 0 unparsable 20 total 415`, and `VOCABULARY-INVARIANCE: 394 compared on a report of 415`. |
| 1.0 suite | `w3c_conformance` | pass (129/129 pinned) |
| Matrix self-test | `conformance-matrix.py --self-test` | OK |
| Doc claims | `check-doc-claims.py` | FAILS (see finding F1) |
| Hygiene | `cargo fmt --all --check`, `cargo clippy -p purrdf-shapes --tests`, `helper-census --no-features`, `--non-rust-ratchet` | all clean |
| Constants | `592 - 209 SRL - 1 unlisted = 382` | K=382 matches `the_shacl_root_scope_is_pinned` |
| Folder defaults | `reports.js:29-33` `DEFAULT_CRITERIA` and `:200-203` | `default_conformance_class` is identical: core, inference-rules and node-expr, with `None` otherwise (the page's 'Unspecified' becomes a panic for a SHACL-root entry) |

Real consumers: `Case12` is used by `w3c12_conformance.rs`, `product_corpus_equivalence.rs`, `vocabulary_import_invariance.rs`, `shacl_shacl_differential.rs`, and `crates/cli/tests/w3c_node_expr_cli.rs` and `w3c_srl_check_cli.rs`. All are migrated from `listed` to `root.is_listed()`, and the CLI counts are updated.

## Answers to the requested scrutiny points

### 1. New ledgers

All entries trace to the 14 known failures, except one permanent exception and one permanent stricter rule, both justified below.

**`product_corpus_equivalence.rs`**, `W3C12_REFUSED_AT_LOAD` (7 entries):
- The 4 `rules-entailment-ruleset-*`, `deactivated-select-001`, `prefixes-003` and `pre-binding-009` are all members of the 14.
- XPASS is enforced in both directions. An entry whose graph starts loading lands in the Agreed bucket and raises `XLOAD ... remove the entry`. A non-ledgered case that stops loading raises OVER-REFUSAL.
- `UNLOADABLE_CASES`, `AGREED_CASES` and the 394/371 pins are exact.
- `W3C12_DECLARED_FAILURES` goes 5 to 6: the new upstream `sparql-syntax-error-001`.

**`vocabulary_import_invariance.rs`**:
- `REFUSED_AT_LOAD` has the same 7 plus `singleInstanceOf-001` (also in the 14). The check is set equality between the ledger and the actual refused-at-load set, so a case that starts passing fails the test. No silent skip.
- `INVARIANCE_EXCEPTIONS` has one entry, `sparql-syntax-error-001`. XPASS is enforced: `(Ok(_), Some)` is a failure with "remove it", and the count is exact.
- The justification is verified. I ran the CLI on the case alone and with the three vocabulary files concatenated:
  - alone: `SPARQL syntax error at byte 536: expected an RDF term, found None`;
  - merged: `... at byte 694 ...`.
- The load refusal is the same in both runs and only the byte offset differs. This is permanent (the case is a declared `sht:Failure`), not a deferral. It is non-blocking, but see F4.

**`shacl_shacl_differential.rs`** (3 new `Stricter` entries, each pinned by exact count at line 648, so a fix forces a count or removal update):
- `unsupported-entailment-regime` (4): the four ruleset cases, which are in the 14.
- `sparql-query-text` (2): `sparql-syntax-error-001` (permanent) plus `prefixes-003` (in the 14).
- `sparql-constraint-deactivated` (1): a non-boolean `sh:deactivated` mutant on `sparql-deactivated-001`'s constraint node. This is a permanent engine rule, not one of the 14. It is cited to Core ("the only valid values for sh:deactivated are the constant literal node expressions true and false", `shacl12-core/index.html:1704`), which is correct. That paragraph is non-normative (see F3).
- Counts moved with the corpus (255, 203, 112, 172, 41, 415, 783), and the test passes.

### 2. Vocabulary-driven behaviour

- The upstream `shacl-shacl.ttl` diff changes `sh:closed` from `sh:datatype xsd:boolean` to `sh:or ([sh:datatype xsd:boolean] [sh:hasValue sh:ByTypes])`. The `closed-by-types` supersession is therefore obsolete, and its removal is correct.
- `host_entry_points.rs` now asserts that `sh:closed sh:ByTypes` gives no shacl-shacl result. It uses a list-valued `sh:nodeKind` for the superseded and refused-neighbour cases, which keeps the "superseded only when the loader accepted" premise tested.
- `shapes_tools_cli.rs`, `crates/cli/README.md` and `docs/book/.../shacl.md` follow.
- The `list-valued-node-kind` differential count of 2 is exact.
- Census:
  - `sh:incomplete` and `sh:incompleteURIStart` are classified as Graph.
  - `ValidationActivity` and `ValidationAgent` are Report; `sh:usedConfiguration` is removed.
  - `sh:codeType` is non-validating.
  - All of this matches the `shacl.ttl` diff, and `spec_census` passes with counts 11, 22 and 19.

### 3. `sh:detail` test

- Upstream `memberShape-001` no longer states details; the test asserts `expected_details.is_empty()`.
- The test builds the previously approved details (`"Bob"` on `ex:list2`, `"Charlie"` and `"Donna"` on `ex:list5`) and asserts the engine produces them. It still proves that the grader fails when details are lost, added or changed, and that an unstated detail is ungraded. It is still meaningful. The new first assertion pins "approved report states none", so a future upstream change is noticed.

### 4. Case12, constants and classes

- `Root12`, `Status12` (a closed enum; any other value panics; `None` only for SPARQL-RL), `requires` (sorted and unique), `W3C12_SUITE_COMMIT` (verified against `PROVENANCE.md`), `W3C12_CLAIMED_CLASSES` (6 constants through `vocab::sht`) and K=382 are all tested.
- `the_claimed_classes_are_the_union_over_the_shacl_root` passes, so the claimed set is confirmed from discovery.
- The `xone-002` and `xone-003` bookkeeping change is correct: upstream now lists them. `UNLISTED_FILE_DELTAS` is empty, `a_listed_entry_is_never_amended` uses a control row, and `W3C12_UNLISTED_ENTRIES` is 1.

### 5. `vectors/`

Clean, as above.

### 6. `docs/CONFORMANCE.md` splice

- The hand-spliced numbers are internally consistent: 571 + 6 + 14 = 591 listed; 592 discovered; 415 = 395 + 20 (13 `sht:Failure` plus 7 refused at load); 213 `sht:Validate`; 209 SPARQL-RL.
- They match the harness output. The row is marked RED, which is accurate while the 14 fail. This is acceptable as an interim if the final full run regenerates the block with `--write-doc`.
- The prose is accurate and carries no process text.

### 7. Issue numbers, process text and attribution

None in the changed non-vector files. The grep for `#570`, `#384`, "issue", "stage", "task", "claude", "anthropic" and "Co-Authored" hit only "Working Group" in code docs.

## Findings

- **F1 (MEDIUM, expected-red, not a defect in the diff).**
  - `scripts/check-doc-claims.py` fails: "matrix row ... totals 577 ... but pins 592 discovered tests".
  - The 14 failing entries are absent from the matrix row's Pass plus XFail, as designed.
  - Plan Task 1 and G9a say to update `check-doc-claims.py`'s pin near lines 3219–3284 in the same commit, but the script is unmodified.
  - `make check` cannot be green at the end of Task 1, because the grading tests also fail. This is already acknowledged by the plan's Task 1 check ("fails on exactly the 14").
  - Required follow-up:
    - Make `check-doc-claims` accept the count only through the final state, or teach it the failing-entry count.
    - Regenerate the block at the final full run.
    - Confirm `check-doc-claims`, `w3c12_conformance` and `make conformance` are green after Tasks 2–7.
    - Do not push Task 1 alone.
- **F2 (LOW).** Prose nits introduced in the docs: "1 files" (`docs/CONFORMANCE.md` table row, `README.md`) and "The other 1 are" (`docs/book/src/validation/shacl.md`). Fix the pluralization in the generator or the template at the final regeneration.
- **F3 (LOW).** The `sparql-constraint-deactivated` citation quotes a paragraph the spec marks non-normative ("The remainder of this section is non-normative", `shacl12-core/index.html:1702-1706`). Say "informative" in the rationale or cite a normative sentence.
- **F4 (LOW).** The `sparql-syntax-error-001` exception compares error text including the byte offset. A permanent exception is justified, but normalising the offset in the comparison would remove the need for it. Optional.
- **F5 (INFO).** `vectors-shacl.sha256` does not cover `vectors/shacl/README.md` (it has no README entry), so there is no hash to update. The plan's "update vectors-shacl.sha256" is a no-op, and `check-corpus-frozen` is green.
- **F6 (INFO).** The product harness prints the 7 refused-at-load cases under "unparsable 20". It is accurate (13 declared plus 7).

No ledger is a silent skip. Every new ledger entry either self-clears (an exact-count or set-equality test fails when the case starts passing) or is a justified permanent engine property.

VERDICT: PASS
