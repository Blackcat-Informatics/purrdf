# Gap analysis, Stage 3 (adversarial): PurRDF #570, PR #599

## Scope and method

- Branch `paudley/570-submit-w3c-shacl-implementation-reports`, head `114b98530`; base `origin/main` `c75cbf947` (merged at `0e4d164bc`).
- Focus: correctness and robustness of what was built. Requirement coverage and honest-claims auditing are the separate completion audit (`reviews/completion-audit-stage3.md`); where a finding overlaps one of its items it is cited, not repeated.
- Specs read from `/home/paudley/Active/w3c/data-shapes` at `4cd46986` (suite commit) and, for the dated-law question, at `32318223` (the gh-pages state on the WD 2026-09-18 publication date).
- Probes run: the built CLI (`target/debug/purrdf`, rebuilt with `cargo build --locked -p purrdf-cli` at head) on throwaway Turtle under `/tmp/gap570/`; one scratch integration test (`crates/shapes/tests/zz_gap_probe.rs`, copied to `/tmp/gap570/scratch/zz_gap_probe.final.rs`, then deleted; `git status --porcelain` empty afterwards); the community runner (`target/debug/community-conformance corpora/community /tmp/gap570/comm-out`); a uv cache-keys experiment in a scratch project (`/tmp/gap570/uvprobe`, uv 0.11.28, offline).
- Not run: `make check`, `make conformance`, any model or GPU path.

## Findings

### G-01 (BLOCKING). The SHACL 1.2 EARL claims hold only under a law no shipped host can select, and the report does not say which law

- **Evidence.**
  - Probe (scratch test): every in-EARL-scope 1.2 `sht:Validate` entry graded through `report_grading::run_validate_case` with `tc.profile = ShaclProfile::LEGACY` (the default of every host): `total=213 failed=3` — `sparql/node/sparql-failure-001` ("suite expects sht:Failure but the engine validated successfully"), `sparql/node/sparql-message-001` (message "Message from the constraint", expected "Message from the query"), `sparql/pre-binding/pre-binding-009` (refused at load).
  - CLI, same three files (`purrdf validate --shapes F F`): `pre-binding-009` → `purrdf: ... VALUES is not allowed in a query with pre-bound variables ... PurRDF refuses every VALUES ...`; `sparql-failure-001` → `shacl conforms false / shacl results 1` (expected a failure); `sparql-message-001` → `sh:resultMessage "Message from the constraint"`.
  - No host exposes the law: `grep -rn "with_profile\|ShaclProfile" crates/cli/src bindings/python/src crates/rdf-wasm/src crates/rdf-capi/src` returns nothing; `purrdf validate --help` has no profile option. `purrdf_validate::complete_profile` (`crates/validate/src/complete.rs:116`) has no host caller.
  - `crates/shapes/tests/support/w3c_earl.rs:136-143`: `doap:description` names only the source commit and suite commit. `crates/shapes/tests/w3c12_conformance.rs:587-591` records `profile: "shacl-1.2"`, which is not emitted and is not the law id either.
- **Violates.** `AGENTS.md` §1 ("one engine, one behavior, carried verbatim into Rust, Python, WebAssembly, and C"); `CLAUDE.md` "a refusal is a claim" (the default law refuses an approved test the report says passes). U1 requires the report to be submittable; a WG reader reproducing the 382/382 with `purrdf validate` or the Python/wasm/C bindings gets 379/382. Overlaps completion audit A1, which rated it advisory; it is rated blocking here because the subject `doap:name "PurRDF <version>"` names the shipped software, not a Rust-API configuration.
- **Recommendation.** FIX-NOW: state the law and surface in the 1.2 `doap:description` (for example "validated through the Rust library under ValidationOptions profile shacl-wd-2026-09-18; the default compatibility law differs on three entries"), assert it in `w3c12_earl_subject_and_assertor_are_exact`, and say the same in `drafts/w3c-pr.md`. NEW-ISSUE for the host gap:
  - Title: `Expose the dated SHACL law on every host (CLI, Python, wasm, C)`
  - Body: The library grades the W3C SHACL 1.2 suite under `shacl-wd-2026-09-18`, but `purrdf validate` and the Python, wasm and C bindings can only run the compatibility law, under which `sparql-failure-001`, `sparql-message-001` and `pre-binding-009` fail. Add a `--shacl-profile <id>` option and the matching binding parameters, decoded once through `purrdf_validate::complete_profile`, with a host-level lane that runs the 1.2 `sht:Validate` entries under the dated law. Neighbour: the compatibility law keeps its current behaviour by default.

