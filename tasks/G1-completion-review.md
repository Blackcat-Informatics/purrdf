# Independent G1 completion delta and review debt — 2026-10-08

VERDICT: PASS

Implementation and local acceptance remain complete at cdc43aaa62507258b5551a9cdb9ede34f0e8a107. This is a bounded Stage3 completion re-review, independent of the implementation, qualification worker and root's review. Hosted CI, publication of feedback dispositions, thread resolution, protected integration and cleanup remain separate pending delivery gates.

Read the complete four-requirement issue, sole nine-criterion plan, T3-completion-review.md and original full/render terminal evidence, G1-remediation.md, G1-qualification.md, G1-root-review.md, actual three-path commit diff and G1 logs/receipts. No new execution, source mutation, Git mutation or forge posting performed. Reused unchanged qualifying evidence rather than imposing another whole gate.

## Current source and real controls

The parser at crates/helper-census/src/glossary/mod.rs:151 refuses a slash-delimited regex when the row's K basis requires literal token survival. It does so before constructing that anchor's patterns and returns the actual row/term/anchor diagnostic. Earlier anchors cannot escape this refusal: the partially assembled row is never returned. Current literal K anchors remain unchanged; non-K regex anchors still use the existing bounded matcher. This repairs the real silent-survival defect without pretending that a regex spelling is a literal token or inventing a second matching implementation.

The new production-entry test at mod.rs:1423 refuses both lone and mixed literal/regex K anchors, then demonstrates the literal K dropped/kept neighbour and non-K negative-lookahead positive/negative neighbours. The authoritative glossary at docs/book/po/glossary-zh-Hans.md:78 now states precisely the literal-only K contract. No current table anchor, rendering or catalogue byte changed.

The real Restore guard at crates/helper-census/examples/glossary_hook_probe.rs:19 still attempts exact restoration. A normal restoration failure panics with path and OS error; it cannot produce a successful qualification. During an already failing unwind, it reports the restoration failure through stderr and preserves the original panic. The ignored stderr result prevents a second diagnostic panic; it does not swallow a restoration failure on a successful path. Existing real-index/catalogue exact-readback checks remain intact.

The three example controls at lines45–85 demonstrate exact original bytes including NUL, a normal failed directory write, and a failed restoration during unwind retaining the original panic payload. The directory control is independent of permissions. All three scratch constructors use the existing testkit TempDir::for_unit_test home and actual Cargo-owned cache admission.

## Actual settled qualification

Worker session99286 ended actual exit0. Read G1-logs/glossary-tests.log (12 passed), example-controls-corrected.log (3 passed, intentional Is-a-directory diagnostic retained), callers.log (2 passed), native-self-test.log (69 rows,43 rejections,24 K,1716 controls), native-scan.log (4181 translated units,3 tracked translated Markdown documents and1716 controls), strict all-target clippy.log, Make helper hygiene, both parity directions/self-tests, fmt and whitespace outputs. Seventeen Rust tests passed with zero failures/ignores; filtered cases are not claimed as executed.

The actual fourteen source/caller/config readbacks all match. The terminal ledger preserves the initial explicit-example compile exit101 from an unavailable compile-time Cargo temporary variable. Only the three new scratch constructors were corrected; corrected explicit-example exit0 is actual, with no fabricated CACHEDIR tag, environment workaround or gate weakening. Historical full exit2, focused101 and coordination abort143 remain historical, not retroactively passed. Root's normal-hook session64170 actual0 committed the unchanged settled source; push is not inferred merely from remote decorations.

## Complete acceptance remains MET

