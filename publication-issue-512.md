# Issue #512: text/sparql-eval: dictionary-match shape -- a VALUES block of phrases on one predicate as one automaton, longest-first spans

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github

## Body

## Summary

An executor shape: **a dictionary match**. A `VALUES` block of N phrases joined to text occurrence on one predicate should run as **one compiled automaton** over each document's analyzed token stream, not as N lookups. It should also support **longest-first, non-overlapping** selection over the matched position spans. The results must be identical to the literal evaluation.

## Why

Two workloads ask exactly this, over every record, often as a standing query at intake:
- `lillith_memetics` matches a 534-name canon lexicon and per-story alias tables (`docs/design/sprint-procedures-as-sparql.md`);
- Katamari's mail package runs SIEVE `:contains`/`:matches` rules (ADR-0017).

Context: Katamari ADR-0045 45.16 (Proposed). Both workloads can be written today: `TermOccurrenceRelation` (`text/src/relation.rs:1662`, at `3b6ec4d9f`) gives token positions, and a multi-token phrase is calls joined on `?doc` with consecutive positions. Longest-first is `FILTER NOT EXISTS` over overlapping spans. Literally evaluated, though, that is one relation call per phrase per token, which is the wrong cost shape at a 534-entry dictionary.

## What it needs

- The planner recognizes the shape: `VALUES ?phrase { ... }` decomposed into tokens through the index's own analyzer (so normalization, e.g. "sìth" matching "sith", is the analyzer's), joined to occurrences on one predicate.
- One automaton (Aho-Corasick over analyzed tokens) per dictionary and analyzer identity, built once and reused.
- Output rows carry the matched phrase (or its entry identity), `?doc` and the start and end positions, so a longest-first, non-overlapping selection is expressible and fast.
- **Exactness:** a recognized shape never changes an answer. Rows equal the literal evaluation, byte for byte.

## Acceptance criteria

- On random dictionaries and documents, the shaped evaluation equals the literal one.
- Cost is one pass per document for the whole dictionary.
- Longest-first, non-overlapping selection matches a reference implementation.
- No compiled-in cap on dictionary size.


## Comments (1)

### paudley — 2026-10-10T15:46:55Z

# Combined implementation plan: exact physical shapes and dictionary matching

## Authority and delivery boundary

This is the one authoritative implementation plan for the complete current contracts of #518 and #512. Intake is issue.md (full #518 body, zero comments) and issue-512.json (full #512 body, zero comments), captured through Stagectl on 2026-10-10. Both were OPEN. Neither contract is shortened by this plan.

Worktree: /home/paudley/Active/purrdf/.worktrees/518-optimizer-a-shapeprovider-extension-for  
Branch: paudley/518-optimizer-a-shapeprovider-extension-for  
Selected Stage: .stage/optimizer-a-shapeprovider-extension-for  
Initial base: origin/main at a4bb1232d3b4bdf682eab126504dd942ce2b3d5b

Root AGENTS.md, .baseline, .goals and the current Stage1/Stagectl skills govern. Main, REWORK_LIST.md, unrelated worktrees, existing experiments and any dictionary donors remain untouched. Stage evidence is process material, never source. Root reviews this combined plan independently before shipping-source edits. There is no child delegation, model/backend action, outside-repository submission, new runtime dependency, semantic feature, fabricated vocabulary or chosen semantic size limit.

The delivery contains the shared ShapeProvider seam, dictionary shape/automaton/cache and span consumers, ordered-window shape, typed bucketed-series shape, host registration/refusal, all current evaluator entry paths, original workspace/error laws, and evidence propagation through the landed #515 public carrier. It is one complete behavior batch before consolidated qualification. #515's current private source is not copied: its supplied public carrier contract guides integration, and the actual landed combination must be qualified before this delivery is publishable. Missing required evidence propagation blocks completion; it is not a follow-up.

## Governing source and prior-art fit