### G-02 (MAJOR). The "dated" WD 2026-09-18 law implements Editor's Draft text that postdates and contradicts that WD; the docs say the tests are written against the WD

- **Evidence.**
  - `crates/shapes/src/profile.rs:42-43,80-87`: `WD_20260918` is "SHACL Core WD 2026-09-17 and SPARQL WD 2026-09-18", `sparql_specification()` = `.../WD-shacl12-sparql-20260918/`. Its own doc: "no unknown date is mapped to a nearby edition" (`:26-28`).
  - WD-era text (`git show 32318223:shacl12-sparql/index.html`, tags stripped) vs suite head `4cd46986`:
    - SPARQL-based constraint `sh:deactivated`: WD says "this value is either true or false"; head says "is a node expression ... as for shapes". The engine accepts a node expression on a SPARQL constraint under `WD_20260918` (`crates/shapes/src/shapes/parser/node_expr.rs:552`; `shape_deactivation_expression.rs:425` asserts the 1.2 law accepts it).
    - Prefix path `(owl:imports/(^owl:versionIRI)?)*`: 0 hits at WD, 1 at head (added `07458ebb`, 2026-10-10).
    - "Note that blank nodes cannot be passed as arguments": 0 at WD, 1 at head (`07458ebb`).
    - "Entailment using Rule Sets": 0 at WD, 1 at head (`2ada6ac4`, 2026-10-12).
    - SPARQL 1.2 RL run-once triple-term clause: WD-era text lists only assignments and head blank nodes.
  - The approved tests these fixes target were all added after the WD: `rules-entailment-ruleset-*` 2026-10-12, `closed-001` 2026-10-15, `blankNodeArgument-001`, `prefixes-003`, `deactivated-select-001`, `pre-binding-009` 2026-10-16 (`git log --diff-filter=A`).
  - `docs/CONFORMANCE.md:690`: "The sht:Validate entries run under the dated shacl-wd-2026-09-18 profile, the law their text is written against." This is false.
  - Law leak: `crates/shapes/src/shapes/prefixes.rs:226` (`reached_holders`) takes no profile, so the 2026-10-10 path also applies under `REC_20170720`. The 2017 Recommendation's path is `sh:prefixes/owl:imports*/sh:declare`, so the 2017 law now resolves prefixes the 2017 text does not reach.
  - `tasks/T3-triage.md` and `T5-triage.md` attribute these sentences to "WD 2026-09-18".
- **Violates.** PurRDF's dated-law contract (`profile.rs` docs: an exact edition, no nearby-edition mapping); SHACL 1.2 SPARQL WD 2026-09-18 "Syntax of SPARQL-based Constraints" (boolean only); SHACL REC 2017-07-20 §5.2.1 prefix path.
- **Recommendation.** FIX-NOW: correct `docs/CONFORMANCE.md:690`, the book passage and the code comments that cite the WD for post-WD text; gate `reached_holders`' version-IRI steps off under `REC_20170720` (with a neighbour test); amend the T3/T5 triage attributions and the WG findings draft. NEW-ISSUE:
  - Title: `Add an exactly dated SHACL law for the editor's drafts the current W3C suite tests`
  - Body: `shacl-wd-2026-09-18` now includes semantics first published in the 2026-10-10 and 2026-10-12 editor's drafts: SPARQL-constraint node-expression `sh:deactivated`, the version-IRI prefix path, blank-node argument errors, and rule sets as `sh:entailment` values. The WD itself says a SPARQL constraint's `sh:deactivated` "is either true or false". Introduce a law id for the suite's spec state (or the next WD), move the post-WD semantics under it, and refuse them under `WD_20260918`, each with a neighbour. Re-grade the 1.2 harness under the new id.

