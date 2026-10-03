# Independent governor profile 10 compliance and golden review

**ACCEPT the reviewed v10 source, documentation, corpus and regenerated traces, contingent on final full qualification. No remaining correction identified in this diff.** Restoring the previous global-consumption undercount is rejected. This review ran no build/test and changed no repository file.

Snapshot 2026-10-03T09:54:22+00:00; HEAD `92842627b840a755a3fca31a78d8aa274a34fa4b`; 20 tracked files in the committed profile transition from `d6f94f62a80c641f3b51734c73f19c64c0140ebc`. SHA-256 of sorted `digest + two spaces + relative path + newline` records: `e7a97aeed67eff73f585d705deb2e9c88eb3bd59c61bc3bd4c8b73b2d607efbc`.

Authority: `docs/SPARQL-GOVERNOR-PROFILE.md` §12 requires an increment whenever a budget trip can move, including unchanged fuel schedules. `vectors/sparql-governors/README.md` explicitly identifies first-party, not vendored evidence and documents complete measured regeneration, freeze refresh and digest re-pin. `evaluator_trace.rs` documents its ignored query/update regeneration paths; `governor_corpus.rs` documents its fuel trace regeneration. The reviewed [qualification addendum](https://github.com/Blackcat-Informatics/purrdf/issues/387#issuecomment-5967880876) amends the prior blanket preservation wording for this versioned first-party budget evidence. Official/upstream and GTS inputs and expected answers remain immutable. The transition fulfills the requested accurate budgets/freshness and `.goals` maximal robustness; it is not an expectation edit used to conceal a semantic defect.

Public identity audit: profile id remains `purrdf-sparql-governors`, version is 10, all 17 fuel labels/costs and STOP_POLL_FUEL4093 are unchanged. Independently calculated profile digest is `d1a2df1c68427c65add1e1d9279bb0d252290f921be8086384b4178531921ea8`. Corpus freeze manifest independently hashes to `ac0b35b6444e5640dca77fc72e083733c646c6c5d567fe67d30d07ae5ff908bc`; both source and prose pins agree. Header, §10 recipe, §12 history/migration and §13 consumer identity consistently publish v10. The new scratch proxy/ownership and immediate reservation/mint checkpoints are described; old ceilings must be remeasured.

Accounting authority: one arena watermark tracks its own computed values/reservations. Aggregate survivor tuples and custom state have separate owners and cannot pay for a later arena result. Clear/replacement/copies/new receipts start their applicable accounting; repeated checkpoints and reattaching the same receipt do not double-charge. Existing stateful child/source/mint controls remain. The new independent built-in/custom aggregate control prices resident inputs0..11 as878 plus a distinct result66 as74, proving952; below-bound951 trips, inclusive952/953 complete, and repeated checkpoints stay952. It does not derive its constants by calling the implementation proxy. The actual corpus instead holds1..12 and sums78:879+74=953. A small custom accumulator adds64, hence1017. Corrected README/module prose now says initial state is always admitted and additional states are charged beyond the chunk threshold.

Every changed governor record reviewed:

| Record(s) | Exact change and independent justification |
| --- | --- |
| `aggregate-accumulation-fuel-boundary.metered`, `aggregate-accumulation-fuel-over-bound.metered` | scratch879→953; final integer78 costs2+40+32=74; every other dimension unchanged |
| `aggregate-custom-fuel-boundary.metered`, `aggregate-custom-fuel-over-bound.metered` | scratch943→1017; the same final result74 is independent of input879+state64; all other dimensions unchanged |
| `aggregate-custom-scratch-bytes-boundary.metered`, `aggregate-custom-scratch-bytes-over-bound.metered` | scratch92202→92280; inputs0..1199 cost90090,33 accumulator states cost2112, result719400 costs6+40+32=78; total92280 |
| `aggregate-custom-scratch-bytes-boundary.spend`, `aggregate-custom-scratch-bytes-over-bound.spend` |92202→92280 in scratch only; other dimensions remain zero because only scratch is engaged in these configured runs |
| `aggregate-custom-scratch-bytes-over-bound.answer` | limit92201→92279 and consumed92202→92280; unknown Group barrier becomes certain positional-prefix with total719400. The refreshed ceiling is full metered cost minus one, so the fold has completed before its separately owned final value crosses at the commit checkpoint. Returning the complete aggregate row is lawful; stopping within state allocation previously supplied no completed group. Budget-exhausted scratch outcome remains. |
| `manifest.tsv` | Only those scratch boundary/over-bound ceilings become92280/92279. All50 case identities, data/query/source/band fields and outcome discriminants remain; transport and relation sidecars are unchanged. |
| governor freeze manifest | Only the nine expected records above and the manifest row digests change. All198 payload hashes independently match on disk; no input case changed. |

Every changed first-party trace block was compared, including both receipt occurrences in query traces and every fuel sweep line. Inventories remain390 query,100 UPDATE and15 governor-sweep blocks; no added/missing/reordered block. Exactly18 query blocks/36 lines,4 UPDATE blocks/4 lines and3 governor blocks/3 lines change. Removing only numeric `scratch-bytes` fields yields byte-identical complete blocks. Answers/canonical hashes, diagnostics, fuel, other dimensions, ledgers, sweep ranges, certificate classes/flags and update store digests are unchanged.

| Query trace case | scratch before→after | Exact new ownership charge |
| --- | --- | --- |
| aggStatMedianPercentile |5556→5779|223: one shared decimal3(73),1.4(75),4.6(75); identical median/percentile3 is interned once |
| aggStatMoments |1716→2046|330: outputs75+73+91+91 |
| aggStatTopk |877→953|76: string5 4 3 |
| aggStatTopkDiscriminating |882→961|79: string50 40 30 |
| groupConcatOrder |216→292|76: stringc-b-a |
| listConcat |35→245|210: six lc1..lc6 reservations at3+32 each |
| listSlice |181→251|70: two lc labels at3+32 each |
| agg-sum-01 |375→451|76: decimal11.1 at4+40+32 |
| constructlist |0→272|eight c-label cells at2+32 each |
| bnode01 |228→456|six separately retained bnode labels at6+32 each; original memo/bag sharing unchanged |
| bnode02 |76→152|two label reservations at6+32 each |
| service5 |222→342|three response-local service labels at8+32 each |
| construct-1 |0→34|one reifier reservation |
| construct-2 |0→34|one reifier reservation |
| construct-3 |0→136|four reifier reservations |
| construct-4 |0→34|one reifier reservation |
| construct-5 |0→68|two reifier reservations |
| select-variable-reuse |584→730|146: independent integer4 and5 at1+40+32 each |

| UPDATE trace case | scratch before→after | Exact new ownership charge |
| --- | --- | --- |
| insert-where-same-bnode |73→149|first c1 reservation34 plus second-operation append0_c2 reservation42; existing count result73 unchanged |
| insert-where-same-bnode2 |73→149|the same two template reservation charges with empty WHERE |
| update-1 |0→68|two fresh reifier reservations34 each |
| update-2 |0→68|two fresh reifier reservations34 each |

The three changed governor sweep blocks change only metered scratch totals879→953,943→1017 and92202→92280, respectively; every fuel sweep observation/answer/certificate remains exact. The separate changed over-bound corpus answer is explicitly justified above, not hidden under a numeric-only claim.

Preservation audit: other freeze manifests are independently byte-identical to HEAD; no official/W3C/GTS input, original expected result, inventory or ledger appears in the profile diff. Root recorded all21 unrelated frozen payload trees matching beforehand and a normal22-root freeze PASS afterward. Unrelated protected main/sibling work and hooks remain untouched. No new dependencies/features, no bypass and no disabled assertion/regeneration-only success claim.

Observed qualification evidence: existing generators completed the complete corpus, governor trace and both evaluator traces. Normal verification log `profile-10-qualification.log` shows evaluator traces2PASS and governor corpus15PASS; ignored writers are not counted as normal validation. Root reports independent952 control1PASS and document claims154PASS. Final full `make check`, wasm, unsharded conformance and generated-artifact qualification must succeed on the final committed source before PR/merge; no pending gate is treated as passed. The previous failed logs are retained as `make-check-before-profile-10.log` and `make-conformance-before-profile-10.log`.

Reviewed file hashes:

| File | SHA-256 |
| --- | --- |
| `crates/sparql-eval/src/governor/mod.rs` | `13889106399497132d310b020260521c2e74ecdd26a7ea0453b302d97831199f` |
| `crates/sparql-eval/src/governor/charge_points.rs` | `20455898f248070fd2c3159ceed448b40e1c1183add42ce66e9fc6e0b0e60be2` |
| `crates/sparql-conformance/tests/governor_corpus.rs` | `833c8ec51be527850ef54e0d53a50fda00976e1febaa82a37ae35e3b68c40569` |
| `docs/SPARQL-GOVERNOR-PROFILE.md` | `fd9a5ac9b71dacd3f9b2cefd05992cb9c39d8843a384e8a317aed78de1b56ebc` |
| `vectors/sparql-governors/README.md` | `ab9195b98ba013da7fba0aac57d3736e0572c3065e304e322da21ec98c28ba54` |
| `scripts/conformance-frozen/vectors-sparql-governors.sha256` | `ac0b35b6444e5640dca77fc72e083733c646c6c5d567fe67d30d07ae5ff908bc` |
| `crates/sparql-conformance/tests/goldens/evaluator-trace/suite.trace` | `4f0512410e5e9e1650b09935da39e86a1ec7d75de4ca520cf99abe8f646a910b` |
| `crates/sparql-conformance/tests/goldens/evaluator-trace/update.trace` | `b13df16dd6c50e8d373534c2c281db3b8f33fd85f1cc59e631b1a22eb856d746` |
| `crates/sparql-conformance/tests/goldens/evaluator-trace/governors.trace` | `a3f42b872deda5831c7124d1957289d455e5ecfef0554502d4701db04ac403bf` |
