# Final review: plan Addendum A (PurRDF #570)

Single pass, per Patrick's instruction.

- **Plan:** `.stage/submit-w3c-shacl-implementation-reports/plan.md`, "Addendum A" (lines 813–963): U7–U9, A-G1..A-G3, Tasks 13–22, A-R1..A-R16.
- **Sources:** `reviews/gap-analysis-stage3.md` (G-01..G-16), `reviews/completion-audit-stage3.md` (F1–F8, A1–A3).
- **Code read for feasibility:** `crates/shapes/src/profile.rs`; `crates/validate/src/complete.rs`; `crates/shapes/src/engine.rs:1701-1760` (`ValidationOptions`, `with_profile`); `crates/shapes/src/constraints.rs` (`walk_shape` 1203-1310, `AnyViolation` 637-672, `conforms_memoized` 934-963, `deactivated_at` 1559-1608, `Not`/`And`/`Or`/`Xone`/`Node` arms 3042-3330); every `WD_20260918` / `REC_20170720` branch site in `crates/shapes/src` (`prebinding.rs:209-225`, `sparql.rs:1658-1668, 2451-2454`, `report.rs:1125`, `constraints.rs:3527-3531`, `admission.rs:214`, `complete_reports.rs:93-269`); `shacl_corpora/mod.rs:243-249`, `shacl12.rs:525-531`; `w3c12_conformance.rs:276, 353, 587-592`; `w3c_conformance.rs:112-115`; `tests/shape_deactivation_expression.rs:419-431`; host builders `crates/cli/src/validate.rs:398-410, 486-499`, `bindings/python/src/shacl.rs:131-170`, `crates/rdf-wasm/src/shacl.rs:268-299`, `crates/rdf-capi/src/shacl.rs:320-379`; `crates/cli/Cargo.toml:69`.

## Coverage: G-01..G-16 to tasks and checks

| Finding | Task | Check row | Falsifiable on the real entry point? |
|---|---|---|---|
| G-01 | 14 | A-R1 | CLI lane 213/213 under `--shacl-profile`: yes. Host neighbours: **no as written (B2)**. |
| G-02 | 13 | A-R2 | Yes (591/591 under ED; WD refusal + neighbour; REC prefix neighbour; grep). |
| G-03 | 15 | A-R3 | Yes. |
| G-04 | 16 | A-R4 | Direct members only; **does not test what A-G3 promises (B1)**. |
| G-05 | 17 | A-R5 | Yes (`?value "false"` probe; fuel trip fails). |
| G-06 | 16 | A-R6 | Yes. |
| G-07 | 18 | A-R7 | Yes (either branch is an exact assertion). |
| G-08 | 16 | A-R8 | Yes. |
| G-09 | 19 | A-R9 | Yes. |
| G-10 | 19 | A-R10 | Yes. |
| G-11 | 20 | A-R11 | Yes. |
| G-12 | 21 | A-R12 | Yes (rebuild observed; recorded in `validation.md`). |
| G-13 | 13, 21 | A-R13 | Yes. |
| G-14 | 18 | A-R14 | Yes. |
| G-15 | 21 | A-R15 | Yes. |
| G-16 | 21 | A-R16 | Yes. |

All sixteen findings have a task. G-01, G-02, G-05 and G-11, which the gap analysis offered as NEW-ISSUE, are fixed in #570 (U7, U8, U9). G-14's conditional NEW-ISSUE is resolved inside Task 18. No finding is deferred, filed, or moved to a local log.

## U7–U9 fidelity

- **U7** ("Fix in #570", G-01): A-G2 adds selection to all four hosts, a CLI lane, and the description sentence. No host is dropped. Faithful.
- **U8** ("Add the ED law in #570", G-02): A-G1 adds `shacl-ed-2026-10-16`, grades the suite under it, restores WD fidelity with neighbours, and gates the REC prefix path. Faithful.
- **U9** ("Fix all in #570", G-05 and G-11): Tasks 17 and 20. Faithful.

## Findings

### BLOCKING-1. A-G3 as written does not remove the order dependence it names, and A-R4 cannot detect that

Plan line 859–864: "A shape's `sh:deactivated` is evaluated before any short-circuiting caller (`sh:or`, `sh:and`, `sh:xone`, `sh:not`, conformance probes) consults its conformance, so the verdict never depends on member order."