### G-03 (MAJOR). The prepared-product and vocabulary-import lanes certify the 1.2 suite under the compatibility law, grew refusal ledgers for a case the claimed law passes, and document that refusal falsely

- **Evidence.**
  - `crates/shapes/src/shacl_corpora/mod.rs:243-249`: these lanes "ignore it by design and always run the compatibility law".
  - `crates/shapes/tests/product_corpus_equivalence.rs:166-170`: `W3C12_REFUSED_AT_LOAD` gains `pre-binding-009`. `crates/shapes/tests/vocabulary_import_invariance.rs:77`: `REFUSED_AT_LOAD` gains the same entry, with the reason "SHACL 1.2 features the engine does not evaluate".
  - `docs/CONFORMANCE.md:129`: "1 SHACL 1.2 entry whose shapes graph the engine refuses at load (SHACL 1.2 features it does not evaluate)". Two rows above, the same file says that entry passes (591/591).
  - `crates/shapes/src/prebinding.rs:299`: the compatibility-law refusal still says "because the frozen W3C pre-binding corpus requires the stricter reading". The re-vendored corpus now requires the opposite (`pre-binding-009`).
  - The same lanes therefore never pack, restore or import-merge any 1.2 shapes graph under the law the EARL claims. That also hides `sparql-failure-001` and `sparql-message-001`, whose answers differ by law (G-01).
- **Violates.** `AGENTS.md` §2 (exact counts; XPASS discipline is meaningful only under the graded law); `CLAUDE.md` "a refusal is a claim" (a ledgered refusal rationale that is false).
- **Recommendation.** FIX-NOW: run both lanes under `tc.profile` (the field already exists, `report_grading::load_shapes` already honours it), delete both `REFUSED_AT_LOAD` entries, regenerate the row in `docs/CONFORMANCE.md`, and reword `prebinding.rs:299` to name the 2017 Recommendation's total VALUES ban as the reason.

### G-04 (MAJOR). The SHOULD-fail rule for a non-boolean `sh:deactivated` makes the verdict depend on `sh:or` / `sh:and` member order and on short-circuiting

- **Evidence.**
  - `crates/shapes/src/constraints.rs:1559-1607` (`deactivated_at`) hard-fails when the expression yields 0, 2+ or a non-boolean node. It is only reached when evaluation gets there: `walk_shape` calls it at `:1221/:1244/:1293`, and conformance probes stop at the first violation (`AnyViolation`, `:1525`).
  - Probe. Data: `ex:a a ex:T` (no `ex:muted`). Shapes: `ex:S sh:targetClass ex:T ; sh:or ( ex:A ex:B )`, where `ex:A sh:nodeKind sh:IRI` and `ex:B sh:datatype xsd:integer ; sh:deactivated [ sh:path ex:muted ]`.
    - `sh:or ( ex:A ex:B )`: exit 0, `shacl conforms true`.
    - `sh:or ( ex:B ex:A )`: exit 1, `purrdf: sh:deactivated of shape <ex:B> produced 0 output nodes at <ex:a>`.
    - `sh:xone`: fails. `sh:not ex:B`: fails.
  - Common authoring (`sh:deactivated [ sh:path ex:flag ]` where most nodes lack the flag) fails whole validations. `[ shnex:exists ... ]` is the only robust spelling, and no test or doc says so.
- **Violates.** SHACL 1.2 Core, "Deactivating Shapes and Constraints": only "produces true as its only output node" defines deactivation. `sh:or`/`sh:xone` are defined over the members' conformance, independent of list order; the engine's chosen failure makes order observable. `AGENTS.md` §1 "deterministic, boring".
- **Recommendation.** FIX-NOW, with a neighbour pair for each order. Pick one deterministic policy:
  - (a) Follow the evaluation sentence literally: anything but a single `true` is "not deactivated", reported through the existing mandatory-diagnostic channel.
  - (b) Keep the failure but make it independent of evaluation order: evaluate the shape's deactivation before any short-circuiting caller consults it.
  - Record the question in the WG findings draft (T3 clarification 2 already raises it).

