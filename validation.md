# Current acceptance: required local qualification PASS

The approved plan is `plan.md`; independent plan judgment is PASS in
`reviews/plan-review.md`. Final task review PASS is in tasks/T1-review.md.
Normal hook commit45d1fb09f and branch push succeeded. PR529 is OPEN at
https://github.com/Blackcat-Informatics/purrdf/pull/529; its live body/head/base
were confirmed after creation. Hosted checks are pending, and integration,
merge and issue closure are not claimed. Publication details are in publication.md.
Root independently reviewed the complete implementation: source PASS is recorded
in implementation-review.md. Normal publication is authorized after the remaining
full gate and final task-review verdict pass.
The assigned base remains unchanged; no synchronization has been performed.

Source uses the original native renderer and the shared lex WorkList, permanent
blank structural-key guard40, indentation saturation40, the original strict
collection walker over indexed renderer edges, existing term writers and
first-party benchmark/testkit runtime. Singleton objects do not compute unused
ordering keys. Cached suffixes cannot bypass the blank depth guard. Competing
equal bounded keys use authored term identity rather than local TermId order.
No dependency, semantic feature, vocabulary default or enlarged stack is added.
The frozen-dataset events16 term-depth law is unchanged.

Independent root review found the original ObjKey PartialEq still compared local
IDs while the new Ord compares key and authored identity. Corrected PartialEq to
delegate cmp.is_eq(); added actual singleton/competing blank equality, ordering
and BTreeSet-cardinality control. Exact neighbors now also compare actual named
TriG under reversed interning/rows. Initial final-source identity is preserved in
final-source-sha256.txt; correction identity is review-suffix-source-sha256.txt.
Ongoing full/benchmark evidence is preserved; affected strict/native/portable
suffix execution qualifies this isolated correction without another whole gate.
Strict correction PASS strict-review-suffix.log terminal0. Direct native key
control PASS native-key-review-suffix.log terminal0, one actual selected test;
Both public corpus suffixes PASS terminal0: native-review-suffix.log7/7 in11.04s and
portable-review-suffix.log7/7 in18.69s. All six frozen output receipts remain
identical, and exact neighbors include actual named-TriG permutation. The source
review is PASS in implementation-review.md. The original full gate session95104
finished with exit code0; full-check-final.log ends with the optimized wasm
release build completing in7m18s. Native workspace tests/doctests, the standalone
preserve-order consumer, all static/hygiene/generated gates and wasm build pass.
The new public Turtle corpus also passed7/7 inside the full gate in11.73s, with
all six frozen100k/1M receipts identical. No second whole gate was invoked.

| Criterion | Current evidence |
|---|---|
| Source implementation | PASS independent implementation-review.md after Eq/Ord correction. Complete batch in turtle_render.rs, public shared corpus, benchmark and wasm registration. |
| Affected strict compilation | PASS strict-review-suffix.log terminal0, core/RDF all-target clippy -D warnings on corrected source. Historical iterator, support visibility and single-character-pattern lint failures are corrected; no lint bypass. |
| Affected native package checks | Earlier broad core/RDF suites passed before a new-corpus count oracle failure (two root objects share one predicate, so201 rather than202). Corrected control now passes in final native/portable7/7 corpora; direct key consistency test also passes1/1. Earlier typing/import failures remain recorded historically. |
| Original exact neighbors | PASS: Stage-only original-source Cargo probe, baseline-cargo.log terminal0. Original/current .ttl captures at0/1/2/31/32/33/39/40/41; all term/punctuation lines unchanged, original indentation<=40 exactly unchanged. Actual depth40 delta4 spaces and depth41 delta16 spaces are saturation at actual levels41/42. Direct-rustc failures retained historically, superseded by ordinary Cargo probe. |
| Native actual100k/1M routes | PASS native-review-suffix.log terminal0, all7 exact cases,11.04s. All six actual default-stack route receipts asserted from frozen literals, including statement metadata round trips. Historical exact-selection invocation refusal retained and corrected. |
| Portable actual100k/1M routes | PASS portable-review-suffix.log terminal0, all7 exact cases,18.69s. Actual optimized wasm runtime through the existing runner; all six native frozen byte lengths/digests match, named graph and statement metadata round trips included. |
| Guard, cycles/shared/quoted, long collection | PASS final native/portable suffixes: exact golden/permutation including named TriG, cycles/shared/quoted fidelity, two guarded100-node branches, actual100k collection, continuation objects at40/41. |
| Benchmark | PASS benchmark-final.log terminal0,3 measured/0failed,10 samples each. Medians depth8 4.645µs,100k236.3ms,1M2.928s; native release profile. Original JSON estimates preserved in benchmark-estimates/. Timed renderer/fixture code unchanged by isolated Eq coherence correction; initial artifact identity final-source-sha256.txt retained, no changed-source artifact claim. |
| Single full qualification | PASS original session95104 terminal0, full-check-final.log. Full native workspace tests/doctests, preserve-order consumer, static/hygiene/generated checks and final optimized wasm release build passed. Public Turtle corpus7/7 in11.73s. |

Managed local compiler: rustc1.100.0-nightly(4b6d04e70 2026-09-13).
Required debug assertions and overflow checks remain enabled; optimized workspace
gate profile remains intact. No required hook is bypassed. Normal commit, push and
PR publication are authorized once required qualification and task review pass;
merge remains with the coordinator.

Baseline neighbor policy counts actual indentation levels: a chain of40 inline
blanks has its terminal predicate on level41, so saturation first changes that
line. Original lines at levels0..40 remain byte-identical; all punctuation and
terms remain identical at every captured neighbor. The assigned policy does not
introduce the donor's independent32 ceiling.

## Actual main integration and published review remediation

PR529's initial head45d1fb09f conflicted with origin/main eb1fd88b in the
Makefile's portable selection chain. Both complete registrations were retained:
the seven Turtle cases and sixteen regular-role cases. Ordinary synchronization
commit42ca16721 passed normal hooks and was pushed; renderer source is unchanged.
The combined source's strict core/RDF all-target check passed in
integration-suffix.log; its original native seven-case corpus and optimized
portable same seven-case corpus remain running in original execution23493.
Unchanged full gate, benchmark, key-coherence and exact original-neighbor evidence
remain applicable as independently assessed in completion-audit.md.

Published review found stale WASM summary counts/exception text; corrected to
57 named cases,16 targets,62 executions and20 invocations, documenting both added
portable semantic targets. Commit6a5913485 passed normal hooks. Hosted assembly
checks found the new benchmark absent from the required census; mapped it to the
actual shared IRI egress site core.canon-escape. The original document coverage
validator passes in simd-doc-coverage.log; no assembly measurement is claimed by
that document-only check. Commit8b7cb5542 passed normal hooks. Fresh hosted checks,
review resolution, completed integration suffix and final merge remain required.

Integrated native corpus is now terminal PASS7/7 in12.05s inside the original
integration suffix23493. All six frozen packed/default Turtle/named TriG
100k/1M length/digest receipts match. Optimized portable compilation continues
in the same original handle; its runtime result is not yet established.
Independent review confirms both fixes, accepted reviewer response and resolved
thread. Fresh final-head hosted runs remain pending.

Integrated suffix23493 is now terminal PASS, exit0: affected strict core/RDF
all-target qualification, native7/7 in12.05s and optimized portable7/7 in24.13s.
Every original six-route frozen receipt remains exact on both targets. This
qualifies actual combined renderer behavior; the later changes are documentation
only and separately covered by normal hooks/document coverage. Fresh final-head
hosted checks remain the only uncompleted qualification prerequisite.
