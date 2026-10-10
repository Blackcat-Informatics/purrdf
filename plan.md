# Exact ranking over input-derived capacity

Primary issue: #482. Branch `paudley/482-text-rank-inputs-past-query-terms-max`, isolated worktree `/home/paudley/Active/purrdf/.worktrees/482-text-rank-inputs-past-query-terms-max`, originally fetched base `origin/main` at `37e3a26a7`, now synchronized normally to `3ac169b96`. Full issue and zero comments are captured in `issue.md`; `brief.json` and `prior-art.md` record the eight linked issues and bounded recurrence search. The implementation and acceptance fixtures are written; `validation.md` is the authoritative current evidence and publication index.

The complete delivery removes the chosen term, document, field, frequency, weight and score ceilings while preserving every operation and truncation of the published BM25F law. Negative weights, malformed routing, inconsistent corpus/document facts, undefined zero normalization, and intrinsic storage representation failures remain typed errors. No accepted requirement is deferred to the broader arithmetic optimization or retrieval work.

## Governing constraints and relevant prior art

The root `.baseline` requires no deferrals, immediate ownership of touched defects, and no development references in shipping documentation. `.goals` requires maximal performance, portability and utility, Rust-first implementation and RDF 1.2. `AGENTS.md` requires one home per job, no new semantic features, no implicit vocabulary, deterministic bytes, unchanged conformance corpora, normal hooks and complete gates. The existing exact numeric tower (`xsd::exact::Integer` and `xsd::bigint::BigInt`) supplies the checked i128 fast path and unbounded fallback; `xsd::wide::mul_div` already handles a scaled product whose final result fits u128. The ranking law lives in `text/src/ranking.rs` and `text/RANKING.md`, and the index uses that same prepared scorer for full ranking, bounded heaps, point scoring and explanations.

The cited ADR-0030 is a motivating Katamari decision, not a PurRDF ADR found on disk. The brief records that absence rather than inventing a local decision. Its packing premise does not constrain this carrier. The linked numeric-tower issues are complete; the open optimization sweep is not a dependency. Bounded recurrence: 19 `none`, one `semantic-identity-loss`, one `performance` among the last 200 commit trailers; this is not an all-history recurrence claim.

## Design

Use a fixed-scale adapter around the existing canonical `exact::Integer` only for intermediates that leave i128. Share the existing `Fixed` signed wide-product kernel for small multiplication/division; promote per operation when its result no longer fits. Apply truncation at exactly the original boundaries, including normalization, field weighting, saturation and final IDF multiplication. Do not retry a second scorer, implement a new integer engine, or switch to final-only rational rounding.

Counts widen directly from u64 into i128 at the public scale. `length*population*S/total` uses the existing wide product/quotient; `length*population` fits u128. Corpus totals remain exact u128 per field because both population and per-document field length are u64. The honest consistency bound is `population*u64::MAX`; it is a consequence of the input type, not a chosen ceiling. Summing totals across unrestricted fields uses the existing exact integer home. Every frequency addition is checked before publication.

Replace the fixed sixteen-field buffers with the shared `SmallVec` (sixteen inline slots plus physical spill); profile fields themselves remain an unrestricted Vec. Preserve caller field order and mapping identity. Use usize for query ordinals and matched-term counts, avoiding the existing narrowing cast to u32. The in-memory index's explicit u32 document/position address representation is a separate physical storage law and remains honestly documented.

Prepare a private-construction `ScoreBound` from the sum of `floor(idf_i*(k1+1))` for the actual prepared query. Every nonnegative pseudo-frequency has saturation at most `k1+1`, so this is a certified bound including all truncations. Expose its exact maximum, bit width and ranking-profile fingerprint. Native ranked results carry the certificate. Pure whole-document scoring returns an explicit value-and-bound carrier, so the certificate is reported with that result too; per-term contribution remains a scalar under the same prepared query. Validate against this certificate, including exact interval rather than width alone.

The score itself still fits `Fixed`: for u64 corpus counts the shifted IDF argument is at most `2N+2`, whose natural logarithm is below 46; each addend is below `46*2.2*S`. Multiplying this by any physically addressable query length on the supported 32/64-bit targets is below i128. Large field weights and tiny positive normalizations can nevertheless create much larger intermediates, so their promotion is required. A normalization that genuinely rounds to zero under b=1 remains the original typed arithmetic-domain error, not a chosen input ceiling or clamped approximation.

Bump the ranking law identifier/revision to v2/2, remove all selected ceiling values from its canonical description, and bind the rounding, certificate law, corpus law and actual caller choices. Record the identity migration in the changelog. Original score TSV/lexical goldens stay byte-for-byte unchanged; the old profile fingerprint vectors change deliberately with the admission/identity contract.