### G-05 (MAJOR). An evaluation failure inside a custom function body still fails the whole validation; the specification makes it the call's SPARQL error

- **Evidence.**
  - `crates/shapes/src/shapes/parser/functions.rs:712`: `eval_custom_function_call(...).map_err(|e| EvalError::function(...))?` turns every body failure into a host-function failure.
  - Probe: `ex:twoNodes a sh:ListParameterExpressionFunction ; sh:bodyExpression [ sparql:abs ( [ sh:select "SELECT ?v WHERE { ?x ex:n ?v }" ] ) ]`, with data `ex:a ex:n 1 . ex:b ex:n 2`, called from `BIND (ex:twoNodes() AS ?r) BIND (str(bound(?r)) AS ?value)`. Result: `purrdf: ... native-sparql-function-operational: host function error: custom SPARQL function: <sparql:abs> argument 0 produced 2 nodes`. The expected result is a report with `?value "false"`.
  - `tasks/T3-triage.md` ("Not distinguished") records this as a deliberate deferral, because processor failures and evaluation failures share one channel.
- **Violates.** SHACL 1.2 SPARQL, "Evaluation of Custom SPARQL Functions": "if the evaluation produces an evaluation failure, the result of the SPARQL function call is an error". Also `.goals`/`CLAUDE.md` NO DEFERRALS.
- **Recommendation.** NEW-ISSUE:
  - Title: `Type node-expression evaluation failures apart from processor failures so custom SPARQL calls return an error`
  - Body: `invoke_expression_function` maps every body failure to a host-function error, which aborts validation. The SPARQL Extensions text makes an evaluation failure inside the body "an error" of the call, so a BIND leaves its variable unbound. Split `ShapesError`/`EvalError` into an evaluation-failure kind (arity, list-argument cardinality, type errors) and a processor-failure kind (governor trip, recursion guard, resource exhaustion). Map only the first to `Ok(None)`. Add the neighbour pair: a two-node argument is unbound, while a fuel trip still fails.

### G-06 (MAJOR). A constant reifier `sh:deactivated true` on a `sh:property` statement removes that path from `sh:closed`'s permitted set; the always-true expression does not

- **Evidence.**
  - `crates/shapes/src/shapes.rs:3628` (`annotate_property_edge`) returns `None` for `Annotated::Deactivated`, so the property shape leaves `shape.property_shapes`. The closed lowering builds P from those siblings (`crates/shapes/src/plan.rs:2746-2766`).
  - Probe. Data `ex:a ex:p 1`. Shapes `ex:S sh:targetNode ex:a ; sh:closed true ; sh:property ex:P {| sh:deactivated V |} . ex:P sh:path ex:p ; sh:minCount 5`.
    - `V = true`: `shacl results 1`, `sh:ClosedConstraintComponent` on `ex:p`.
    - `V = [ shnex:if true ; shnex:then true ; shnex:else false ]`: `shacl results 0`.
    - `ex:P`'s own `sh:deactivated true`: `shacl results 0`.
- **Violates.** SHACL 1.2 Core §7.9.1: "P is the set of IRI properties that can be reached from the current shape via the SPARQL path sh:property/sh:path". The path is syntactic; a reifier deactivates "the constraints that use the triple", and the `sh:property` triple is not a parameter of the closed constraint. The constant case reports a spurious violation, and the two semantically equal spellings disagree.
- **Recommendation.** FIX-NOW: keep a reifier-deactivated edge's simple path in the closed sibling set (or carry the edge with `deactivated: true` instead of dropping it). Test node-shape and property-shape closed under both spellings.

### G-07 (MAJOR). `SECONDS` does not do what its stated rule says, and the change diverges term-for-term from the SPARQL WG's own approved test