Stagectl's prior-art search covered the last 200 commits and counted 22 none, 2 performance and 1 semantic-identity-loss trailer instances. These are counts within that search, not proof of no other recurrence. It found no PurRDF docs/adr/0045; the cited source was resolved read-only to Katamari docs/adr/0045-programs-are-sparql-rules-and-ops.md (45.16 and 45.19) and the exact docs/design/PURRDF-SEAM.md at e65fe50, §2.3. They require one PurRDF planner and exact general/host shapes, not a second host planner. Katamari ADR-0017 and lillith_memetics/docs/design/sprint-procedures-as-sparql.md supply the actual contains/aliases/window/day-series workloads.

The recovery packet combined-text-retrieval-preparation-20261009.md is useful design evidence, not qualification of current source. Its 512 domain/multiplicity/cache warnings remain valid; its old fixed gate ceiling is overridden by current AGENTS.md.

Current production homes:

- sparql-eval/property_fn_plan.rs, blank_scope.rs and join_plan.rs perform registry feasibility, blank correlation and logical rewriting. engine.rs preparation and eval.rs::prepare_query_context_over install the immutable numbered plan. Physical selection belongs AFTER those logical changes, and after per-execution substitution when it changes a subtree.
- plan/mod.rs::PlanShape/PlanHandle already own dense NodeId side tables; eval.rs::dispatch_node and RowDelivery are the one materialized/yielded execution funnel. Keep the logical AST unchanged and attach selected physical plans to this existing numbered execution owner.
- rdf-core/dataset_view.rs::AccessCost and sparql-eval/bgp.rs::PatternCost are landed #517. Work ranks accesses; rows advance cardinality. They are estimates, NOT storage admission certificates or governor charges.
- solution.rs owns immutable VarSchema, RetainedRow, RowBag, SolutionSeq and native builders; workspace.rs owns WorkspaceCapability/WorkspaceAllocation/WorkspaceTerm, fallible vectors/shared controls and original first-error latches. Native shapes use these homes.
- property_fn.rs/ property_fn_eval.rs are the existing contained host declaration/open/pull and typed error boundary. ExtensionEnv is the one options-carried registry environment. Add physical registration there without introducing a second parser configuration.
- text/analysis.rs::Analyzer is the only dictionary/document analysis law. TextIndex assembly in index.rs retains (term ordinal, position, predicate ordinal) while analyzing but currently discards the stream after postings; document identity is (graph, subject, language), while TermOccurrenceRelation emits subject/language/position and omits graph.
- text/relation.rs::TermOccurrenceRelation already uses admitted analysis, cursor and term ownership and a deterministic partition/document/position order. Its multi-token rejection stays unchanged. text depends on eval; eval MUST NOT depend on text.
- modifier.rs::eval_group_with and its contextual aggregate step/finish own grouping identity, first-seen order, numeric folds, DISTINCT, FOLD/custom states and opaque truncation. Bucket execution reuses these bodies rather than introducing a second aggregator.
- xsd/temporal.rs::cast_calendar, parsed_value.rs and native_numeric.rs own typed temporal/ordinal parsing, equality, arithmetic, rendering and physical layouts. No substring date, fixed-second month, floating arithmetic or second temporal parser.

No registered local #512 worktree/branch was found at intake. This is not a statement that no donor history exists. Preserve all existing donor branches/worktrees; any subsequently found owned donor is read/adapted, never reset or deleted by this implementation.

## Chosen physical extension design

### One registry and one runtime funnel

Add an object-safe ShapeProvider declaration/proposal seam in sparql-eval, registered by a caller-owned ShapeRegistry carried through ExtensionEnv and every existing QueryOptions.env path. Built-in providers are general evaluator operators; dictionary support is a neutral capability supplied by a registered text occurrence/phrase producer. No literal IRI is interpreted as a shape merely because its spelling looks familiar.

The seam is physical: registration can change work and its report, never logical answers or parsing. The existing logical parser environment remains authoritative. Provider identities/declarations get a separate physical registry identity/fingerprint in preparation/execution-plan keys and EXPLAIN; logical content identity is not falsely changed because an equivalent access was selected. An empty host registry preserves literal execution where no built-in applies.

Concrete interfaces to implement at this home:

- ShapeProvider::offer(ShapeRequest, &WorkspaceCapability) -> Result<Option<ShapeOffer>, EvalError>. ShapeRequest borrows the actual rewritten subtree, its numbered/schema/bound context, active graph/dataset, producer declarations, scope restrictions and available AccessCost information. None is a semantic non-match/decline; an operational, storage or contained host failure is Err.
- ShapeOffer contains an immutable compiled physical operator, original covered-node/input-edge mapping, exact output schema/domain/multiplicity/order contract, physical identity and comparable work estimate. Offers cannot capture a borrowed AST or query reservation into a longer-lived cache.
- A narrow object-safe ShapeRuntime gives compiled host operators owned/borrowed original input-row handles and native comparison/term-production/output-building operations. The generic implementation remains in the existing EvalCtx<D>/RowsBuilder/RowDelivery home, so DatasetView::Id never becomes an invented global ID or a new join remapping table. The external row handles carry their original input lease; they do not expose mutable raw rows, evidence-table replacement or ownership-detaching Vec extraction.
- Host output references original rows/columns (and explicit admitted WorkspaceTerm producers where the equivalent logical operator computes a value). The engine's original logical row/evidence builders perform Retain/Join/Group publication. No host-created row can claim an arbitrary evidence ID or omit bag search scope.
- Built-in kernels execute through that same native row pipeline. Do not add a parallel engine facade, a family of shape-only query entry methods, a generic QueryOptions migration, a second expression evaluator or a physical GraphPattern variant.

Provider consultation and selection are once per actual execution-plan subtree/context, not once per output row. Selected maximal regions prevent overlapping offers from executing the same logical node twice. Children execute through existing native dispatch. Refuse a mismatched schema, unsupported scope/domain/order contract or incompatible identity before opening its physical producer; retain the original logical operator when an offer is legitimately declined. A failure AFTER an accepted physical operator begins aborts with the original cause, never silently retries literally.

Use existing contained host-callback helpers for provider declarations/proposal/open/pull; all actual callback/control allocations are admitted before construction. Check declared output width/modes and bound/equality filters with the existing property-function row admission law. A declaration is an honest host contract, not an engine proof of arbitrary external code; differential production fixtures must establish each shipped/fixture host operator's exactness.

### Correct execution and preparation placement

Physical selection is attached to PlanHandle/its execution-owned side table after the normalized logical AST is fixed. Prepared execution preserves logical parsing and substitutes parameters through the existing native rewrite. A subtree whose values/bindings changed receives a context-correct physical selection; unchanged immutable portions reuse compiled plans under the correct key. Correlated EXISTS/APPLY/LATERAL and rewritten copied trees use the same numbered-plan installation, not a root-only bypass.

Every production entry reaches the same dispatch interception: direct text/algebra queries, SELECT/ASK/CONSTRUCT/DESCRIBE, prepared query/execution, interned/scoped visitors, governed/fallible reads, EXPLAIN, UPDATE WHERE, remote/correlated child views, user-function bodies and native parallel forks. Central installation/dispatch handles those paths; only actual missing configuration propagation needs caller edits. Missing producer registration still follows original hard-error/ordinary-data interpretation, not an invented namespace.

Provide one public non-semantic physical-execution policy to disable shape selection for differential execution on the SAME query/env/dataset. It keeps logical normalization, source lookup, active dataset, semantics and producer registration unchanged. It is a runtime diagnostic policy, never a Cargo feature or an alternate language. EXPLAIN records selected provider/operator, estimate versus actual visits/transitions/output, semantic non-match/decline and original logical-node ownership. It must not make a physical estimate an admission fee.

## Dictionary recognition, literal path and automaton

### Concrete recognized requests

Provide an analyzer-owned phrase-occurrence adapter over a registered TermOccurrenceRelation capability, under a caller-supplied IRI. It accepts a phrase plus language/start/end positions, exposes its own neutral dictionary capability, and its literal path expands a phrase through the SAME analyzer and original token-occurrence adjacency law. Existing single-token occurrence behavior and its multi-token refusal do not change.

