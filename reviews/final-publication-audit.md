# Geo final publication audit: revised delivery scope

## Final combined-head integration review

Bounded independent source verdict: PASS for the inspected conflict resolution
at `3c92cbeebad8d52bac064f1d4f9a714e7d9b7fb6`, whose parents are geo checkpoint
`b5ef3879d340cd68fd91daf75e46912f22b60a66` and actual R2 main
`29661e74bdfa5e760fca5595e72b572176d7dd0e`. The worktree is clean at readback.
Root confirms publication as PR #534. This is a source/integration review,
not a completed hosted review or original full technical PASS.

- Hash framing retains R2's fallible `try_frame_le_with` and exact checked
  labelled-frame layout, together with geo's infallible sink and direct display
  framing. Prefix bytes remain shared and unchanged; fallible append still
  stops at the first refusal. A failed partial display digest cannot be published.
- Governor v16 preserves the R2 v15 EXISTS explanation and historical digest
  `34f3ed243e70e82eb473de8a264af5a712ccab053c48bc4be286f1c7293071a1`.
  The original 17 labels/prices retain their positions; native-function-work is
  appended at index17. Source, version assertion and published recipe agree on
  v16 digest `a62e6e30a5ad78c6c25253fd6d0757c7bee48900a6a9001e06178d87d5c66e53`.
- Property opening retains R2's established-row ceiling and dictionary dispatch
  inside geo's metered call. Physical failure and stopped-work precedence remain
  before cursor consumption, followed by the original whole-search evidence
  retention. Candidate-scope support checks and occurrence/dictionary APIs remain.
- `WorkspaceCursor` keeps its concrete Box before its original control grant;
  `box_native` charges before allocation. Bounded raw extraction refuses.
  KNN/HNSW retain scoped candidate generation before ranked depth, original-rank
  declarations, invocation/payload owners and reported work. Their common cursor
  constructor now transfers concrete control to this outer owner without a
  second charge or an internal grant released before Box deallocation.
- Text phrase/dictionary cursors use that same admission/destruction path.
  PhraseBatch/Results and term payload owners remain intact. On-demand CallCursor
  retains WorkspaceCursor directly, preserves R2 search/binding owners, and
  passes the actual engine option's GeoProfile with its explicitly ungoverned
  context; it does not extract a bounded cursor into an unowned Box.

The current `validation.md` integration section records combined-tree command
sessions: evaluator/text/HNSW library check PASS; profile digest1, evaluator
evidence_ingress20, governor_correctness9, ranked_declaration35, text
phrase_composition5, HNSW ranked_declaration17, governor corpus15 with one
intentional regeneration ignore, and metadata154 claims/37 license profiles
PASS. Root confirms these were command-session outputs, not standalone raw logs.
This review cites that execution record, not the unrelated old
`recovery-integrated-kernel.log` or any predecessor receipt.

No new material semantic/ownership contradiction is established in the inspected
resolution. Earlier numerical corpus receipts retain the reviewed unchanged
kernel applicability. Pre-merge package, portable and ASM passes remain bound to
their earlier artifacts; these focused native integration results do not by
themselves establish a new full combined-head package/portable/ASM pass. Keep
that final-head applicability limit with #533 and actual hosted outcomes; no
extra local repair/test cycle is requested under the user's revised scope.
Revised shipping disposition remains PASS; original technical qualification
remains FAIL/incomplete. No source/index edits, builds, tests or forge actions
were performed by this review; only these review artifacts were updated.

VERDICT: PASS for the user-authorized revised shipping disposition; original technical qualification FAIL/incomplete.