- **Evidence.**
  - Probe `purrdf query`: `SECONDS("2023-12-25T10:30:05.500"^^xsd:dateTime)` returns `"05.5"`; `SECONDS("10:30:00.250"^^xsd:time)` returns `"00.25"`. The "seconds part of the lexical form" is `05.500` / `00.250`.
  - `crates/xsd/src/numeric.rs:1171` re-renders the parsed value with a padded integer part; it does not read the lexical form.
  - `docs/book/src/sparql/querying.md` and `tasks/T7-triage.md` item 4 state the lexical-field rule.
  - `crates/sparql-conformance/suite/w3c-sparql11/functions/seconds-01.srx` expects `"1"`, `"2"`, `"0"`, `"3"`^^xsd:decimal for seconds fields `01`, `02`, `00`, `03`. PurRDF now emits `01`, `02`, `00`, `03` and passes only because that harness compares numbers by value.
  - `fn:seconds-from-dateTime` (which the SPARQL 1.1 text says SECONDS corresponds to) returns the value `0`, canonically `"0"`.
  - The T7 triage never cites `seconds-01`. No term-exact rule satisfies both suites, a fact the WG needs.
- **Violates.** The engine's own documented rule. CLAUDE "a refusal is a claim" applies equally to a stated derivation.
- **Recommendation.** FIX-NOW: either extract the seconds field from the argument literal's lexical form when it is a literal (keeping `05.500`), or restate the rule exactly ("the seconds value with at least two integer digits"). Add the `seconds-01` vs `seconds-example` conflict to `drafts/wg-findings.md`.

### G-08 (MINOR). "At most one reifier with a value for sh:deactivated" is not enforced once a value is a node expression

- **Evidence.**
  - `crates/shapes/src/shapes/parser/annotations.rs:289-311` collects every expression-valued reifier. Booleans are only checked for disagreement (`:385`).
  - Probe: two reifiers `ex:r1`, `ex:r2` with `rdf:reifies <<( ex:S sh:datatype xsd:integer )>>`, one stating `sh:deactivated [ sh:path ex:m1 ]` and the other `[ sh:path ex:m2 ]` (also tried `false` with an expression). Both load and validate (`shacl results 1`, OR-ed).
  - Neighbours are refused correctly: two values on a shape, on a SPARQL constraint node, and on an `sh:expression` node all give "more than one distinct value for sh:deactivated".
- **Violates.** SHACL 1.2 Core: "A triple that has a shape as subject, a parameter ... as predicate can have at most one reifier with a value for the property sh:deactivated"; ill-formed graphs SHOULD fail.
- **Recommendation.** FIX-NOW: refuse a second reifier stating `sh:deactivated` for one statement whenever either value is an expression. Pair the test with the single-reifier neighbour.

### G-09 (MINOR). Rule-set entailment validates rule-set references inconsistently

- **Evidence.**
  - Probe. `ex:RS a sh:RuleSet ; sh:hasRule ex:R ; sh:includesRuleSet ex:Missing`:
    - with `sh:entailment ex:RS`: the shapes graph parses, then validation fails with `rule set <ex:Missing> is not declared` (detected at execution, `crates/shapes/src/rules.rs:1134-1152`);
    - with `sh:entailment ex:RS , sh:RulesEntailment`: `shacl conforms true`. The same defect is silently ignored, because `engine.rs:3764` takes the default rule set and never resolves includes.
  - `ex:RS sh:hasRule ex:R` with no type is a rule set to `parse_rule_sets` (`rule_parse.rs:246-250`, and so to `RuleOptions::with_rule_set`), but `parse_entailment` (`:157-185`) refuses it as an unsupported regime.
- **Violates.** The engine's own chosen rule (T5 clarification 3); one shapes graph reads two ways.
- **Recommendation.** FIX-NOW: resolve every declared rule set's include closure at load (`parse_rule_graph`), refusing an undeclared include regardless of which regimes are named. Apply one definition of "rule set" (typed, per the text) to both readers, each with a neighbour.

### G-10 (MINOR). `validate_change` on any shapes graph that declares `sh:entailment` fails with a false diagnosis

- **Evidence.**
  - `crates/shapes/src/engine.rs:2420` replaces the bound data with the entailed dataset. `affected_focus_raw` (`:2984-2988`) then finds no delta source.
  - Probe (scratch test): `sh:entailment ex:RS`, a `sh:TripleRule`, bound with `bind_delta_with_shapes_graph`. Both `affected_focus_node_ids` and `validate_change` returned `this validator is not bound to a mutation snapshot ... bind through PreparedShapes::bind_delta_with_shapes_graph`, though it was. Full `validate()` on the same binding works.
  - This branch widened the set of graphs that reach this path: rule-set entailment used to be refused at load.