What the code does: `walk_shape` (`constraints.rs:1221`) already evaluates the member shape's own `sh:deactivated` first. The dependence G-04 found comes from two short-circuits *downstream* of that point:

1. `Or` (`:3267-3278`) breaks on the first conforming member and `And` (`:3244-3254`) on the first non-conforming one, so a later member is never walked.
2. Every probe uses `AnyViolation` (`:659-672`), which returns `Flow::Stop` on the first disallowed violation, so inside a walked member the remaining constraints, their reifier deactivations (`plan.constraint_deactivation`, `:1244`, `:1293`), its property shapes (per value node), and anything reached through `sh:node`/`sh:property`/nested `sh:or` are never evaluated.

Evaluating only the member's *own* `sh:deactivated` before the loop fixes G-04's literal probe and nothing else. Counter-example the plan's own rule would still fail: `ex:S sh:or ( ex:A ex:B )`, `ex:B sh:node ex:C`, `ex:C sh:deactivated [ sh:path ex:muted ]`. With `ex:A` first, `ex:C`'s expression is never evaluated (exit 0); reversed, it is (hard failure). Same for a reifier-deactivated constraint on `ex:B` listed after a violating one. A-R4 tests only direct members in both orders, so an implementation of the sentence as written passes A-R4 while the finding stands.

**REWRITE TO (A-G3):**

> **A-G3. Deactivation evaluation order (G-04).** The text's SHOULD-fail is kept, and the set of `sh:deactivated` node expressions a validation evaluates is made independent of list order and of short-circuiting. Rule: at lowering, each shape plan records whether its reachable closure (through `sh:or`, `sh:and`, `sh:xone`, `sh:not`, `sh:node`, `sh:property`, `sh:qualifiedValueShape`, `sh:filterShape`, `shnex:conformsToShape`, cycle-safe) contains any node-expression `sh:deactivated` on a shape, a constraint reifier or a `sh:property` reifier. Within such a closure, `sh:or` and `sh:and` evaluate every member before combining, and the conformance probe (`AnyViolation`) records the first disallowed violation but returns `Flow::Continue`, so every constraint, property shape and nested shape the data reaches is walked exactly as a full validation walks it. Outside such a closure, evaluation is unchanged (no cost for graphs without deactivation expressions). The memo (`conforms_memoized`) keeps repeated probes cheap. The result: a failure is raised iff some reachable expression is ill-formed at a focus node, whatever the member order.

**REWRITE TO (A-R4):**

> `sh:or`, `sh:and`, `sh:xone` and `sh:not` give identical verdicts in both member orders when the expression sits (i) on the member shape, (ii) on a shape reached through the member's `sh:node`, (iii) on a property shape of the member, and (iv) on a constraint reifier of the member listed after a violating constraint. For each, the probe verdict equals the verdict of validating the member shape directly with `sh:targetNode`. A neighbour graph with the same structure and `sh:deactivated true`/`false` constants keeps its short-circuit (asserted by a count of member walks or a timing-free fuel observation).

Alternative, if the cost is unwanted: the gap analysis's option (a) ("anything but a single `true` is not deactivated, with a mandatory diagnostic") is order-independent by construction for the verdict. It changes the SHOULD-fail decision Patrick has not been asked about; see Open decisions.

### BLOCKING-2. Task 14's host neighbour check compares two laws that give the same answer on `pre-binding-009`

Plan line 881–883: "the Python, WASM and C neighbours select WD versus ED and get the different answers on `pre-binding-009`." A-R1 repeats "host neighbour tests pass."