The final round is terminal. The original complete technical acceptance is not
qualified: the full gate and conformance command failed, eight affected Python
calls were unreached, and private/architecture/performance acceptance remains
incomplete. The user explicitly authorizes publication and merge after this
round regardless of findings. Root confirms visible follow-up issue
[533](https://github.com/Blackcat-Informatics/purrdf/issues/533) is filed.
This revised-scope PASS does not represent a passing full gate or hosted checks.

## Terminal final-round disposition (supersedes pending states below)

Read current `recovery-final-round-remaining.log`: projections terminal 0,
host refresh terminal 1, assembly terminal 0, full-gates launcher terminal 1,
master terminal 1. Its independent steps continued after failures as specified;
no failed composite invocation is promoted to PASS.

- Adapter atlas23/topology73 and governor projection replay remain PASS.
  `recovery-shared-mean-portable.log` records all twelve native/scalar-WASM/
  SIMD-WASM replays PASS, terminal 0; metadata terminal 0.
- Fresh installed C bundle/header/library/real smoke PASS. Python release
  wheels build and pass metadata, shadow imports and graph-operation self-check,
  terminal 0. The separate selected-test launcher fails importing the private
  `purrdf.purrdf_native` path from the installed public module, terminal 1,
  before its eight cases. This is an actual qualification-launcher failure,
  not proof of a wheel defect or of those eight host calls passing.
- Optimized WASM package build, four geographic cases, three poison cases,
  typecheck and packed-tarball smoke PASS. Fresh assembly generator/checker
  PASS for all 112 sites on seven configurations with unchanged thresholds.
- `recovery-shared-mean-make-check.log` terminates 2: formatting PASS, strict
  Clippy FAIL at `topology/relate.rs:569`, `scan_for_areas` has eight arguments.
  Subsequent build/tests/hygiene/nested all-release WASM are unreached. This
  touched production signature is a concrete unresolved source-gate defect.
- `recovery-shared-mean-conformance.log` terminates 2: reported suites are
  green or ledgered, but the committed Python count 4218 differs from measured
  4223 in `docs/CONFORMANCE.md`. The stale projection must be regenerated;
  the command is FAIL despite its inner green suite summary.
- Private completion remains safe aggregate 4/31 active, not a full pass.
  Actual Arm64/i686/i586 runtime and affected polygon/atlas performance remain
  unqualified. Both full public 500000-row geodesic passes retain applicability
  by the reviewed callgraph; no rerun is required by the changed witness mean.

The reviewed follow-up draft covers all these failures, unreached suffixes,
private population, architecture and measurement limits, without restoring an
exclusive timing campaign or exposing private information. Root corrected its
two stale pending statements before filing #533. No material follow-up omission
is established by this bounded audit. Carry subsequent hosted/integration
findings visibly to that issue. No new source repair/test cycle or skill-only
green-CI hold is imposed. Normal hooks, actual enforced merge guards and ghprsq
remain binding; this review does not bypass any of them.

The following detailed criterion table and inventory preserve the earlier
pre-terminal assessment; the terminal disposition above governs current status.

## Authority, scope and reviewed evidence

The user's explicit decision is:

> geo get's pushed and merged after this round regardless of findings, all findings go into a new issue.

This supersedes withholding publication until every original criterion passes.
It does not make unrun, failed or incomplete acceptance successful and does not
permit bypassing hooks or required merge checks. Finish the already planned
round, record actual results and visible follow-up, then proceed through the
authorized normal publication/integration workflow. No new broad review,
discretionary repair cycle or repeated unchanged test is requested here.

Applied the complete Stage completion-adversary rubric and evidence-reuse rules
to the supplied worktree and selected Stage. Governing inputs are applicable
AGENTS, repository `.baseline`/`.goals`, `plan.md`, both complete resumed issue
captures and accepted addenda, and current `validation.md`. Worktree:
`/home/paudley/Active/purrdf/.worktrees/409-geo-deterministic-hierarchical-spatial`;
branch: `paudley/409-geo-deterministic-hierarchical-spatial`; comparison:
`origin/main`. No fresh remote/integration identity is invented by this review.
The owner additionally refreshed complete live issue surfaces into
`publication-issue-408.md` and `publication-issue-409.md`: both remain open,
the two existing geographic-plan comments are retained, and no new contract
change was found. The focused adapter and projection portions have finished.
The remaining final round is running in session 32406 with source frozen;
no new implementation cycle is requested.

Reused substantive source judgments in `recovery-resumed-material-review.md`,
`../root-source-review-notes.md`, `../recovery-formatter-review.md`, helper triage,
native-profile fixture review, benchmark duplication and WASM assembly findings,
and the planar query diagnosis/corpus applicability reviews. The final shared
mean adapter was independently reread: segment owns the arithmetic and admission
adapter; planar arrangement and atlas relation import it; relation workers and
atlas cut witnesses call it. No new ownership, operand, mathematical or typed
error defect was found. This does not claim a new exhaustive mathematical audit.

The review ran no build/test/timing, touched no process or shipping source, and
read no private input or private raw result. Only safe aggregate private status
supplied by the owner is used: 4 of 31 bundles completed, process still active.
No identifying private content, path, name or coordinate belongs in the follow-up.
The emergency ledger is clean. Its repository policy forbids using it for an
authorized scope cut or unrun environmental acceptance; no entry was created.

## Criterion and applicability judgment

All evidence paths below are relative to the selected Stage. An existing
component PASS is not relabeled as a successful whole failed invocation.

| Original required criterion | Actual evidence / applicable observation | Current status |
|---|---|---|
| WGS84 inverse/direct, deterministic rounding, edge cases and exact range refusal | Scratch-corrected kernel/XSD suites, cut-locus controls and both complete public GeodTest modes; original numerical law and range/certificate controls retained | Demonstrated on recorded native artifacts; cross-architecture acceptance pending |
| Full 500000-row plain distance corpus | `recovery-resumed-geodtest-distance.log`: 500000 complete, zero mismatches/refusals, fixed fallback 32.2752%; successful checked transition to all-mode establishes exit success | PASS, retained applicable |
| Full public inverse/witness/direct corpus | `recovery-resumed-geodtest-all.log` plus status receipt: 500000 complete, zero distance/witness/direct mismatches/refusals, conditioned metadata excesses zero, terminal 0 | PASS, retained applicable |
| Complete geometry metrics, source-linear/arc topology, holes/collections/complements and materialization | Complete kernel predecessor 714 PASS; final adapter atlas23 and topology73 PASS, terminal 0; existing actual metric/buffer measurements and evaluator/validate consumers | Native components demonstrated; remaining portable/package/final qualification pending |
| Original planar law and query rewriting | `recovery-shared-mean-query.log`: all nine original public query cases PASS, terminal 0, unchanged answers and 262144 cumulative work; direct 49-pair, topology73, ownership1 and determinism4 receipts also terminal 0 | Planar failure resolved; no budget increase or oracle weakening |
| WGS84/CGCS2000 profiles, reference/axis/epoch distinctions | Existing kernel and real engine default CRS84/explicit EPSG consumer controls, exact profile identities; source reviewed without implicit datum conversion | Focused demonstrated; full host/architecture completion pending |
| Projection, GCJ/BD09, caller-parameter/grid operations and continuous images | Current scratch-corrected public provider fixture passes all 664 rows per model with separate numerical/model limits; native operation cases and 63 recorded measurements | Recorded components demonstrated; not global provider or surveying certification |
| Native cell assignment, hierarchy, levels, identity, seams and ranges | Kernel controls, frozen cell transcript, physical-scale and bounded/unbounded parity; actual public APIs reviewed | Native demonstrated; final portable/architecture refresh pending |
| Finite fixed/mixed/prepared-region covers and no false negatives | Kernel/oracle controls and complete six-case 1/30/400 km smoke; existing emitted-count/time measurements with their actual conditions | Demonstrated components; final affected source acceptance pending |
| Index cover/range/refinement and rounded versus physical predicates | Real index controls and measurements report identical 83 accepted keys at two levels with 287/101 candidates | Recorded native component evidence; no new scaling claim |
| Original scratch/grant admission, refusal, live ownership and cancellation | Corrected cover working width, shared numeric owner/retry reviews, 414 XSD cases, added singleton success/zero-capacity/late-refusal accounting | Focused demonstrated; final gate remains pending |
| Scalar/prepared/batches/backends and inner allocations | Existing geodesic 32, transform63 and cells/index7 measurements; batch sizes 1/4/16/256/4096; 25 normal and three inverse windows allocate zero. Published wide proofs have separately reported allocations and zero retained bytes after destruction | Component measurements retained; not a successful combined performance invocation |
| Current polygon/atlas performance applicability | Shared witness means change the computation behind affected polygon/atlas paths; earlier timings keep their own artifact identities. Existing 10 m buffer measurement explicitly used 20000000 work, not the default budget | Current affected performance not established; disclose, do not imply speedup/default-budget buffer acceptance |
| Complete private original-population qualification | Safe owner-supplied aggregate: 4/31 complete and active; earlier incomplete/failed receipts remain historical | INCOMPLETE, not qualified; preserve process |
| Standard scalar/property functions and all query lanes | Existing 95 evaluator/47 validate controls, real default/prepared/governed/entailment/update forwarding, original typed error channel, plus latest nine planar queries | Native production-path evidence; final full conformance pending |
| CLI | Five restored actual CLI cases PASS with exit/stdout/stderr refusal law; no CLI algorithm added by last adapter | Retained focused evidence; required full gate pending |
| Installed C and header parity | Earlier `recovery-installed-c-bundle.log` builds/installs/links real smoke, terminal 0; shared dependency/control/helper changes require current library rebuild and smoke | Prior artifact PASS; affected refresh pending |
| Installed Python | Earlier sdist-to-wheel audit and five installed geographic cases PASS; shared constructor/lexical edge/response/control changes require fresh wheel and affected actual callers | Prior artifact PASS; affected refresh pending |
| Optimized packaged WASM | Original 446 PASS/3 FAIL, then all three poison failures plus four geographic cases/type/packed smoke repaired; separate identity fixture PASS. Rust helper/link dependencies changed afterward | Composed predecessor coverage retained only for unchanged behavior; fresh affected artifact acceptance pending |
| Native/portable-WASM/SIMD-WASM exact replay | `recovery-resumed-portable-registered.log`: all twelve executions PASS after registration and pooled-owner expectation repair; current witness computation differs | Historical PASS; final affected replay pending |
| Arm64/i686/i586 runtime | Reviewed actual CI target registrations and numeric backend names, actionlint PASS | Runtime UNVERIFIED; compilation/registration is not execution |
| Seven-configuration emitted instruction evidence | Repaired generator/checker both terminal 0 for 112 sites; selector corrections measure actual retained WASM callers. Helper/source changes follow that report | Historical PASS; final source-bound ASM refresh pending |
| Mathematical provenance, licensing, H3 design | Existing complete source/provenance/design review, 37 generated license profiles; native grid implementation remains distinct from accepted H3 architecture-only deliverable | Source/design covered; final generated checks pending, no H3 runtime claim |
| Manual/generated claims, headers, book/translation, charge projections | Earlier metadata/book pass; measured regeneration adds only zero native-function-work rows to 21 charge fixtures, matching manifest hashes and repinned digest; original failed gate retained | Projection diff reviewed; digest/full-corpus replay PASS, final metadata pending |
| Mandatory full gate, all-release WASM and conformance | `recovery-resumed-make-check-prose-fixed.log` terminal 2, three failed targets; nested WASM/conformance suffix unreached | NOT PASSED; final round pending |
| Normal hooks, publication, hosted review/checks, integration, closure/mirror/cleanup | No final PR/check/merge result supplied | NOT MET, later workflow obligations |

## Why the public corpus evidence survives the final witness repair

`recovery-planar-corpus-applicability.md` traces the public driver through
PreparedGeodesic distance/inverse/direct, without planar relation or the changed
atlas means. The sanitized private driver's line-only geometry has an empty
areal region; boundary processing exits before polygon witness means. Its point
index refines through PreparedGeodesic, and it does not request prepared-geometry
relation. Its line materialization does not take polygon/folded-image paths.

The last repair is explicitly limited to topology/atlas witness means and their
same-law admission adapters. Operation subdivision, interval/MathContext and
retained-owner arithmetic were not migrated. Consequently both completed public
corpora and the ongoing private computation retain their original artifact
applicability. This is not a new-source execution claim and does not turn private
4/31 into completion. Polygon/atlas, portable, package and emitted-source
evidence must respect their own affected-source boundaries.

## Exact final-round disposition and follow-up inventory

The pending round is already specified by the owner: final adapter topology/atlas
checks, charge projections, portable replay/metadata, installed package refresh,
ASM, mandatory full gate and conformance. Record each actual terminal result;
do not start a new discretionary cycle if it finds more work. The visible
follow-up must carry every unresolved item at publication, with these initial
entries reduced only by applicable evidence from that round:

1. **Private full-population acceptance:** 4/31 completed at this audit, no full
   pass. Retain and finish the original active process; record safe aggregate
   completion/refusal outcomes. No private identifiers or raw inputs/results
   may be copied into public issue or Stage publication material.
2. **Final adapter and charge projections:** final adapter atlas23/topology73
   acceptance now passes. Measured regeneration executes one case successfully;
   independent diff readback confirms exactly 21 `.charges` files each append
   `native-function-work\t0`, with no old row removed and no answer, spend,
   metered result or fuel-sweep change. The existing freeze generator changes
   exactly those 21 manifest hashes; `GOVERNOR_CORPUS_DIGEST` is repinned to
   `e41edb5340fc26e81e6de6d22ee644ee4e3e2bf580a2feced9585ad108449421`.
   This is projection reconciliation, not weakened behavior expectations.
   `recovery-governor-projection-check.log` now terminates 0: all 29 frozen
   corpora verified, reproducible digest one PASS, governor corpus 15 PASS and
   one intentionally ignored regeneration case. The two original failed
   projection tests are now repaired and replayed; this item is resolved.
3. **Current-source portable and architecture acceptance:** twelve final replay
   executions and actual Arm64/i686/i586 target runtime remain to be established.
   Hosted results arrive after publication and must be tied to the actual
   candidate. Unavailable/failed architecture evidence remains explicit.
4. **Fresh installed artifacts:** existing `recovery-helper-host-refresh.sh`
   covers C bundle/header/smoke, Python sdist-to-wheel and eight affected cases,
   optimized WASM four geographic plus three poison cases/type/packed smoke.
   Select fresh artifacts and retain exact failure propagation. Unaffected
   long async coverage may be reused under the host applicability review;
   do not relabel old binaries or failed whole invocations as newly passed.
5. **Final seven-configuration ASM and projections:** original 112-site pass
   precedes helper and topology source changes. Preserve it historically and
   record actual final generator/checker disposition with unchanged thresholds.
6. **Full local/conformance/release-build result:** prior full gate is failed.
   Native-profile36 is repaired and executed in R2 on the exact shared patch;
   planar query9 is repaired and passed here; charge projections now pass replay.
   Neither component repair establishes full `make check`, nested `make wasm`
   or separate `make conformance`. List every remaining final-round failure or
   unreached suffix in the follow-up.
7. **Measurement limits:** carry unqualified current affected polygon/atlas
   timing and any other unsupported final-source performance claim. Keep the
   earlier covers invocation failure distinct from later successful component
   receipts; retain explicit buffer budget and published-proof allocations.
   No required exclusive timing campaign is added by this audit.

Publish the new follow-up link and actual round disposition in validation, PR
and squash notes before merge. Continue required hook/check/review safeguards;
no local artifact can waive them. Assess the actual main delta, including the
other recovery delivery's governor profile version, before ghprsq. Verify both
issue closures, ordinary mirror and durable evidence capture before removing
the branch/worktree. These are pending actions, not new technical findings or
claims of original full-contract completion.

Final focused receipts read for this update:
`recovery-witness-final-atlas.log` (23 PASS),
`recovery-witness-final-topology.log` (73 PASS), and
`recovery-governor-native-charge-regeneration.log` (one executed regeneration
PASS), each terminal 0. The master package/portable/ASM/full-gate round has not
started at this checkpoint. No tests/builds or generators were run by the reviewer.