- **Violates.** Fail-closed is correct, but the message names the wrong cause; `AGENTS.md` typed, actionable diagnostics.
- **Recommendation.** FIX-NOW: when the shapes graph declares any entailment, answer `FocusExpansion::Everything { reason: "rules entailment" }`, which is the module's existing opaque-construct route. Test it beside a non-entailing neighbour that stays bounded.

### G-11 (MINOR). `Report`/`to_earl` render several records about one test as indistinguishable assertions

- **Evidence.**
  - `crates/conformance-kit/src/outcome.rs:263-278` sorts but never rejects duplicate `case` IRIs; profile and surface are not emitted.
  - Probe: two records for `http://example.org/t1` (profiles p1 passed, p2 failed) render two assertions about one `earl:test`. The scoreboard says `passed 1, failed 1 of 2`.
  - Community runner output: 535 assertions over 64 distinct tests, 58 tests with more than one assertion. The WG page's `loadReport` (`reports.js`) keeps whichever it meets last.
  - The W3C harnesses are unaffected, because `w3c_earl::assertions` refuses duplicates on read-back.
- **Violates.** EARL 1.0 assertions are per subject and test; contradictory assertions for one subject and test have no defined reading. G11's "one report structure" claim for other harnesses.
- **Recommendation.** NEW-ISSUE:
  - Title: `EARL: one assertion per test and subject, or distinguish profile and surface`
  - Body: `outcome::Report` accepts several records with the same case IRI and `to_earl` renders them as indistinguishable assertions. The community runner already emits 535 assertions for 64 tests. Either refuse duplicate case IRIs in `Report::new` (one test per record), or make profile and surface part of the subject or test identity. Add a kit test with a passing and a failing record for one case.

### G-12 (MINOR). The new uv cache keys miss compile inputs this very branch changed

- **Evidence.**
  - `bindings/python/pyproject.toml:79-85` keys on `.rs` files, the root manifest and the lockfile.
  - `crates/shapes/src/lint.rs:110,786` `include_str!` `crates/shapes/spec/shacl-shacl.ttl` and `shacl.ttl`, both re-vendored on this branch with no `.rs` change. Also missing: `bindings/python/Cargo.toml`, the 46 `crates/*/Cargo.toml`, `bindings/python/rust-toolchain.toml`.
  - Scratch uv project (uv 0.11.28): `../` globs do trigger a rebuild on change and a no-op sync does not, so the mechanism works and the gap is the key list. Walk cost is small (2,262 tracked `.rs` files).
- **Violates.** The commit's own claim ("rebuild the extension when the workspace's Rust sources change"). A stale extension makes Python tests grade old behaviour.
- **Recommendation.** FIX-NOW: add `{ file = "Cargo.toml" }`, `{ file = "rust-toolchain.toml" }`, `{ file = "../../crates/**/Cargo.toml" }` and `{ file = "../../crates/*/spec/**" }` (or every `include_str!` target). Python line growth is in `pyproject.toml`, which is not under the ratchet.

### G-13 (MINOR). Behaviour that is sound today is unpinned

- **Evidence.** Scratch differential (same oracle as `change_footprint.rs`): the incremental footprint is a correct superset in these cases, but `change_footprint.rs` has none of them (`git diff` shows no change to it):
  - property-shape `sh:closed`;
  - `sh:deactivated [ sh:path ( ex:owner ex:muted ) ]` on a shape, a property shape and a `sh:property` reifier;
  - a deactivated shape reached through `sh:node`.

  Other unpinned behaviour:
  - `closed_property_shape.rs` has no inverse, sequence or `zeroOrMore` path case (with `zeroOrMore` the focus node itself is a value node).
  - The 2017 refusal is tested on a shape, a SPARQL constraint and a reifier, but not on a property shape, an `sh:expression` node or a `sh:property` reifier (shared code, untested positions).
  - The 2017 harness grades under the compatibility law although its records say `shacl-20170720` (`w3c_conformance.rs:112`). A scratch run under `REC_20170720` gives `total=120 failed=0`, so pinning it costs nothing.
