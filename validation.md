# Role refusal source retention

The baseline public regression failed with exit 101 at `original source must be retained` (baseline-role-refusal.log). The original InvalidProperty classification was already present, but its presentation was absent after the source interner died.

The production repair changes one mapping line in CeExtractor::role_of: the existing native with_source helper is called with the actual borrowed interner before converting into EntailError. The canonical helper and its allocation/cancellation behavior are unchanged; no grant, formatter, error enum, input filter or fallback is added. Valid input follows the unchanged success path.

Current checks all pass:
- role-tests.log: all 17 public regular-role tests pass. The new case covers annotation-property IRI and literal restriction roles in ordinary and proof-recording construction, four combinations total. It drops the dataset before asserting typed InvalidProperty classification, named DiagnosticValue::Text source parameter, preserved spelling and original typed error chain.
- entail-tests.log: 718 library tests pass, zero fail, one is ignored.
- reasoner-tests.log: all 56 public reasoner service tests pass, covering the broader public callers of class-expression extraction.
- clippy.log: all-target entailment clippy passes with warnings denied.
- wasm-role-tests.log: three actual wasm32 cases pass: the new constructor/source matrix, the existing RBox source-witness/top-role control and the existing cancellation control.
- The new portable case is registered in both the harness and the exact WASM Make inventory.

These are explicit tiny synthetic inputs; no GMEOW corpus production, runtime host identity, machine/resource/environment probe or profile weakening ran. The deficiency ledger has no entries. Native issue 558's broader error inventory and GMEOW's class-model delegation are not claimed completed by this producer fix. Independent subagent dispatch is unavailable; no such report is fabricated. Full hosted acceptance remains separate under the existing instruction to merge and handle subsequent CI findings.

Commit bfd8e4af9 passed normal hooks and push. The fetched actual base remains 0994332d93bc89747f95e5a942715faf36129648, and the clean integration prediction is 2312e735662612c023b38a2c09565b003ea7b210. There is no intervening base delta or conflict invalidating the checked source. CodeRabbit explicitly refuses review because the organization usage cap is reached; it reports the next included review in 10 minutes. No completed independent review or full hosted CI pass is claimed. Existing user authority permits merging with external checks pending; no required implementation behavior is deferred.