The concrete ordinary SPARQL request is, for example:

    VALUES (?entry ?phrase) {
      (<https://example.org/lana> "lana")
      (<https://example.org/lana-audley> "lana audley")
    }
    ?doc <https://example.org/matches> (?phrase ?lang ?start ?end)

The IRI is supplied/registered by the caller; no vocabulary is minted in shipping code. Shape-disabled execution drives each VALUES row through the ordinary phrase adapter. Its literal oracle is also checked against explicit original TermOccurrenceRelation calls joined on doc/language and p_next = p_first + offset. Recognize equivalent already-expanded occurrence-call/VALUES/UNION adjacency regions using the same certificate, including all original VALUES payload columns and intermediate position bindings; do not require a new private generated-query API as the only optimized caller.

Recognition proves: one registered occurrence predicate/capability; actual analyzer; token adjacency and original position coordinates; logical document/join key; graph/language/predicate projection; repeated-variable/bound-position equality; full original schema and VALUES multiplicity; original row traversal order; safe residual-filter/outer-context placement. Query/corpus restrictions from #515 are applied at the original eligible input stage, before any answer depth or heap. Bound parameters participate in the same recognition/compiled-cache key.

A general unsupported expression is evaluated by the existing literal operator. This is not permission to reject the requested VALUES/phrase workload: the concrete one-predicate phrase and expanded-adjacency forms above are required optimized production paths, including parameterized/prepared execution and named-graph collision controls.

### Actual token stream and exact bags

Retain a compact ordered token-occurrence view during TextIndex construction, with term ordinal, original position, predicate ordinal and original occurrence/partition ordinal. Share native dictionary/document data; do not keep a second set of per-token Strings or re-analyze documents per query. The view's order is a derived representation, not a gratuitous scoring/index fingerprint change. Any new semantic predicate projection or phrase law has its own explicit identity.

Predicate scoping preserves original positions and resets matching at gaps; deleting other-predicate tokens must not manufacture adjacency. Preserve current literal-concatenation boundaries exactly. Half-open [start,end) spans use checked end arithmetic; source position representation remains its existing law, not a new dictionary cap.

For the usual unique physical document per logical subject/language/domain, run one deterministic Aho–Corasick token automaton per stream. Named graph/document identities are NOT silently assumed to be visible join keys. When original occurrence calls merge multiple physical partitions into the same logical subject/language key, reconstruct the literal join's alternatives and multiplicity from the once-read position/occurrence stream: admitted active automaton states with retained original occurrence-parent references, emitting the required occurrence combinations in literal traversal order. A position gap resets this frontier. This supports the requested workload even with hidden graph aliases; it does not silently refuse such dictionaries or substitute graph-local phrase semantics.

Every source occurrence is visited once for the entire dictionary. Alias-frontier transition work and required output multiplicity are separately measured; no claim that inherently many literal derivations are constant work. There are no N posting lookups or N document analyses. Original VALUES row ordinals, original entry/phrase spelling, duplicate rows and duplicate normalized sequences are retained as distinct derivations. Empty analyzed phrases follow the established literal empty/refusal law; no empty-pattern everywhere matches. All intermediate/output bindings and column order are reconstructed from the recognition certificate through the original row builder.

Restore the deterministic LITERAL ordered bag, not merely set/bag equality or a new physical scan order. Carry literal traversal/occurrence ordinals and use admitted ordering only where physical scan order differs. Equality of binding terms, lexical values and output bytes must remain unchanged.

### Automaton, cache and ownership

Implement one first-party token Aho–Corasick home in text, with deterministic ordered edges/failure links/outputs. Use native shared work lists/maps/fallible storage; no aho-corasick dependency, second Unicode table, ad hoc parser or compiled dictionary-size/phrase-length ceiling. All capacities/indices are checked from actual dictionary token/entry counts and actual physical layouts.

Separate reusable analyzer/dictionary compilation from binding it to a specific index's token ordinals. The automaton key includes the complete ordered VALUES dictionary with entry identities and multiplicities, analyzer fingerprint/table law and explicit phrase/match law. Index-generation/domain/projection/eligibility identity belongs in the binding/execution key. Thus rebuilding an index under the same analyzer reuses the dictionary automaton but cannot reuse stale occurrence IDs or scope membership.

Compiled automata and bindings are immutable owner-carrying shared artifacts retained by the registered producer/prepared-plan cache. Native query-produced nodes/edges/output arrays/normalized terms/cache keys/shared controls/pins are admitted BEFORE allocation through the existing WorkspaceCapability. Actual physical allocator failure is typed. Resident and bounded execution share the same body. Caller-prebuilt artifacts remain actual caller ownership; a cached engine-created artifact retains its original account through every clone, extraction and eviction. Traversal/frontier/output scratch has its own current invocation owner. No post-construction census, chosen multiplier or cache hiding after refusal.

Cache entries use the existing bounded/explicitly configurable cache policy, with no semantic dictionary limit and no unbounded anonymous thread-local. Disabled retention affects reuse only. Eviction releases ownership after payload destruction, source/index replacement cannot reuse a stale binding, and a failed build never publishes a cache entry.

## Span selection is a separate explicit logical law

Ship all-match span output and a public owner-preserving span-selection consumer over actual matched rows, including an ordinary query-result-to-selection entry path. Longest-first non-overlapping selection is an explicit caller-selected logical operation, never a physical flag that silently changes a SPARQL answer.

Freeze the geometric law: within the declared logical occurrence domain, order distinct spans by descending token length, then ascending start, then ascending end; greedily retain spans disjoint from those already selected. Adjacent half-open spans do not overlap. Every original entry/VALUES/derivation row attached to an identical selected span survives, preserving multiplicity and evidence handles. Return rows in the input's original stable literal order. An independent simple reference enumerates geometric spans and already-selected intervals; it does not call the production automaton/selector.

The memetics example's FILTER NOT EXISTS rejects strictly longer CONTAINING spans. It is NOT greedy non-overlap and can preserve partial overlaps/equal spans. Preserve that ordinary filter exactly; an optimized containment-suppression kernel may replace it only after proving its exact predicate and tie behavior. Never label a containment/priority-domination predicate as the greedy law. Support the complete explicit greedy selector and all-match query output in this batch; document the distinction on the public API and test both with overlapping/equal-length counterexamples. No new query dialect or implicit function namespace is required.

## Ordered-window shape

Recognize ordinal arithmetic over a common (sequence,position) relation: bounded directed or absolute windows expressed by existing comparisons/arithmetic, including the exact source expressions and complete left/right binding domains. Evaluate the source occurrences once through existing native reads and retain original row ordinals/evidence. Source access may attest a matching physical order; otherwise use the one admitted stable ordered input preparation, explicitly reported, followed by ONE ordered sweep.

Partition by the original sequence identity; parse/compare positions through native numeric owners, not term-ID or lexical order. Maintain the native window/frontier and emit all required ordered pairs, including duplicate equal positions, multiple occurrences, empty/zero-width/asymmetric windows and clipped edges. Reconstruct original pair/binding/output order rather than exposing sorted traversal order. Integer bounds/offsets use existing exact Integer/Decimal helpers; overflow, ill-typed/nonintegral positions, unbound inputs and expression errors preserve the literal path's actual outcome. No chosen maximum width or sequence length.

Residual filters and ordinal barrier NOT EXISTS use the original expression/correlation/evidence law. For the stated between-position barrier form, prepare admitted ordered barrier occurrences/prefix state once and consult the exact condition while sweeping; preserve graph/sequence domains, duplicate support, negation closure and invalid/error rows. Do not treat an arbitrary NAF body as this certified barrier or leak an outer binding into a source that did not see it. All other retained residuals still run before semantic LIMIT/answer admission. A matched region cannot count doomed pairs toward a pushed ceiling.

## Typed bucketed-series shape

Recognize Group over existing typed calendar truncation/cast bindings, including day via xsd:date and month via xsd:gYearMonth, with any preserved grouping keys/aggregate outputs. These target IRIs are the existing datatype home, never raw W3C literals. The literal query uses the same actual casts, so xsd:dateTime/dateTimeStamp, timezones, absent timezone, 24:00 normalization, leap boundaries, negative/large years, invalid lexical and unbound keys retain native semantics.

Build the key through the existing parsed-value/cast/render/interner home. Do not normalize all zones to UTC, compare only timestamps when the literal GROUP uses final RDF binding identity, use lexical substring or approximate a month by seconds. Additional key columns and unbound states are preserved.

Reuse modifier.rs's native aggregate state/step/finish and group-domain identities. Add an ordered-input execution mode at that original home: a certified source order or one explicitly admitted ordering preparation, then ONE ordered grouping pass. Keep groups in original first-seen order. Keep each group's original input order for Sample, GROUP_CONCAT, rounding-sensitive numeric folds, DISTINCT, FOLD/custom accumulators and expression error precedence; physical reordering cannot change these values. The kernel must support every existing aggregate carried by a recognized bucket group, not close this issue with only COUNT and leave other aggregates to a foundation task.

An incomplete/truncated child remains an opaque Group edge under the existing lift/certificate law; no aggregate over a partial series is published as complete. Missing time/function vocabulary or unsupported expression shape remains the original literal interpretation, never a fabricated default truncation function.

## Evidence, scope and operational ownership

Root supplied #515's public contract: RetainedRow<I>::evidence/with_evidence; clone_admitted keeps its original dictionary owner; binding Eq/hash excludes metadata. RowEvidence is immutable, default id0 without a table owner; QueryEvidence::new/retain and RowEvidence::combine/with_record are fallible original-owner operations. Conjunction and Alternatives have different logical laws, and IDs are owner-local. RowBag::search_evidence retains invocation scope including zero rows.

Integrate the actual landed carriers in this delivery. Selection/reordering forwards each original RowEvidence handle unchanged. A logical join/group still calls the SAME original evidence combiner in the SAME semantic premise/order law; a physical operator must never invent a new proof because it executed differently. Dictionary occurrence combinations retain precisely their original source/invocation support and VALUES multiplicity. Preserve the bag search scope for zero and nonzero outputs, input restrictions and completeness/undecided/refuted facts. Add the necessary narrow public admitted row/bag publication door at the owning solution/evidence home for an external ShapeProvider; do not expose a private setter by copying source, create a duplicate evidence carrier or rebuild evidence by census after row birth.

Raw numeric evidence IDs from separate query owners are not comparable. Prove unchanged IDs for passthrough/reordering in the SAME owner; prove canonical record identity for independently executed shaped/literal derivations, plus shared-owner equality controls for logical combinations. Canonical records must exclude physical implementation details.

Root's reached-invocation refinement is also mandatory: forward the WHOLE original RowEvidence/bag search handle, including reached invocation closure stored independently of record completeness. Never extract/reconstruct only EvidenceRecord. Clone/combine/project preserve that handle with no default row/table allocation. A completely searched open-world SERVICE remains Complete while ClosedAbsence stays Unknown; physical selection must not change FOUR/frame or conflate those two facts.

All shapes retain the original input/output leases and typed source-error precedence. Admission failure happens before real growth; native allocator refusal is typed; stop/governor trip uses the existing Lift/RowDelivery/charge/certificate machinery at covered logical nodes and original cause sites. Preserve root read checkpoint precedence and zero-byte execution permits. Shape estimates never stand in for actual buffers or semantic completeness. EXPLAIN and returned/partial/retained results keep original owners through clone/extraction/drop, with allocation-free failure snapshots. Host refusal must occur before reads, physical failure after acceptance must remain visible, and LIMIT only skips work with the original soundness certificate.

## Completeness contract

All rows belong to Task 1 source + focused acceptance and Task 2 complete qualification/publication. A test-only helper does not discharge production-path rows.

| ID | Required behavior | Falsifiable production/acceptance demonstration |
|---|---|---|
| S1 | ShapeProvider after logical rewrite, one planner, real host registration | Ordinary query through NativeSparqlEngine with caller-registered host provider; record offered normalized subtree and selected physical execution; host declines a neighbor, literal answer is exact; wrong schema/certificate and contained failures are typed. |
| S2 | All three shipped shapes | Ordinary text/algebra direct and prepared requests execute dictionary/window/bucket physical nodes, confirmed by EXPLAIN/visit counts; each compares exact ordered binding rows/schema and result bytes to shape-disabled literal execution. |
| S3 | Exactness/determinism, approximate exclusion | Testkit generated dictionaries/windows/calendar series plus invalid/bound/unbound/duplicates/scopes; repeated runs, reversed registration/build order and sequential/parallel execution preserve outputs. An approximate/uncertified offer is declined before source work. |
| S4 | Original per-row evidence and search scope | Actual #515-enabled query with nonzero evidence, BOTH/refuted/undecided/NAF controls and zero rows; original handles/owner-local IDs, canonical combinations and bag scope preserved through all three shapes/host, projection and retained extraction. |
| D1 | Own analyzer, VALUES decomposition and full output | Actual phrase adapter and existing expanded occurrence adjacency query for entry/phrase/doc/lang/start/end/intermediate bindings; diacritic/case/stemming/CJK profile cases, original phrases and duplicate VALUES survive. |
| D2 | One reused token automaton per dictionary/analyzer | Public direct + prepared/repeated calls show one compilation on repeat, new dictionary/analyzer invalidates compilation, new generation invalidates only occurrence binding; complete cache identity and retained-owner/drop/refusal controls. |
| D3 | One source pass per document, not N lookups | Real 534-entry canon-shaped fixture and larger dictionaries read each immutable stream once; analyzer/document-scan/posting-probe/transition/output counters establish actual work and no per-phrase source lookups. |
| D4 | Exact random dictionary/document rows | Independent literal original occurrence joins vs shaped ordinary queries over random dictionaries/documents; ordered bags, row schemas, entry multiplicity, predicate gaps, same subject/lang in named/default graph aliases and literal concatenation all match. |
| D5 | Longest-first non-overlap | Public owner-preserving selector over actual query matches agrees with independent reference on nested/crossing/equal/duplicate/adjacent spans, retains identical-span entries; separate actual containment-filter counterexample prevents semantic substitution. |
| D6 | No dictionary cap, truthful native failures | Empty, singleton, 534, 4097+ dictionaries and arbitrarily long representable phrases use the same path; source inspection has no chosen cap; checked physical/layout overflow and tiny-budget allocation refusal preserve first cause/no partial cache/output, healthy budget returns full exact result. |
| W1 | Ordinal one-sweep window | Ordinary source/position queries, directed/absolute/zero/large windows and exact barrier form; compare literal pair rows/bytes/evidence, visits show one source scan and one ordered sweep, output multiplicity/errors/order unchanged. |
| B1 | Typed one-pass bucketed series | Ordinary day/month GROUP queries over actual typed data, every aggregate family and order-sensitive numeric/text/custom controls; exact dateTimeStamp/zone/calendar invalid-neighbor rows and literal output bytes/evidence; one ordered pass and preparation reported. |
| P1 | All real consumers, native storage/cost/error laws | Direct/prepared/parameterized, all query forms, scoped/interned, governed/fallible/paged, EXPLAIN, UPDATE WHERE and correlation/forks run actual selected operators; source failures before/during/final checkpoint, retained leases/clone/drop and #517 cost versus actual work remain correct. |
| P2 | Real portable consumer | Original wasm-bindgen runner executes public dictionary/window/bucket/host, random frozen differential summaries, selection, evidence and typed refusal; native/WASM deterministic bytes. All release crates still build WASM. |
| P3 | Complete settled validation/publication | Affected all-target strict lint + native/portable family, measured scaling benches, metadata/docs/translation checks if changed, unchanged mandatory make check and normal signed hooks pass; original failures preserved, current-head hosted review/candidate/debt qualifies before root ghprsq. |

## Tasks and ordered implementation

### Task 1 — complete shape runtime and all kernels/consumers

Implement this WHOLE behavior unit before the first consolidated compiler/runtime family:

1. Neutral provider/offer/exact contract and contained host boundary; owner-preserving native row runtime, plan side table, registry identity/cache and central installation/dispatch/EXPLAIN.
2. Neutral occurrence/dictionary capability and text phrase literal adapter; original compact token stream retention/projection and one automaton/binding cache; correct aliased-domain frontier, complete schema/order/VALUES reconstruction.
3. Owner-preserving longest-first selector and exact containment-filter differentiation; actual query input and public consumer paths.
4. Ordered window/barrier and bucket modes at native numeric/row/aggregate homes; no alternate kernels.
5. Full source caller propagation, admission/error/cancellation/lifetime handling, all native fixtures/bench/portable targets, generator/layer/helper registrations at their actual homes.
6. Integrate landed #515 public evidence/scope carriers and actual production proofs. This remains REQUIRED in Task 1; no partial success if the public seam is absent. Root's concurrently owned 515 code is not mutated or copied.
7. Run one consolidated affected strict all-target compiler/lint family, then complete native semantic/allocation/production matrices and selected actual WASM family. Collect all concrete failures before a coherent correction; rerun only affected/invalidated checks. Measure dictionary compilation/reuse and scan counters plus report-only scaling benchmarks using existing testkit/alloc-probe; no per-edit test/eval ritual.
8. Root independent applied-contract judgment on the written full behavior and actual evidence. Resolve concrete findings proportionately without new panels. Normal signed hooks/commit/push cover this coherent source; record task evidence and original failures.

Relative effort: dictionary recognition/domain/order and cache ownership are the largest piece; provider/runtime/evidence integration is next; window and bucket reuse existing kernels but need exact order/proof tests. No invented duration estimate.

### Task 2 — final whole-group qualification and publication

Update validation.md from exact terminal receipts; assess actual current integration candidate once (no rebase/full rerun solely because behind). Run unchanged mandatory make check after the full source/fixtures are settled, using the admitted original toolchain and resource lane. Current AGENTS.md controls repeats; no fixed gate cap or exemption. Changed inputs or actual failures justify affected repairs/checks.

Complete required generated metadata/English/public API documentation and real translations if message changes touch catalogs; no source issue/task branding, raw W3C literals or handwritten generated projections. Reuse valid prior evidence only with explicit affected-source/candidate reasoning. Then normal signed hooked commit/push, complete two-issue PR body/closing references, Stage evidence/task/PR/squash notes and current-head hosted qualification. Root owns distinct final feedback/candidate judgment and sequential ghprsq merge/archive/cleanup. Neither issue is counted closed before actual merge and verified closure.

## Qualification/resource plan and current status

No source changes, builds, tests or qualification were run during this planning. Root's #515 source is uncommitted and NOT imported. All implementation/acceptance rows above are currently NOT MET; plan review may PASS without runtime evidence.

Use the established pinned Stage rustup toolchain, private target/build/tmp under /opt/purrdf-shapes-dictionary-qualification, CARGO_BUILD_JOBS=4, RUST_TEST_THREADS=4, systemd user scope MemoryMax=16G and MemorySwapMax=0. Coordinate actual host headroom/root lane; max three capped compiler lanes overall, no unrelated model or host campaigns. Do not repurpose system HOME variables or bypass wrappers/hooks.

Planned settled families:
- cargo clippy --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --all-targets -- -D warnings (include additional touched crates if the native carrier/lower home changes);
- cargo test --locked -p purrdf-sparql-eval -p purrdf-text -p purrdf-retrieval --lib --tests --no-fail-fast with the whole written source frozen through terminal;
- actual shared native/wasm harness targets for dictionary/window/bucket/host/evidence using scripts/wasm-test-runner.sh, with original semantic assertions and source identities;
- report-only testkit benches for dictionary cardinality/token volume/cache hit/miss/aliases, window input/output widths and typed series widths; record scan/transition/output/allocation counters alongside time;
- required metadata/hygiene/docs/catalog checks for actual changed inputs, then untouched make check and all32 release-WASM paths.

The exact final commands/exit statuses belong in validation.md. No timing/green/runtime/closure claim is made by this plan.