- **Violates.** `AGENTS.md` §2 (conformance corpora are graded under the law claimed); footprint module's "superset law" is the contract for every construct.
- **Recommendation.** FIX-NOW: add the four footprint cases and the complex-path closed cases; set `profile = REC_20170720` for the 120 W3C 2017 entries; extend the 2017-refusal test to the three missing positions.

### G-14 (MINOR). Computed-decimal spelling has side inconsistencies

- **Evidence.**
  - `+(1.5+2.5)` returns `"4"` but `(1.5+2.5)` returns `"4.0"`. Stored `"4.0"^^xsd:decimal` under unary plus becomes `"4"` (`crates/sparql-eval/src/expr.rs:7814-7817`).
  - `crates/sparql-conformance/tests/data_r2_harness.rs:170` claims "Unary plus returns its operand's spelling"; it canonicalizes instead.
  - `xsd:string(1.5+2.5)` is `"4"` while `STR(...)` is `"4.0"` (XPath-correct, but undocumented).
  - `crates/xsd/src/numeric.rs:456-462`: `numeric_render_cost` prices "every byte of the text" from the canonical form, two bytes short for a wide computed decimal and one byte short for a padded seconds field.
  - T7's "one rule, applied wherever the engine computes a number" overclaims: `purrdf-text` scores (`crates/text/src/relation.rs:1821`) and retrieval scores (`crates/retrieval/src/matching.rs:1428`) are computed `xsd:decimal`s with their own 12-digit spelling.
- **Recommendation.** FIX-NOW: correct the test comment and the triage and book wording; make `numeric_render_cost` price the computed form (or document the bound as "within two bytes"). NEW-ISSUE only if unary plus should keep the operand's spelling.

### G-15 (MINOR). Documentation and pins have drifted

- **Evidence.**
  - `crates/shapes/tests/product_determinism.rs:84-87`: the history says "4,736 ... (+8)" but `GOLDEN_LEN = 4_744`. The undocumented +8 is the `entailment_rule_sets` count.
  - `crates/shapes/tests/shacl_shacl_differential.rs:123-129`: the `sparql-constraint-deactivated` rationale cites the non-normative "only valid values ... true and false", which the engine no longer enforces.
  - `crates/shapes/src/json_schema.rs:3517`: the loss for a SPARQL or expression constraint's node-expression deactivation is recorded against the blank constraint node (`_:gts_…`), not the shape the doc comment promises. Its test covers only the reifier route.
  - `vocabulary_import_invariance.rs:67` exempts `sparql-syntax-error-001` wholesale for a byte-offset difference, so any future answer change on that case is invisible.
- **Recommendation.** FIX-NOW: fix the four texts and pins; compare that case's errors with offsets normalized instead of exempting it.

### G-16 (MINOR). `make w3c-earl` reports success without checking its outputs