`pre-binding-009` passes under the dated 1.2 law because of the Appendix A pre-binding rule (`prebinding.rs:219-222`, `Rules::AppendixA`), which the engine attributes to "SHACL 1.2 SPARQL Extensions of 18 September 2026" (`sparql.rs:1653-1657`). G-02 does not list it among the post-WD semantics; its post-WD list is deactivation on SPARQL constraints, the version-IRI prefix path, the blank-node argument rule, rule-set entailment, and the RL run-once clause. After Task 13, WD and ED therefore both pass `pre-binding-009`; the law that differs on it is `legacy` (G-01's probe: refused at load). As written the check cannot be satisfied, or would be "satisfied" by an ED that accidentally inherits nothing from WD.

**REWRITE TO (Task 14 check and A-R1):**

> Each of Python, WASM and C has a neighbour pair on the shipped validate entry: (1) `legacy` versus `shacl-ed-2026-10-16` on `pre-binding-009` (refused at load versus conforms, the two answers G-01 observed); (2) `shacl-wd-2026-09-18` versus `shacl-ed-2026-10-16` on `deactivated-select-001` (the WD refuses a node-expression `sh:deactivated` on a SPARQL constraint; the ED validates it). If the T13 audit moves the VALUES rule itself to the ED, swap (1)'s WD/ED pair in and record why in `tasks/T13-triage.md`. An unknown id (`shacl-ed-2026-10-17`) is refused with `unsupported-profile` on every host.

### ADVISORY-1. State that the ED law inherits every WD semantic, and name the law-census sites Task 13 must touch

A-G1 says what moves *to* the ED. It does not say the ED also carries everything the WD already does. Five engine sites select on `== WD_20260918` rather than "any 1.2 law": `prebinding.rs:219` (Appendix A), `sparql.rs:1662` and `:2453` (no shape-context pre-binding), `report.rs:1125` (message fallback), `constraints.rs:3529` (SPARQL constraint messages). An ED that misses any of them silently runs the compatibility rule there; the 591/591 check catches the pre-binding one, not necessarily the rest. The law-census arrays `admission.rs:214` (`DATED`) and the `[REC_20170720, WD_20260918]` loops in `complete_reports.rs:93, 156, 228` also need the ED or their coverage claims shrink.

**REWRITE TO (add to A-G1):**

> The ED law is the WD law plus the post-WD semantics: every site that today selects `WD_20260918` selects the ED too (one predicate, `ShaclProfile::is_shacl12()` or equivalent, replaces the five `== WD_20260918` tests), and only the post-WD semantics test for the ED alone. Task 13 re-points to the ED: `shacl12.rs:530` (`tc.profile`), `w3c12_conformance.rs:353` (rule cases) and `:591` (`Suite.law`), and every existing test that asserts a post-WD behaviour under `WD_20260918` (`shape_deactivation_expression.rs:425`; the audit lists the rest), each gaining a WD refusal neighbour. The `DATED` array and the `complete_reports.rs` law loops include the ED. For G-13, `w3c_conformance.rs:112-115` sets the 120 W3C entries' `profile` to `REC_20170720` and `Suite.law` to `shacl-rec-2017-07-20`, so the 2017 `doap:description` names the Recommendation law. `core_specification`/`sparql_specification` for the ED return the commit-pinned document URLs (`https://github.com/w3c/data-shapes/blob/4cd469867a1cc6d2fff80e8a40723b26c695d67a/shacl12-core/index.html` and `.../shacl12-sparql/index.html`); `xpath_specification` stays XPath 3.1. The id's date is the commit date of `4cd46986` on `gh-pages`, recorded with its `git show -s --format=%cs` output in `validation.md`; if it is not 2026-10-16, the id follows the commit, not the plan.

### ADVISORY-2. Name the host surfaces, the `--xpath-regex` interaction, and the C ABI shape

A-G2 says "`purrdf validate --shacl-profile <id>` and the matching Python, WASM and C parameters" and "decoded once through `purrdf_validate::complete_profile`". Feasible: the CLI already depends on `purrdf-validate` (`Cargo.toml:69`), `ValidationOptions::with_profile` exists (`engine.rs:1743`), and each host builds options in one place (`validate.rs:401`, Python `shacl.rs:147`, wasm `shacl.rs:280`, C `shacl.rs:144`). Three things an implementer would otherwise guess:

- A dated law requires its XPath bundle (`resolve_xpath`). `--shacl-profile shacl-ed-2026-10-16 --xpath-regex xpath20` must surface the existing `XPathProfileConflict` as a usage error, with a test; without `--xpath-regex`, `Request::admit` installs XPath 3.1 with default limits, so the CLI's non-native arm (`validate.rs:487`) is correct as is.
- The C API keeps ABI by adding twins (`_xpath_regex`). Adding a profile means a new twin (for example `purrdf_shacl_validate_to_sarif_law(..., regex_profile, shacl_profile, out)`), a header entry and the C smoke test, not a changed signature.
- Which entries: at minimum the validate entry each host ships (`purrdf validate`, both whole-graph and `--changes`; Python `validate`; wasm `validateToSarif`; the C validate twin). Products restored through `--shapes-product` carry their own options; say whether `--shacl-profile` applies to them or is refused.

**REWRITE TO (add to A-G2):** the three bullets above, verbatim, plus "every host's unsupported-id refusal is `complete_profile`'s `unsupported-profile` status, tested once per host."

### ADVISORY-3. Dependencies and commit subjects

Tasks 1–11 each name their conventional-commit subject; Tasks 13–22 do not, and the ordering constraints are implicit. **REWRITE TO (Tasks header):** "Task 13 precedes 14 and 15 (both consume the ED id and `tc.profile`). Tasks 16–21 depend only on the base. Task 22 is last." Add a subject per task, for example `feat(shapes): add the dated SHACL editor's-draft law and restore WD fidelity` (13), `feat(cli,python,wasm,capi): select the SHACL law on every host` (14), `test(shapes): grade the product and import lanes under the case's law` (15), `fix(shapes): make deactivation evaluation order-independent and keep closed paths` (16), `fix(shapes): type node-expression evaluation failures apart from processor failures` (17), `fix(xsd,sparql-eval): spell SECONDS and computed decimals as documented` (18), `fix(shapes): resolve rule-set includes at load and bound entailment changes` (19), `feat(conformance-kit): one EARL assertion per test and subject` (20), `test: pin footprint, closed-path and 2017 refusal positions; cache keys; w3c-earl output check` (21).

### ADVISORY-4. Task 22 omits most of the completion audit's open items

Task 22 lists the SIMD table, F6, A1, the docs matrix, CI green (F1) and the Stage 3 gates. Not listed: F3/F4 (PR body wording and the `make check` statement), F5 (record the `gh-pages` recheck command and output in `validation.md`), F7 (`drafts/w3c-pr.md` names the 2017 suite commit `08adb377…`), F8 (`handoff.md` refreshed; Task 1 block marked superseded). The PR body also needs the Addendum's facts: the ED law and its id, host selection, the second `earl.nt` byte change (Task 20), and the 2017 report now naming the Recommendation law. **REWRITE TO (Task 22):** add "F3–F5, F7, F8 per `completion-audit-stage3.md`; update the PR body and the published plan comment with Addendum A; refresh `handoff.md` with A-R1..A-R16 as MET/OPEN."

### ADVISORY-5. Two doc sites the tasks imply but do not name

Task 15: `shacl_corpora/mod.rs:243-249` says the lanes "ignore it by design and always run the compatibility law"; that comment must change with the behaviour. Task 13: `w3c12_conformance.rs:276` ("The node-expression loader and evaluator take no profile") stays true and should say the EARL description's law sentence covers `sht:Validate` and rule records, or the description should scope its claim. Add both file:line references to the tasks.

## Integration check

- No contradiction between Addendum tasks and Tasks 1–12: the EARL description change (A-G2) extends R17/R18's exact-value test; the 2017 law change (A-R13) extends `check_descriptions`; G13's "earl.nt bytes change" statement needs the second change noted (ADVISORY-4).
- The completeness additions table has a row per finding; each new task has the shared structure by reference (line 868–869), with the gaps named in ADVISORY-3.
- No Task 0; Task 22 is the closing task. Dependencies are right in sequence but unstated (ADVISORY-3).
- No new deferrals found. Task 18's "T18 decides on evidence" is an in-PR decision between two exact outcomes, both pinned by A-R7/A-R14.

## Open user decisions

1. **A-G3 policy (only if the orchestrator wants it confirmed):** keep the SHOULD-fail with full closure evaluation (BLOCKING-1's rewrite; cost: no short-circuit in shapes closures that use deactivation expressions), or adopt the gap analysis's option (a) (not deactivated plus diagnostic; no failure). The addendum chose the former; the rewrite makes it implementable without a new decision.

No other item needs Patrick.

Initial verdict: BLOCKED.

---

## Recheck (scoped: the two blockers and advisories 1–4)

Plan re-read at `plan.md` lines 813–979 (Addendum A as integrated). Only the integrated text was examined; no other area is reopened.

| Prior finding | Status | Evidence in the current plan |
|---|---|---|
| BLOCKING-1 (A-G3 / A-R4) | **DISCHARGED** | A-G3 (line 865) is the full-closure rule verbatim: lowering-time closure flag over the nine edge kinds, cycle-safe; `sh:or`/`sh:and` evaluate every member inside such a closure; `AnyViolation` records and returns `Flow::Continue`; unchanged outside; memo noted; "failure iff some reachable expression is ill-formed, whatever the member order". A-R4 (line 966) lists positions (i)–(iv), the probe-equals-`sh:targetNode` check, and the constant-spelling neighbour that keeps its short-circuit. Line 866 records the choice over option (a) with its reason (keeps the T3 SHOULD-fail already shipped) and marks it Patrick-overridable. This is the design I proposed; it removes the dependence for nested shapes and for the intra-member constraint loop, which the original sentence did not. |
| BLOCKING-2 (Task 14 / A-R1) | **DISCHARGED** | Task 14 (line 892) and A-R1 (line 963) now pair `legacy` vs ED on `pre-binding-009` and WD vs ED on `deactivated-select-001`, with the audit-swap clause recorded in `tasks/T13-triage.md` and the unknown-id refusal on every host. Both pairs differ by design. The ordering header (line 871–872) makes Task 13's WD restoration precede Task 14, which check (2) depends on. |
| ADVISORY-1 (ED inherits WD; census sites) | LANDED | A-G1a (line 849), verbatim: `is_shacl12()` predicate for the five sites, re-pointed `shacl12.rs:530`, `w3c12_conformance.rs:353`/`:591`, `shape_deactivation_expression.rs:425`, `DATED` and `complete_reports.rs` loops, `w3c_conformance.rs:112-115` to `REC_20170720`/`shacl-rec-2017-07-20`, commit-pinned URLs, XPath 3.1, date derived from the commit and recorded. |
| ADVISORY-2 (host surfaces, `--xpath-regex`, C twin) | LANDED, one correction below | A-G2 bullets (lines 860–863): conflict surfaced as a usage error with a test; C twin `purrdf_shacl_validate_to_sarif_law` with header and smoke test; entries enumerated; unsupported-id status per host. |
| ADVISORY-3 (ordering, subjects) | LANDED | Lines 871–882: 13 precedes 14 and 15; 16–21 independent; 22 last; a subject per task. (Task 13's `feat(shapes)!:` breaking marker is the orchestrator's call; the WD restoration does tighten accepted shapes under `shacl-wd-2026-09-18`, so it is defensible.) |
| ADVISORY-4 (Task 22 contents) | LANDED | Lines 949–953: F3–F5, F7, F8; PR body and plan comment with the four Addendum facts; `handoff.md` with A-R1..A-R16; the two doc sites from ADVISORY-5 as well. |

### New, introduced by the integration (ADVISORY)

Line 862: "`--shapes-product` restores carry their own options: `--shacl-profile` with a product is refused unless it equals the product's law, tested."

A prepared product records no SHACL law. `admit_bound` (`crates/shapes/src/product/mod.rs:1293-1321`) takes the caller's `ValidationOptions` and re-derives the source occurrences under `options.shacl_profile` at restore time; `restore_shapes_product_with_options` (`crates/validate/src/complete.rs:181-186`) passes those options through. "The product's law" therefore names a field that does not exist, and an implementer would look for it. The product's stored `profile` (`cli/src/validate.rs:272`) is the XPath regex law, which `resolve_xpath` already checks.

**REWRITE TO (line 862, last sentence):** "`--shapes-product` restores take the law from the request: `--shacl-profile` is passed in the `ValidationOptions` given to `restore_shapes_product_with_options`, so a restored product validates under the selected law exactly as a parsed shapes graph does, tested with one product under `legacy` and under the ED on `pre-binding-009`. The product's stored XPath profile and `--xpath-regex`/`--shacl-profile` compatibility are checked by the existing `resolve_xpath` call in `admit_bound`."

This does not block: the mechanism exists and the sentence is a one-line correction. It is noted so the implementer does not invent a stored law to satisfy it.

### Open user decisions after recheck

None required. The A-G3 override remains available to Patrick as line 866 states.

VERDICT: PASS