The actual 2^25-token fixture builds one RDF document from 32 distinct comma-separated source literals, each with 2^20 analyzed terms, in one field. Commas retain the normal analyzer's lexical and surface terms while giving one substring span per literal. Release the existing lexical projection scratch after the native token walk if the owning Han/surface consumers prove it is no longer needed; preserve all auxiliary outputs and source evidence. This is actual index construction, not a fabricated field count. The fixture runs in a resource-capped native process with the full assertions intact.

## Completeness contract

| Requirement | Task | Real acceptance |
|---|---|---|
| 5,000 distinct terms rank | 1 | Build an actual RDF index and analyze the complete 5,000-term needle; native rank/select and the property-function entry agree in row order, matched count and exact score with independent unbounded reference values. |
| Actual field with 2^25 tokens indexes and scores | 1 | Native `TextIndex::from_dataset` builds the real input above; inspect retained field length/frequency and rank it, comparing every raw score to the independent reference. No ignored/synthetic substitute. |
| Declared 2^41 corpus prepares and scores | 1 | `PreparedCorpus`/prepared whole-document and contribution APIs match the independent reference; also exercise u64 maximum population/count conversion. |
| Original bounded scores are byte-identical | 1 | Preserve all sixteen frozen TSV vectors and all existing score lexical/serialized goldens; execute both population modes and unchanged `bm25f.py --check`. |
| No chosen ceiling refusal remains | 1 | Meaningful above-old-neighbor cases for query/field/frequency/weight/score and more than sixteen actual index fields; overflowing intermediates match the independent unbounded oracle. |
| Genuinely invalid inputs still fail | 1 | Negative weights, invalid coefficient/routing, zero fields, duplicate/unsorted query keys, df>N, impossible corpus totals, inconsistent document lengths and distinct-term frequency sums retain typed errors before zero shortcuts. |
| Derived bound is reported and certified | 1 | Pure result and native `Scored` carry the query/corpus certificate; actual score lies within it, exact-bound validation rejects bound+1/negative, and a 5,000-term rare query exceeds the former 56-bit/score ceiling without refusal. |
| Profile identity names the unrestricted law | 1 | Pin v2 fingerprints for dense and carrier laws, mapping-order invariance/field-order differences, exact bound-law description, and changelog migration; no ceiling folded into the identity. |
| Portability/no new engine or dependency | 1 | Affected strict all-target native lint/tests, actual portable ranking harness (including promoted arithmetic), layer/ring-fence checks and final unchanged `make check`/all-release WASM builds. |
| Qualified publication | 2 | Independent complete behavior/evidence review, normal signed hook commit/push, complete Stage artifacts, issue plan/results and PR with `Closes #482`; hosted CI/integration remain distinct. |

## Task 1: Complete unrestricted exact scoring and all consumers

Implement the complete arithmetic adapter, preparation/certificate, dynamic field and query representations, index/scoring/relation migrations, meaningful acceptance fixtures, unchanged independent Python-reference comparisons, and shipping docs/changelog as one substantive behavior unit. Reuse testkit's independent Natural arithmetic as one shared test oracle home; do not use production arithmetic to generate expectations. Include actual small-case controls distinguishing per-operation truncation from a final-only rational result. Do not add another numerical engine, public namespace, external runtime dependency or arbitrary limit.

After all source and useful fixtures are written, run one consolidated affected native strict-lint/test family, collect failures before coherent correction, and run the actual portable harness. Independent review is proportional and covers the whole contract and real callers. Regenerate metadata once the law settles. Run the unchanged mandatory full gate on settled source: the full-check ceiling is three invocations, aim one; every failure counts and `validation.md` records the actual ledger. Mandatory hooks remain normal. No baseline whole gate or per-edit/per-task test ritual.

## Task 2: Publish the complete qualified delivery

Record all actual terminal evidence in `validation.md` and task records, obtain independent complete-contract judgment, then normal signed commit/push and the Stage1 issue/PR publications. Use stagectl for supported mechanics. Parent owns sequential ghprsq integration; no issue closure credit until actual merge. Preserve all sibling worktrees, donors and PR523 evidence. If any required behavior remains missing or a gate fails, keep the delivery open with the exact blocker; no partial PR or success with caveats.

## Considered alternatives

Whole-score BigInt computation would allocate on healthy small cases and discard the existing fast path; rejected. A second outside-the-old-bounds scorer would duplicate the law; rejected. A declared-length-only large-field fixture would miss index refusal and storage behavior; rejected. A packed host comparator or new RDF result column would change an unrelated consumer contract; the native certificate provides the required host decision without inventing those surfaces. Complete generic arithmetic optimization, per-row retrieval evidence, and larger in-memory document address representation have no correctness dependency on this delivery and remain in their own accepted contracts.