- **Evidence.** `Makefile:420-426` echoes "Wrote …" after `cargo test` passes. It never checks that both files exist, are non-empty and are newer than the run. In a reused `W3C_EARL_DIR`, stale files from an earlier HEAD would be announced as written. (The release lane is guarded later by `check-release-notes.py`'s missing or empty check; `release-tags` preflight is not.)
- **Recommendation.** FIX-NOW: remove both files before the run and `test -s` each after it, inside the same recipe.

## Checked and found sound (no finding)

- Node-expression deactivation is evaluated per focus node on every validation route (`walk_shape`, property shapes, reifier shapes, rule eligibility `rules.rs:1347`, expected derived triples `:1815-1828`). Target narrowing is disabled when it applies (`target_eval.rs:181-182,236-237`).
- Failure messages are deterministic across parallel chunks (20,000 focus nodes, 8 runs, always the first failing node).
- Self-recursive deactivation through `shnex:conformsToShape` follows the engine's existing coinductive recursion policy (`sh:not ex:S` self-reference behaves the same).
- A reifier on one `sh:property` edge does not leak into another parent's reach of the same property shape.
- `sh:closed` on reifier value nodes sees `rdf:reifies` and annotation triples (2 results).
- The prepared-product `STAGE_ID` moved with the format change, so old products are refused, not misread.
- The SPARQL-RL run-once classification matches the head text, including nested triple terms and annotation syntax. The aggregation element has no grammar production (`sparql-rl-grammar.bnf:15`).
- `to_earl` refuses an invalid IRI (`iri-disallowed-char`).
- uv `../` globs work in uv 0.11.28.
- CSV, TSV and CONSTRUCT outputs carry the computed spelling consistently.

## Summary

| ID | Severity | Area | Recommendation |
|---|---|---|---|
| G-01 | BLOCKING | 1.2 EARL claims hold only via Rust API under an unstated law; 3 approved entries fail on every host default | FIX-NOW (state law and surface) + NEW-ISSUE (host profile selection) |
| G-02 | MAJOR | `WD_20260918` implements post-WD ED semantics contradicting the WD; docs claim false; 2017 prefix-path leak | FIX-NOW (claims, REC gate) + NEW-ISSUE (dated ED law) |
| G-03 | MAJOR | Product and import lanes run 1.2 under LEGACY; new refusal ledgers with false rationale; stale LEGACY message | FIX-NOW |
| G-04 | MAJOR | Deactivation SHOULD-fail is order-dependent under `sh:or`/`sh:xone`/`sh:not` | FIX-NOW |
| G-05 | MAJOR | Custom-function body evaluation failure aborts validation instead of erroring the call | NEW-ISSUE |
| G-06 | MAJOR | Constant reifier deactivation of `sh:property` shrinks `sh:closed` P (spurious violation) | FIX-NOW |
| G-07 | MAJOR | `SECONDS` violates its stated lexical-field rule; term conflict with SPARQL `seconds-01` unrecorded | FIX-NOW |
| G-08 | MINOR | At-most-one reifier `sh:deactivated` not enforced for expressions | FIX-NOW |
| G-09 | MINOR | Rule-set reference checks inconsistent (runtime-only; masked by `sh:RulesEntailment`; untyped sets) | FIX-NOW |
| G-10 | MINOR | `validate_change` with `sh:entailment` fails with a false diagnosis | FIX-NOW |
| G-11 | MINOR | `Report`/`to_earl` emit indistinguishable duplicate assertions (community: 535 over 64 tests) | NEW-ISSUE |
| G-12 | MINOR | uv cache keys miss `include_str!` spec files, extension and member manifests | FIX-NOW |
| G-13 | MINOR | Sound but unpinned: footprint cases, complex-path closed, 2017 refusal positions, 2017 law | FIX-NOW |
| G-14 | MINOR | Decimal side inconsistencies (unary plus, render cost, overclaim) | FIX-NOW |
| G-15 | MINOR | Doc and pin drift (golden length history, rationale, loss subject, whole-case exemption) | FIX-NOW |
| G-16 | MINOR | `make w3c-earl` does not verify it wrote both files | FIX-NOW |

VERDICT: BLOCKED (G-01). Resolving G-01's FIX-NOW part unblocks the submission; G-02 to G-07 are correctness gaps in shipped behaviour that should be fixed or filed before merge.

## Note: concurrent uncommitted edits seen at the end of this review

When this file was written, `git status` showed uncommitted edits by another agent. This review did not make or touch them: `crates/shapes/tests/support/w3c_earl.rs`, `w3c12_conformance.rs`, `w3c_conformance.rs` and `scripts/publish-github-release.py`. They add a `law` field to `Suite` and the text "graded under the {law} profile" to `doap:description` (1.2: `shacl-wd-2026-09-18`; 2017: `legacy`), and they reword the publisher's asset-count message (completion audit A1/F6). Bearing on this review:

- They satisfy part of G-01's FIX-NOW. They do not disclose that every host's default law (`legacy`) fails three in-scope 1.2 entries, and per G-02 the name `shacl-wd-2026-09-18` does not describe the editor's-draft semantics actually graded.
- For the 2017 file, "legacy" is accurate today. A one-line switch of the 120 W3C entries to `REC_20170720` (probe: 120/120 pass) would let the report name the dated Recommendation law instead (G-13).