| Plan criterion | Demonstration and current observation | Status |
| --- | --- | --- |
| 1 Native single bounded gate, explicit inputs, hard errors | Prior full/source audit; fresh parser/matcher resource and actual external-input caller tests. Same existing matcher/dependency home | Demonstrated |
| 2 Exact Unicode/boundary/case compatibility | Fresh Python Unicode class/case test and all1716 production controls; adapter source unchanged | Demonstrated |
| 3 Table/PO admission and specimens/self-tests | Fresh table/PO tests, new lone/mixed malformed K refusal, real69-row controls/scan. Current specimens unchanged | Demonstrated |
| 4 Single visible projection, real document locations | Fresh destination/title/multiline/FF/fence/whole-document tests and actual renamed-MD caller; projection source unchanged | Demonstrated |
| 5 Every K token, both sides and neighbours | All24 current tokens in1716 production controls; fresh literal dropped/kept and malformed-combination controls | Demonstrated |
| 6 Real anchored sweep, narrow exclusions and poisons | Current4181-unit scan plus unchanged original41 real paragraph poisons and T3 audit; no new broad exemption | Demonstrated by current scan and reuse |
| 7 Research Object specific typography | Unchanged documented rule and current table-derived controls; prior all4issue audit | Demonstrated by qualifying reuse/current controls |
| 8 Content-selected Markdown/inactive PO | Fresh PO suppression and renamed tracked-MD callers; unchanged selector | Demonstrated |
| 9 Production Make/CI/staged hook, retirement/projections | Fresh caller/parity both-direction omission controls and strict hygiene; actual prior staged-hook/full/render qualification reused. Guard restoration controls fresh | Demonstrated locally; hosted pending |

All four issue requirements retain their established mapping: anchored residual sweep (criterion6), URL-only anchors (4), Research Object typography (7), and per-row K survival (5). All nine accepted plan criteria remain MET locally. G1 specifically strengthens criterion3 malformed-input admission and5 token survival; current actual parser/matcher/caller/self-test/scan executions qualify those changed paths. Criterion9 hook restoration is additionally qualified by the real guard controls.

The other production projection, Unicode/PO/matcher semantics, catalogue, render program, gate callers, staged snapshot mechanism, dependency/feature/layer graph and generated projections are unchanged. Reuse the settled full make-check session56552 actual0 and check-i18n session84112 actual0 at b1833aeed, including seven real catalogue poisons, all six clean rendering arms,33 pages and25 SPARQL fences. Those are prior actual runs, not fresh runs on cdc. The current catalogue-derived controls/scan and caller/hygiene checks demonstrate the changed gate on actual existing inputs. A blanket new full/render run would not close a remaining uncovered changed path.

## Every captured bot concern

| Captured concern | Verified disposition | Publication state |
| --- | --- | --- |
| Inline4215915030 / review5452865445: K regex silently survives | Genuine defect repaired by early row-specific refusal; lone/mixed refusal and literal/non-K neighbours actually pass | Root reply/thread resolution pending |
| Review5452865445 Drop nit | Genuine double-panic risk repaired. Blanket proposed silent normal return rejected because a normal restoration failure must remain hard; exact restore/normal failure/original unwind controls pass | Root explanation pending |
| Discussion6054304914 generic docstring coverage20.97% versus80% | No numeric threshold in repository law or accepted plan; actual module/contract/behavior documentation and strict lint gate satisfy applicable obligations. No waiver or boilerplate padding; explain the non-applicable generic threshold | Root explanation pending |

No missing requested capability is parked in another issue, no deficiency entry is used, and no acceptance is completed by refusing a requested supported input: the refused combination is malformed K configuration; all current supported literal K and non-K regex inputs still work. No current-source blocker remains. PASS here does not preempt new actual review feedback or current-head hosted/protected merge prerequisites.

## Fresh feedback publication/readback

Current tasks/G1-current-{reviews,inline,discussion,threads}.json captures all REST pages and GraphQL outer/inner pages at actual cdc43aaa6. Three COMMENTED reviews now exist: original5452865445 plus5453455257 and5453468765 on cdc, whose bodies are empty. These are not inferred approvals. Three inline entries and three discussion comments include root's actual4216396395 and6055342307; both publish the functional repair, normal-versus-unwind Drop rationale and documentation threshold disposition with accurate local-versus-hosted limits.

The sole thread PRRT_kwDOTKq-Ms6qPaBP is resolved. Bot4216407375 independently confirms the current K parse guard and regressions by source inspection, expressly did not rerun tests/check CI, and accepts that boilerplate for the generic80percent threshold is unnecessary. Thus the table's earlier publication-pending cells are discharged by these current readbacks; no captured feedback is silently ignored.

Current tasks/G1-current-checks.txt reports run37744716706 still pending: four observed active jobs (test/lib, integration2, WASM package and RISC-V more-worker determinism), no observed failed checks. Wrapper exit1 here denotes pending, not a pass. Current local PASS and disjoint-main assessment remain; hosted green/protected integration are not yet proven.
