# Issue #402: SHACL dated profile selection and complete validation reports (additive)

State: OPEN   Repo: Blackcat-Informatics/purrdf   Forge: github
Labels: enhancement, conformance

## Body

## Problem
SHACL validation has no explicit profile selection between SHACL 1.0 (REC 2017) and the SHACL 1.2 drafts. The report also cannot carry everything required:
- `sh:sourceConstraint`
- the normative message precedence
- blank-node identity of report contexts

`ValidationResult` is a public struct without `#[non_exhaustive]`, so these cannot be added as new fields. The SPARQL pre-binding admission policy (VALUES / MINUS / SERVICE / hidden pre-bound subquery variables) is one fixed policy instead of one per profile.

## Proposed solution
- Add a profile selector and a richer report door, both additive. The default profile and existing report types stay unchanged.
- Make the pre-binding admission rules depend on the selected profile, with required-rejection tests for constructs the profile forbids.
- Wire the SHACL cases of the community corpus into the community conformance runner, reporting unsupported profiles separately from failures.

## Acceptance
- Exact report comparison: graph isomorphism plus the required fields.
- Every refusal is paired with a valid neighbouring case that still validates.
- Python and wasm expectations stay unchanged under the default profile.
- `cargo semver-checks` passes for shapes and validate.


## Comments (12)

### paudley — 2026-10-05T05:54:05Z

# Complete implementation plan: dated SHACL validation laws and contextual reports

## 1. Issue summary and intake

Issue: https://github.com/Blackcat-Informatics/purrdf/issues/402 — **SHACL dated profile selection and complete validation reports (additive)**. Open, unassigned, no comments at intake; milestone v3.0.2. Full intake is `/tmp/purrdf-402-intake.json`.

The current engine has one legacy query-admission policy, closed public `ValidationResult` and `ValidationReport` structs, and reports that cannot expose the actual `sh:sparql` source constraint or enough source identity evidence for contextual report comparison. The deliverable is explicit dated SHACL validation law selection, an additive complete report API, profile-specific prebinding admission, and native execution of the actual independently specified community SHACL corpus. Existing default APIs, closed types, report/SARIF bytes, prepared-product format and default Python/WASM expectations remain compatible.

Ownership was checked through local branches/worktrees, all open PRs, and the issue body/comments/assignees. No 402 branch, worktree, open PR or ownership claim exists. Root main is preserved at 85d172070273; the merged remote baseline inspected is ab09fcaad8d0f393393c5bb77ad1174c28ba68c4. Preserve other owners' 410/428 PRs and all existing branches/worktrees, including 408/409 overlap, 416, 422, 423, 433 and the dirty 384 recovery candidate. Root owns 406 native XPath regex and Unicode; no edits to that worktree.

### Source and specification findings

- `crates/shapes/src/prebinding.rs` owns the algebra audit. Legacy constraints/validators/rules use `Strict`, while parameterized AF functions and target types use `AppendixA`; unbound targets use `ServiceOnly`. Preserve those legacy defaults exactly. Audit sites also exist in components, rules, extension usage, functions, target types and node expressions.
- `engine::ValidationOptions` is non-exhaustive and additive fields/builders are possible. `ShapesError`, both legacy report structs and `Constraint` variants are closed; changing their public shapes is prohibited.
- Shapes retain an immutable original `Arc<RdfDataset>` and parse provenance. Prepared products retain that dataset and restore private options to default. The richer door can reconstruct private evidence from this existing source, without a new product format or an extra serialized field.
- `report.rs` already owns blank relabeling, result/path emission and sorting; `shacl_corpora/report_grading.rs` remains the existing W3C grader home. Do not move it or write another canonicalizer.
- `sparql.rs` owns ambient context, worker transfer and prepared-query caching. The selected SHACL law must reach that context and be checked on reuse; it must compose with 406's independently selected regex law and fresh resource limits.

Normative authorities are dated, not moving `latest` aliases:

1. [SHACL Recommendation 2017-07-20](https://www.w3.org/TR/2017/REC-shacl-20170720/): Appendix A refuses MINUS, SERVICE and every explicit VALUES clause; AS cannot assign a potentially pre-bound variable, and nested subqueries project the required pre-bound variables. `shapesGraph`/`currentShape` are optional supported bindings and exempt from required nested projection.
2. [SHACL 1.2 SPARQL WD 2026-09-18](https://www.w3.org/TR/2026/WD-shacl12-sparql-20260918/): Appendix A refuses MINUS, VALUES mentioning a potentially pre-bound variable, and AS assigning one. It does not impose the REC hidden-subquery projection rule. PurRDF continues its deterministic local SERVICE refusal under the draft's explicit failure policy. Draft potential bindings differ from the REC; query-purpose roles must select the appropriate set.
3. Both dated SPARQL result maps choose a bound row message before messages declared on the SPARQL constraint/validator, then component messages where applicable, and carry the actual value of `sh:sparql` as `sh:sourceConstraint`. The draft additionally specifies a fallback to ordinary shape/constraint message declarations.
4. [SHACL 1.2 Core WD 2026-09-17](https://www.w3.org/TR/2026/WD-shacl12-core-20260917/) supplies the Core law paired with the SPARQL draft, including reified constraint message precedence. Existing Core behavior must be classified against the selected law; do not label legacy mixed behavior as an exact dated profile.
5. [SHACL-AF NOTE 2017-06-08](https://www.w3.org/TR/2017/NOTE-shacl-af-20170608/) governs the corpus's explicitly labeled rule cases. This is a separate extension applicability fact, not a claim that rules are REC Core.

### Required XPath composition (corrected before implementation)

The named SHACL handles are complete law bundles, not merely labels on report/prebinding behavior. REC20170720 §4.4.3 defines `sh:pattern` and `sh:flags` through [SPARQL1.1 REC20130321 §17.4.3.14 REGEX and §17.4.3.15 REPLACE](https://www.w3.org/TR/2013/REC-sparql11-query-20130321/#func-regex); the query Recommendation's normative FUNCOP reference names XPath2.0 REC20070123. Bind this to 406's native `xpath::Profile::Xpath20`, the [Second Edition REC20101214](https://www.w3.org/TR/2010/REC-xpath-functions-20101214/), under an explicit incorporated-errata contract: its Status identifies it as resolving the First Edition's known errata. This is the corrected XPath2.0 law, not an assertion that First/Second Edition wording is byte-identical or a switch to XPath3.1. Document that exact edition in profile accessors/evidence.

WD Core20260917 §7.4.3 defines pattern/flags through SPARQL1.2. The SHACL SPARQL draft's normative query reference is [WD20260820](https://www.w3.org/TR/2026/WD-sparql12-query-20260820/); its §17.4.3.12/.13 and normative XPATH-FUNCTIONS-31 reference bind matches/replace to XPath3.1 REC20170321. Its REGEX paragraph retains an older 2.0 title/section in prose, but its actual cited reference and function link resolve to3.1; use the identified dated3.1 reference, not that stale title or moving latest query edition. Bind this to 406's native `xpath::Profile::Xpath31`.

Thus `ShaclProfile::REC_20170720` requires native XPath2.0-20101214 and `WD_20260918` requires native XPath3.1-20170321 for Core pattern matching and every REGEX/REPLACE query path reached by that validation session. Profile selection explicitly admits that dependency bundle using the existing native engine API. `Legacy` alone preserves compatibility-engine routing unless the caller separately selects 406's explicit native door. Named dated handles cannot execute through the legacy regex engine. Callers may vary finite compile/admission/runtime budgets; an explicit incompatible regex-law override is a typed profile conflict before query execution, not precedence-dependent routing or silent fallback. A same-law override changes resource limits only. Profile/request/cache/evidence identity includes the required regex law and current admission; resource exhaustion remains operational refusal.

406's verified native API and host routing are a mandatory integration dependency for end-to-end dated validation proof. Tasks1–2 can implement law identities, isolated admission and richer report machinery now; they may test private admission functions independently, but no completed dated validation claim, public fallback, or partial passing stub is allowed while native routing is absent. Task3 closes the verified 406 integration and complete regex/report/prebinding bundle before its receipt. Both complete-issue dependencies (406 and recovered384 corpus) must be satisfied before PR creation.

Historical SHACL selection never introduces an RDF 1.1 carrier mode. RDF 1.2 remains the one kernel. The REC's applicable term subset and separately labeled RDF 1.2 draft cases are explicit corpus applicability, not silent downcasts.

### Concrete community dependency

The approved [384 recovery map](https://github.com/Blackcat-Informatics/purrdf/issues/384#issuecomment-5976578982) assigns the original unpublished `crates/conformance-kit`, `corpora/community` data/catalog/licensing, and the Rust `purrdf-sparql-conformance` community runner to the re-scoped recovery. It assigns SHACL adapter work to 402. Baseline ab09 contains none of that infrastructure. This plan does not salvage the rejected governor/collection rewrite, SHACL product v3, Python protocols or relocation of `shacl_corpora`.

Read-only staged candidate identities and counts are captured in `/tmp/purrdf-402-community-intake.json`. Candidate inventory `corpora/community/shacl/inventory.json` is Git blob 5713cd171ae8eee7b591c5735e98e186f24e50fe, SHA-256 `9112e6f69e4458cdb4dac891c69faa4c96642077335c4dc3304d99dc05dcb307`; catalog SHA-256 `114d3354bbef6d3c48221148680a1015bec456d9e9911209cdc67ec9f17d1873`. There are exactly 64 SHACL cases: 62 validation and 2 AF rule cases, 57 REC and 58 draft executions. REC expectations are 48 reports + 6 admission refusals + 1 semantic failure + 2 inference deltas; draft expectations are 51 reports + 4 admission refusals + 1 semantic failure + 2 inference deltas.

**Dependency blocker requiring root resolution before Task 4:** the current candidate's SHACL review ends with an inventory approval for SHA-256 22cebc8d…, whereas current staged/worktree bytes are 9112e6f6…; the review index and README additionally differ between index and worktree. These bytes are an identified candidate, not approved execution input. The 384 recovery owner must reconcile exact independently approved source identities, port only the approved kit/corpus/Rust runner, verify it and merge it through its own workflow. 402 must then ordinary-integrate that merged dependency and verify the actual landed identities/counts. No copying from dirty 384, engine-derived expected results, empty stand-in runner, ignored prerequisite, success with missing corpus or incomplete 402 PR is allowed. Independent Tasks 1–3 proceed without that dependency; Task 4 and completion stop if it is unavailable.

## 2. Baseline defaults

`.baseline` requires adversarial gap review, immediate complete remediation, no deferred work, and no issue/PR/process references in repository documentation. A discovered defect remains owned until fixed. `.deficiencies` has only notice and marker; verify that state before each completion boundary. `CONSTITUTION.md` is absent.

## 3. Standing constraints from `.goals`

Adopt the greenfield, Rust-first, maximal utility/performance/portability design. Use one RDF 1.2 engine and first-party homes. Profile selection is an explicit semantic request supported by the always-built core, never a Cargo feature or optional component. Add no runtime dependencies, no toolchain/source feature attributes and no new WASM semantic suite. Preserve all native suites and exact frozen corpora. Native Rust owns grammar, refusal, report identity, default projection and cache tests; existing release/WASM interface gates check portability and actual host behavior.

Do not change or investigate the toolchain, install/select components, source custom validation environments, create adapters/wrappers, alter global settings or inspect host/process/disk/cgroup state. Use the existing toolchain normally. Hooks are never bypassed. All source/generated/document changes carry proper licenses and regenerate projections through their existing owners.

## 4. Completeness contract

| Requirement | Concrete implementation and proof | Task |
|---|---|---|
| Explicit SHACL REC/draft selection, unchanged default | Immutable dated law handle with Legacy default; admission matrix and default equality tests | 1 |
| Additive API and preserve closed public types | New sibling complete-report/error types and profile-aware entrypoints; non-exhaustive options addition only; downstream struct/variant compatibility probes | 1–3, 5 |
| Actual `sh:sourceConstraint` | Private per-constraint occurrence metadata from original source graph, captured before result sorting and emitted by the one report writer | 2 |
| Normative message precedence | Profile-aware ordered source selection, row binding/template mapping and Core fallback; multilingual/directional literal tests | 2 |
| Blank identity of report contexts | Original scoped source identities plus minted report/path identities, shared/independent source domains and explicit egress correspondence | 2–4 |
| Profile-specific VALUES/MINUS/SERVICE/hidden-subquery policy | One purpose-aware algebra walker, applied before shape admission and on environment/profile reuse | 1, 3 |
| Every refusal has a valid neighbor | Native paired cases for every prohibited construct and query role, plus corpus controls; both execute through selected public doors | 1, 3, 4 |
| Exact report graph isomorphism plus required fields | Existing kernel canonicalizer through recovered conformance-kit contextual grader; required source constraint/messages/path/detail/result multiplicity retained | 2, 4 |
| Community SHACL wiring | Actual 64-case corpus, all 115 applicable dated executions, typed statuses and receipts in Rust community runner and native matrix | 4 |
| Unsupported profiles separate from failures | Explicit unsupported status, distinct from mismatch/admission/semantic/resource/IO/crash; both implemented dated profiles cannot be skipped | 1, 3, 4 |
| Default Python/WASM expectations unchanged | Native default byte/payload fixtures and existing Python plus WASM package/interface gates; no generic WASM replay | 3, 5 |
| Shapes/validate semver checks pass | Actual `cargo semver-checks check-release` for both packages against captured released baseline | 5 |
| Prepared/bound/restored/context path consistency | Shared law/evidence through parse, free/prepared/bound/focus/change/projected/product and worker contexts; alternating law tests | 1–3 |
| Complete issue workflow | Verified signed task commits/pushes/issue receipts, independent reviews, final-head CI/CodeRabbit and root-serialized ghprsq | 0–6 |

## 5. Enhancement Audit and design decisions

**Transformation, adopted (L, decisive payoff): a validation session is a dated semantic law over an immutable source context, producing a provenance-carrying report.** An enum switch plus extra report fields cannot ensure that a cached query was admitted under the law the report claims, or that an isomorphic result names the correct data blank. Structure the implementation around one private request/session holding selected SHACL/XPath law bundle, current options/governors and source identity context. A rich report retains that law and correspondence. Parsing/admission, query execution and report interpretation consume the same session. Legacy entrypoints are deliberate projections of this same engine under the Legacy law. This provides a reusable formal boundary for preparation, restored artifacts and community grading without inventing another validator or ontology.

**Leverage, adopted (M, high payoff):** keep the existing prebinding walker, result ordering/emitter/blank relabeling, retained shapes dataset, product codec, ambient context and kernel canonicalizer. Add policy/evidence to those homes. The recovered conformance-kit is the exact-grading home; no second blank matcher in shapes/validate/community tooling.

**Utility, adopted (M, high payoff):** immutable public `ShaclProfile` handles with named constants and stable IDs/specification accessors; a sibling complete report with read-only accessors, canonical RDF output, legacy projection and source correspondence; shared validate boundary functions usable by the Rust native runner without interpreter glue. Carry selection through reused and restored prepared values, not only a cold text helper.

**Robustness, adopted (M, high payoff):** typed admission reasons versus explicit solution failure versus operational resource/input/crash failures; profile/environment admission on cache reuse; full result multiplicity and RDF 1.2 nested-blank identity; cross-profile interleaving and low/high governor tests; unknown profile tests and corpus inventory/selection refusal tests; exact default byte/product and semver verification. Preserve governored error precedence rather than converting an exhausted computation into a conformance verdict.

**Performance, adopted (S/M, measurable payoff):** retain source graph once with Arc and compact per-constraint identity metadata, materialize full context only for the requested rich door, keep legacy result overhead bounded, and benchmark cold/warm parse/admission and legacy/rich validation separately with equal inputs and publication schedules. Report measurements and allocation costs, not an asserted optimization.

Declined as closed design decisions: a second canonicalizer/grader (duplicates the kernel/kit law); a new product wire revision (existing source graph already carries the required evidence, and default compatibility is required); runtime dependency or Cargo feature for profiles (violates the repository); live-network SERVICE execution (violates local deterministic refusal); another WASM semantic corpus (user requires Rust ownership); copying a rejected recovery implementation (no authority or qualification).

## 6. Task 0: approved isolated setup

1. Root independently reviews this complete plan, `.goals` compliance and Enhancement Audit. Incorporate every concrete correction. Obtain root approval before repository mutation; then post the approved exact plan to issue 402.
2. Immediately refresh ownership again: all local branches/worktrees, open PRs, issue comments/assignees. If a 402 owner appears, stop and coordinate rather than duplicate it.
3. Fetch origin normally without modifying root main. Create branch `paudley/402-shacl-profiles-reports` and worktree `/home/paudley/Active/purrdf/.worktrees/402-shacl-profiles-reports` from actual `origin/main`; confirm branch/path/head, original root/sibling states, and notice-only deficiency ledger. Preserve every unrelated path and snapshot. Use existing worktree exclusion; do not alter global configuration.
4. Re-read worktree AGENTS/.goals/.baseline and current applicable source. Establish native baseline `cargo test --locked -p purrdf-shapes -p purrdf-validate` and source hygiene with logs under `/tmp/purrdf-402-*`; no host/toolchain probes. Record real failures and fix attributable defects. Baseline does not silently qualify a later source.
5. Capture exact released `rust-v3.0.1` tag object and remote readback as the additive semver baseline for shapes and validate (the repository's recovery compatibility baseline); if live release baseline is newer, select and record the actual last release instead. Use normal existing semver-checks only, no installation work. Capture default report/SARIF/product fixtures before source edits.
6. Coordinate root 406 reservations before shared file edits. The source homes expected to overlap are shapes `engine.rs`, `shapes.rs`, `sparql.rs`, `product/mod.rs`, and validate `shacl.rs`. 402 makes no xsd_regex/Unicode changes. Require and ordinary-integrate the independently verified merged 406 native API/routing before Task3 dated end-to-end qualification; requalify both complete bundles after integration. Do not cherry-pick/copy dirty406 work or claim the legacy engine fulfils the dated law.

## 7. Implementation tasks

### Task 1 — dated request and complete query-admission law

- Introduce a public immutable `ShaclProfile` with private identity and named Legacy, REC20170720 and WD20260918 constants. The WD handle names its Core20260917/SPARQL20260918 bundle. Stable string decoding refuses unknown dates as a typed unsupported-profile outcome; there is no nearest-date or legacy fallback.
- Declare the exact required XPath editions in immutable handle accessors, with no duplicated XPath enum or engine. Add a purpose-aware standalone `admit_query` door returning structured refusals; this proves parsed query admission only. Task3 installs the options selector and validation constructors with the actual verified native bundle, so no public validator ignores a selection or routes a dated handle through compatibility regex. Keep public product capability `product::ShapesProfile` distinct from the normative SHACL profile. Document extensions separately.
- Build one private request and query-purpose admission policy in `prebinding.rs`. Preserve exact legacy constraints/functions/targets behavior. Apply the dated matrix to ASK/SELECT constraints and component validators, rule CONSTRUCT, parameterized function bodies, target types, scalar/node expressions and parameterless targets according to their actual prebinding roles. Treat REC shapesGraph/currentShape support and AS restrictions separately from their optional subquery projections; do not assume draft and REC variable sets coincide.
- The standalone admission API checks its actual query form and purpose-specific bindings. REC optional shape-context bindings remain restricted for assignment but exempt from mandatory subquery projection; a caller parameter with the same name remains required. Preserve Legacy wrappers and diagnostic strings. Repair aggregate argument/order-expression traversal in this single walker using a failing-first native refusal and valid local neighbor. Task3 installs full constructor/prepared re-admission wiring.
- Use typed private/public sibling admission reasons; do not add a ShapesError variant or classify display-string substrings. Legacy errors retain their contract through projection.
- Add failing-first native paired refusal/valid controls including local vs prebound VALUES, MINUS versus OPTIONAL/NOT EXISTS, forbidden AS versus local alias, hidden/projected nested SELECT including SELECT-star scope, SERVICE versus local GRAPH, optional REC variables and ASK value/component parameters. Check default behavior exactly; demonstrate both dated profiles and unsupported decoding through actual public doors.
- Review wiring with root before Task 1 commit. Focused native tests, warning-denied package Clippy and parser/thread-local hygiene must pass. Publish the verified task receipt.

### Task 2 — evidence-bearing complete report, source constraint and messages

- Add sibling complete report/result/context types with private storage and accessors, plus a typed validation error whose categories distinguish admission, explicit solution failure, underlying shape errors, resource refusal and unexpected execution failure. Retain all existing closed structs/enums and public function signatures.
- Private per-constraint occurrence metadata records the original actual sh:sparql node, message declarations and origins, and source dataset identity. Capture result evidence while the engine is evaluating that exact occurrence, before canonical result sorting; sorting carries payload and evidence together. Do not recover the source constraint after the fact by matching equal query text, result tuples or the report's own labels. Equal query literals on different constraints remain distinct.
- Carry scoped data/shapes source references for focus, value, source shape and source constraint, including blanks nested in RDF 1.2 terms. A shared source retains shared identity; independently acquired equal bytes/labels have distinct domains. Minted report/result/detail/path nodes belong to their own domain. Reuse/extend the existing report relabeler and single result/path emitter, with an optional internal evidence view that leaves legacy output byte-identical. The complete report exposes egress correspondence to original source identities and exact source context without fabricating RDF vocabulary.
- Implement ordered SPARQL message selection (row binding, constraint/validator, component, then applicable Core fallback), literal types/language/direction, template bindings and unbound placeholders. Selected REC Core shape message behavior and selected WD reified-message precedence are explicit. Legacy precedence remains unchanged. Native tests pin conflict priorities and absence fallbacks, not just each message in isolation.
- Add complete-report validation siblings for immutable dataset sources and prepared validators. If a generic opaque view lacks admissible source identity evidence, the rich door requires an explicit source context or gives a typed refusal; it cannot synthesize a success from spelling. Legacy view APIs remain unchanged. Focus/change/projected helpers reuse the same internal result/evidence path.
- Native regression/properties: named and blank source constraints; two equal queries/different source nodes; data/shapes same-label separate domains and deliberate same-source alias; wrong source blank that still permits report-only isomorphism; duplicate results; complex sequence/inverse/list path topology; recursive detail and annotations; RDF 1.2 nested blanks; minted-label collisions; retained-source lifetime after caller drop. Pair every evidence refusal with valid context.
- Check native legacy byte equality, complete required fields, package tests/Clippy/rustdoc and focused allocation behavior. Add matched native bench cases to existing shapes bench homes (setup excluded, fixed corpora, cold vs warm separately), measuring requested complete context versus legacy cost. No new generic WASM bench/test. Publish verified task receipt.

### Task 3 — one law across preparation, restore and shared Rust boundary

- Add the selected handle to non-exhaustive `ValidationOptions` with a Legacy-default builder and profile-aware sibling constructors that admit the selected request before parsing. Existing infallible options setters remain source-compatible; execution re-admits any newly selected law. Incompatible XPath overrides produce typed conflicts; same-law overrides vary finite admission/runtime limits. Thread the private request/evidence through free validation, prepared bind/rebind/bind_shared/bind_view/focus/change/projected paths, linked imports/shapes graphs, and ambient worker/context-fork snapshots. Include profile in prepared-query admission/cache identity or re-admit under the current request before reuse; never carry a prior success/refusal as a verdict for another law. Governors remain one operation's fresh current budget with resource failure precedence.
- Compose with the verified 406 native engine through the explicit dated bundle described above. Native tests cover sh:pattern plus constant-linked/dynamic/cached REGEX/REPLACE under each bundle, grammar differences (noncapturing groups/q), backreferences and typed resource refusal. Alternating compatible bundle requests, incompatible-override refusals and prepared/context reuse must not cross law or budget boundaries.
- Preserve product codec/version/default bytes and frozen vectors. Derive private metadata from the authenticated retained source dataset. Add profile/options-aware admission/rebuild siblings that apply the selected request before AST/query admission, registry installation and evaluation; existing admit/rebuild doors use Legacy. The request is not trusted from an old memo and does not become a new serialized field. Profile changing a restored preparation must re-admit actual current capabilities/queries before execution.
- Add shared `purrdf-validate` Rust siblings for complete contextual report/payload and typed status, plus profile-aware SARIF/complete report doors where relevant. Existing host-facing functions, default JSON shapes, Python and WASM exports remain unchanged. Reuse the existing lexical JSON/record codec and SARIF implementation, never a second validator or report mapper. Required source correspondence is data, not inferred from status prose.
- Native default text/payload goldens and all public route tests prove unchanged projection. Alternate valid/invalid profiles and low/high budgets through fresh/prepared/bound/linked/restored contexts; force explicit solution failure independently of resource/parse failure. Run existing default Python expectations and actual existing WASM interface/package tests during final qualification, without new semantic WASM cases.
- Root reviews the shared wiring and product/default compatibility. Fix every finding; focused shapes/validate native checks and related hygiene must pass, then publish the verified receipt.

### Task 4 — integrate the reviewed recovery dependency and execute the real SHACL corpus

- Require the 384 owner/root to close the exact provenance discrepancy and merge only its reviewed kit/corpus/Rust SPARQL runner. Capture merge SHA, source hashes and independent review identities. Ordinary integrate merged origin/main into the 402 worktree, preserving unrelated owners. This task must not run against the dirty candidate. No expectation changes from an engine observation.
- Extend the **existing Rust** `purrdf-sparql-conformance::community` execution adapter and existing `community-conformance` binary to invoke the new shapes/validate siblings for SHACL validation/rules. Keep conformance-kit unpublished and runtime published crates independent of it; tool/dev edges use the existing workspace/layer declarations. Keep existing W3C `shacl_corpora` under shapes. Do not import rejected recovery APIs or create Python/foreign protocols.
- The kit owns graph/report/outcome grading. Feed it dedicated full report graph/root, independent expected source correspondence and producer's actual source correspondence. Compare under the independently declared dated test-format policy; required source constraint, exact messages, complex paths, detail/annotations and multiplicity cannot be dropped. Use Exact where declared; any permitted optional-field projection is test-format law, never engine convenience. Canonicalization limits stay a typed non-pass.
- Execute all 64 SHACL entries, every applicable profile/AF extension, and refuse missing/duplicate/unknown selection IDs. Verify the landed inventory's expected totals (currently REC57/WD58, 115 executions) and all report/admission/semantic/inference categories. Built-in dated profiles and required capabilities must have no unsupported/unexecuted applicable cases. Explicit unrecognized-profile probes report Unsupported separately and do not become a passed negative case. Other-date applicability is Inapplicable, not failure or false pass.
- Stable admission reasons match independently declared reasons; `failure=true` maps to semantic failure, not a normal violation. Resource exhaustion, malformed IO, exceptions/crashes, missing files or unavailable prerequisites cannot satisfy semantic rejection. Add native runner tests proving every category and strict totals fail closed.
- Integrate the SHACL row into the native conformance matrix using Rust-produced explicit per-profile counts/statuses. Retain raw actual artifacts plus machine-readable/EARL receipts keyed by exact source, corpus identity and selected law. Report separate unsupported/inapplicable/resource/failure columns. Update conformance/spec docs from actual output with no issue/process references; no W3C certification claim.
- Run the real full applicable SHACL selection and the existing native community/matrix path. Every expectation must pass; if a semantic defect appears, fix its actual owned home rather than xfail/skip it. Publish verified integration and native evidence receipt.

### Task 5 — complete final qualification and compatibility proof

- Integrate all already merged main changes normally before final qualification, coordinate any shared 406 changes and resolve both implementations exactly. Verified signed integration commit, push and issue receipt precede frozen final-head checks. If main/source changes again, repeat affected local checks plus final-source hosted qualification; stale receipts do not qualify a new source.
- Run full `cargo test --locked -p purrdf-shapes -p purrdf-validate -p purrdf-sparql-conformance -p purrdf-conformance-kit`, focused native profile/context/community tests, warning-denied all-target package Clippy and rustdoc. Existing W3C 1.0/1.2/first-party corpus and prepared-product equivalence suites remain intact with exact counts/XPASS discipline. Frozen GTS/SHACL corpora are not regenerated.
- Run `cargo semver-checks check-release -p purrdf-shapes --baseline-rev <captured-release>` and the equivalent validate command. Both must actually pass. New APIs are additive; closed type/signature regressions must be fixed, not waived. Preserve external struct literals, exhaustive matches, old generic/free/prepared/product routes and default host payloads.
- Run normal full `make check`, `make metadata`, `make conformance`, documentation/book checks applicable to changed docs, `make wasm`, `make wasm-pkg-test`, and existing focused `make wasm-test`; run default Python integration gate (`make pytest` / repository's existing installed-project invocation). These are existing shipping/host obligations, not added compiler testing or WASM semantic corpora. No special toolchain commands/settings. Record exact actual command, head, status, counts and artifact paths; unrun is not pass.
- Execute the matched native bench measurements and allocation checks; retain inputs/settings/raw outputs and measured limitations. Report only proven performance/cost, with no CI threshold weakening.
- Hygiene includes features/deps/layers/helpers/terminals/thread locals/parser drops/generated projections/SPDX/spec attribution, default product/report bytes, notice-only deficiencies, mechanical deferral scan, and clean tracked state. Resolve every hit/failure before completion.
- Independent root final source/wiring/compliance review must pass. Make repairs as their own normal verified signed commits/pushes/issue receipts and rerun affected local plus complete final-head hosted qualification.

## 8. Validation gates and completion rule

Each task runs its focused native proof before committing. Final qualification includes actual source-matching all native/default host/compatibility gates. No changed parser/evaluator branch is considered covered solely because two PurRDF paths agree; independent normative fixture/report/source context establishes correctness. Both built-in dated laws must implement all applicable community requirements. Exact corpus dependency identity and all SHACL results are mandatory; dependency unavailable means an explicit blocker, not a completed issue.

Before posting this plan, before each commit and before PR/merge, scan the complete plan, changed source/docs, issue/PR text and squash notes for mechanical deferral tokens using stage-1's regex. Fix any real omission; record only proven lexical false positives. Never weaken gates, add emergency-ledger entries to normalize incomplete work, or claim a percentage/metric substitutes for acceptance.

## 9. Commit, push and issue receipt points

After Tasks 1, 2, 3, 4 and final qualification/docs, stage only the reviewed task paths and use ordinary signed `git commit` with all hooks. Wait for slow hooks; repair failures without escape switches. Verify exact signed commit/head and clean task state, push normally (`-u` for first push), read back the exact remote branch SHA, then post issue 402 receipt with substantive behavior, actual checks/counts/artifact paths and exact head. Integration/feedback repairs follow the same verified commit/push/readback/receipt sequence. No batch unrelated changes, branding or co-author attribution. A failed hook/sign/push/post is an explicit blocker. Task 0 is setup only; no pretend source commit.

## 10. Final pull request, review and merge readiness

Create the PR only after every task and acceptance gate is complete, committed and pushed. Title describes dated SHACL validation and complete contextual reports. Body explains behavior, unchanged default compatibility, exact native community/profile totals, typed status distinctions, semver and real measurements/gates. Link `Closes #402` and post the approved plan. Do not create a partial or draft completion PR while Task 4 is blocked.

Stage-2 compares final source against every issue criterion and every actual inline/review comment. Request one normal exact-head CodeRabbit review, verify coverage of the exact head and dispose every actionable finding with verified repair receipts. Require all normal hosted CI/docs checks successful on that final head; compiler/source-bound assembly aggregation, when the existing workflow requires it, remains strict with its existing matrix/sites/limits. Do not add diagnostic loops or toolchain changes.

For finalization, ordinary synchronize origin/main in this assigned worktree and requalify any changed source. Prepare sibling `402-shacl-profiles-reports.squash_notes` containing the final implementation, exact corpus/default compatibility/semver evidence, closed reviews and clean state. Root independently approves notes and grants the exclusive merge lock. Confirm issue/PR notes posted, notice-only ledger, no deferrals, exact head/remote/main parity and actual final-head reviews/checks. Only `/home/paudley/stage/root/bin/ghprsq` may merge. If base moves, synchronize and qualify normally. Verify signed result/tree, notes/audit refs/archive, issue closure and remote branch deletion, then remove only this clean owned worktree/local branch, preserving root main and all sibling/candidate/evidence paths. Release lock and report exact merge SHA and evidence.


### paudley — 2026-10-05T06:33:24Z

Task 1 complete: dated query admission, with unchanged validation defaults.

Signed commit `8520523dd920eefe6d6c948854b0860d3f572c66` is pushed to `paudley/402-shacl-profiles-reports`; exact remote SHA readback agrees and the worktree is clean. Normal commit hooks passed. Independent wiring review approved the bounded admission contract and the production aggregate traversal repair.

The additive `ShaclProfile` API provides stable Legacy/REC20170720/WD20260918 identities, exact specification editions, unknown-profile refusal and purpose-aware parsed-query admission. The existing walker retains legacy diagnostic contracts while applying the dated VALUES/MINUS/SERVICE/assignment/projection rules. Aggregate arguments and aggregate ordering now traverse nested expressions; the native regression failed on the former walker because SERVICE hidden in aggregate EXISTS was admitted.

Actual verification on this source:

- Baseline shapes + validate native suite: 2170 passed, 2 existing ignored.
- Final shapes + validate native suite: 2185 passed, 2 existing ignored, including all 14 new public admission tests and the repaired aggregate regression. `cargo test --locked -p purrdf-shapes -p purrdf-validate`.
- Warning-denied all-target package Clippy, package rustdoc, formatting and `git diff --check`: passed.
- Shapes parser-drop check: 12 files; thread-local hygiene: 51 statics; shared-helper gate: 77 jobs, 23 reasoned variants and 90 distinct rows over 1785 files: passed.

Native artifacts: `/tmp/purrdf-402-baseline-native.log`, `/tmp/purrdf-402-aggregate-failing-first.log`, `/tmp/purrdf-402-task1-native.log`, `/tmp/purrdf-402-task1-clippy.log`, `/tmp/purrdf-402-task1-rustdoc.log`, `/tmp/purrdf-402-task1-helpers.log`.

This is the standalone query-admission task, not end-to-end dated validation. The complete report task is next. Actual native XPath bundle integration and execution of all independently approved community cases remain required tasks in the full plan; no completion PR has been opened.


### paudley — 2026-10-05T09:26:41Z

Task 2 checkpoint is signed and pushed: [`37dc716abca8f63bc2af40ce7b3938ef1e395b1e`](https://github.com/Blackcat-Informatics/purrdf/commit/37dc716abca8f63bc2af40ce7b3938ef1e395b1e) on `paudley/402-shacl-profiles-reports`. Normal commit hooks passed, `git verify-commit` reports a good signature, remote branch readback matches the exact head, and the assigned worktree is clean.

The additive complete-report API retains the immutable data/shapes acquisitions and original scoped blank identities. Actual `sh:sparql` source occurrences travel with their result and recursive details through ordering and the existing RDF emitter. Independent acquisitions stay distinct; deliberately shared acquisitions preserve identity. Query acquisitions mint distinct deterministic blanks. Changed source slots and invalid source context return typed refusals. Canonical per-focus failure capture includes untyped errors, and actual governor refusal takes precedence over report construction. Legacy public types, signatures, report emission, product version, stage constant and frozen product bytes remain unchanged.

The private source-occurrence cache is reconstructed from the retained authenticated shapes graph. The complete structural census includes it. Only its exact private `Shapes::sparql_sources: OnceLock<Arc<ConstraintSources>>` declaration is excluded from the carried-model preimage. Native proofs preserve the shipped stage ID after cache addition/removal/warming, keep warm clone identity, verify source reconstruction/report parity and actual product bytes, and continue detecting public, other-private, preamble, model-variant and meaning-table changes. The shipped stage ID remains `4504ff68f40d1a2788468f8ff85bf95509fc0ed686476229dd2fb8ffeb48a63d`.

Actual validation receipts:

- `cargo test --locked -p purrdf-shapes -p purrdf-validate` repeated on the exact committed head `37dc716abca8f63bc2af40ce7b3938ef1e395b1e`: **2,222 passed, 2 existing ignored, 0 failed**, 93 suite summaries. This includes 23 complete-report integrations, 11 new private report proofs, all 12 existing allocation cases, all 28 product census cases, frozen product equality and public doctests. The ignored tests are deliberate committed-vector regeneration commands, unchanged.
- `cargo clippy --locked -p purrdf-shapes -p purrdf-iri --all-targets -- -D warnings`: passed.
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p purrdf-shapes -p purrdf-iri --no-deps`: passed.
- Formatting, source whitespace, shared helpers/hash domains, layers, resolver, serializer, terminals, thread locals, build profile, issue-reference, branding and specification-attribution gates: passed. Notice-only deficiency ledger verified.
- Complete Core warm allocations remain **5 calls at both 16 and 32 foci**. The existing restored-admission allocation pin remains **277**.
- This branch's actual Cargo metadata registers **104 unique benchmark targets**; the **29 documented paths** match the prose/table in identical order, every one actually registered. The inventory correction changed documentation only. One earlier rustfmt-only test line break was subsequently covered by the complete exact-head native rerun; the final source manifest remains unchanged.

Native cost measurements ran through the existing `shared_views` target:

`PURRDF_BENCH_HOME=/tmp/purrdf-402-task2-bench cargo bench --locked -p purrdf-shapes --bench shared_views -- shacl_complete_reports`

All eight cases passed with **100 samples each**, 3-second warm-up, 5-second target measurement and a seeded 95% bootstrap interval over 10,000 resamples. Each fixture has 128 foci. Product restoration and destruction are outside cold-bind timing; warm reports reuse their views/context. Medians in microseconds:

| Fixture / boundary | Compatibility | Complete |
| --- | ---: | ---: |
| core / cold_bind | 1.107 | 25.160 |
| core / warm_report | 103.849 | 112.744 |
| select / cold_bind | 1.037 | 33.470 |
| select / warm_report | 547.357 | 705.760 |

These are one native report-only run measuring the requested evidence cost, without a speedup claim or timing threshold. All new semantic and cost proofs are Rust-native.

Task-owned receipts: `/tmp/purrdf-402-task2-final-native.log` (exact committed head), `-native.log` (earlier run), `-clippy.log`, `-rustdoc.log`, `-projection.log`, `-hygiene.log`, `-source-hygiene.log`, `-helpers.log`, `-bench.log`, `-bench-summary.json`, `-bench-inventory.json`, and `-validation-source.json`. The source manifest binds all 15 committed paths to their SHA-256 values and this exact head. Failing-first receipts captured the canonical parallel-failure race and equal-value/different-evidence ordering defect before their repairs.

This checkpoint completes Task 2 machinery under the compatibility law, with isolated native REC/WD mapping proofs. **Issue 402 remains open.** Task 3 must integrate the actual admitted dated SHACL/XPath bundle through preparation, runtime, cache, restore and shared boundary; restored occurrence reconstruction must use that SAME bundle, never Legacy re-admission. Task 4 must reconcile the independently specified community corpus identity and execute all **64 cases / 115 profile executions** through its existing exact grader. No completion PR is opened before these mandatory requirements and final qualification pass.


### paudley — 2026-10-05T09:43:33Z

The signed native XPath dependency checkpoint `90ae47bdb5b312281a4154643756ff54277efcc5` is integrated into this branch by signed merge `7cd49ce1c8e5e82283e140cbf7e806b508c9490b`, with Task 2 head `37dc716abca8f63bc2af40ce7b3938ef1e395b1e` as its first parent. Both the dependency signature and merge signature verified; normal commit hooks passed, the worktree is clean, and the pushed remote branch readback equals the merge head.

The only merge conflict was the benchmark documentation. Actual combined Cargo metadata proves 104 unique registered benchmark targets; the prose and inventory table name the same 31 unique registered targets in the same order. Metadata and reconciliation receipts are `/tmp/purrdf-402-task3-dependency-metadata.json` and `/tmp/purrdf-402-task3-dependency-bench-inventory.json`; hook, signature and push logs use `/tmp/purrdf-402-task3-dependency-*.log`.

This checkpoint supplies the existing native XPath profile, finite limits and successful-program cache homes. It does not complete the dependency's host routing or this issue. Task 3 must wire one admitted dated bundle through SHACL preparation, execution and restoration, preserving complete report source identities and deterministic query blank minting. The independently reviewed community corpus dependency and all applicable SHACL executions remain mandatory before a completion PR.


### paudley — 2026-10-05T13:06:20Z

### Task 3 supporting checkpoint: exact function-admission causes

Signed commit `0142b7a30ca8bb38b85b7780c3ab7d1067031f1a` (tree `983bf90ef302ef665f4fe2056499f4335c58ab4c`) passed the normal commit hook, was pushed to `paudley/402-shacl-profiles-reports`, and has an exact remote readback. This checkpoint contains only the 12 qualified evaluator paths; the continuing Shapes changes are excluded.

The additive invocation hook carries an exact typed refusal through child and worker contexts. The observer receives only the canonically selected error after the actual final source and governor checkpoints. A losing query refusal cannot replace a stale-source or budget outcome. Default requests retain `None` and do not allocate this observer machinery. The existing closed protocol failure codes remain unchanged.

Read-only `prepare_interned_request` inspection uses the existing request-parameter/rewrite cache and exposes the original algebra without accessing data or executing expressions. A native bound-property-function neighbor proves that inspection and raw execution reuse that actual cached plan; mismatched/free prebinding modes still refuse.

Actual qualification on the unchanged 189-file evaluator Rust snapshot:

- `cargo test --locked -p purrdf-sparql-eval`: 2,198 passing tests/doctests across 80 result groups, with 15 existing ignored cases; log `/tmp/purrdf-402-task3-sparql-eval-native-final-source.log`.
- `cargo clippy --locked -p purrdf-sparql-eval --all-targets -- -D warnings`: passed; log `/tmp/purrdf-402-task3-sparql-eval-clippy-rerun.log`.
- Warning-denied package rustdoc: passed; log `/tmp/purrdf-402-task3-sparql-eval-rustdoc.log`.
- Six native invocation cases include real indexed-worker cause selection, production serial UDF routing, short-circuit non-invocation, complete optional declarations, and eight public fallible doors with ready/source-failure neighbors. The premature-observation control failed first at the witnessed source-precedence defect, then the repaired cases passed.
- All 12 prepared-parameter cases passed. The uncontrolled-stack test correction is byte-identical to the already qualified `d0080891e96e0b9058ff7b5867e2baaefc56cfa3` dependency; its independent controlled refusal/canary and large-stack success proofs remain intact. This does not represent a complete synchronization of that dependency checkpoint.

Evidence manifest: `/tmp/purrdf-402-task3-evaluator-checkpoint-receipt.json`. Staged paths matched the qualified source hashes exactly. The deficiency ledger remains notice-only; the mechanical scan's two hits were the implemented local `deferred` observer variable, not undone work.

This is a verified supporting checkpoint, not completion of Task 3 or issue 402. The actual complete dated SHACL/native-XPath routing and restoration are still being integrated with the separately owned regex work. The required independently approved community corpus and all applicable executions remain part of this issue's acceptance. No PR or completion claim is made here, and no WASM semantic cases were added.


### paudley — 2026-10-05T13:46:21Z

Private query-admission/source-occurrence checkpoint is signed and pushed at `adcdb47a55e0cc88fe7ef6c6be6a345695c13688` (tree `e0b7f63fbf2c4eee252958a9037d55df9c54d32a`). The exact remote branch head was read back, and the worktree is clean. Normal commit hooks passed; all 23 committed file hashes match the independently reviewed and qualified snapshot.

This checkpoint retains authored query roles and all potentially pre-bound declarations, audits original cached algebra at the actual executor, and carries the admitted source occurrence/profile/provenance through reconstruction and reports. REC graph-wide declarations remain separate from WD restrictions at reached execution and dormant AF functions. Native proofs cover actual ASK target invocation, target/source mismatch with a valid neighbor, optional declarations, canonical typed user-function refusals, legacy-warmed/restored preparations and dated global-rule execution. The existing ambient-state ledger includes the new query-law context; no WASM semantic tests were added.

Validation on these unchanged source hashes:

- `cargo test --locked -p purrdf-shapes`: exit 0, 1,962 tests/doctests passed across 79 result groups, one existing ignored case. This includes 19 private parser/admission cases and existing legacy/corpus/allocation/product suites.
- `cargo clippy --locked -p purrdf-shapes --all-targets -- -D warnings`: exit 0.
- `RUSTDOCFLAGS='-D warnings' cargo doc --locked -p purrdf-shapes --no-deps`: exit 0.
- `make thread-local-hygiene`: exit 0; ledger/static parity passed for 52 statics.
- `python3 scripts/check-shapes-parser-drops.py`: exit 0; 13 parser files checked.
- Staged whitespace and mechanical deferral checks passed; `.deficiencies` contains only its notice and marker.

The target-evidence and dated-global-route regressions have retained failing-first receipts, then pass in the complete native suite above. Task-owned log/source hashes and exact-head records are retained in `/tmp/purrdf-402-task3-shapes-source-law-receipt.json` and `/tmp/purrdf-402-task3-shapes-checkpoint-receipt.json`.

This is a qualified private admission/occurrence checkpoint, **not full Task 3 or issue completion**. The coordinated, qualified #406 dependency and actual dated native execution/restoration composition remain open, including exact canonical native/query/source/governor cause precedence. Complete shared validation/report surfaces, the mandatory 64-case/115-execution community corpus acceptance and final integration/qualification are still required. No completion PR is being opened at this checkpoint.


### paudley — 2026-10-05T15:22:34Z

Dependency integration checkpoint: signed head `aef3e2c7ad95e38e48859906b28875708ade1b47`, tree `02f9d6c14215c34cf265f70aad99ef9387e33c90`, normally merged the reviewed #406 checkpoint `254b1554f7208e1565498f0a38f8d32098d6b7a5` into the qualified #402 admission/report source. Normal commit hooks and push succeeded; exact remote head readback matches and the worktree is clean.

Resolved the four source conflicts by retaining both dated query-law and native XPath ambient states, applying both QueryOptions selections, and wrapping the existing generic report executor in the single native canonical root-error reducer. Both complete-report and explicit native XPath documentation sections remain. The combined benchmark source inventory is104 registered targets and32 identically ordered narrative/table entries.

Post-integration native tests, Clippy, docs and measurements are **NOT RUN** yet under the coordinated measurement hold. The dependency's1,907 native passes belong to its prior paired-error source, not this integrated source. Actual public dated composition, restored/current request admission and shared validation boundary remain in progress; no full Task3 or issue-completion claim.

The mandatory community acceptance is still **NOT MET**: the reviewed conformance-kit/corpora dependency is absent current main. The approved plan requires64 SHACL cases and115 applicable dated runs (REC57/WD58), using independently graded exact graphs and required fields. The known pre-rewrite inventory identity discrepancy remains assigned to the recovery owner/root; no replacement fixtures or grader have been created.


### paudley — 2026-10-05T18:40:26Z

The composed dated/native execution and complete-report checkpoint is signed, normally committed and pushed at `9b88ac778403c0aa2d15aa6ce42b06613015e8f3` (tree `7771dcb01052a3f6514fe880f3c4f0a0832ae3f1`, signature G). Remote readback matches exactly and the owned worktree is clean. The checkpoint consumes the independently reviewed native allocation mapper repair `19cf6a160c1429b79edcd748dce8da7647c11092` via signed `-x` cherry-pick `b5a4f7994ef1986779b2ef1940eb63a4f5340cd2`; its exact patch identity matches and all 24 owned source paths were preserved.

One admitted dated SHACL/XPath request and retained occurrence context now compose through the existing bind, validation, restoration and rule doors. The existing paired error home retains actual admission/source/governor errors alongside native query causes. Complete Rust report/payload/SARIF siblings retain the report root and authored source correspondence. Default compatibility signatures, frozen product stage identity and product bytes stay pinned by the native suites. Only the existing suspension-state ledger guard/comment accounts for the two actual typed ambient slots; no WASM semantic test, runner, dependency or feature was added.

Two actual Legacy-wrapper witnesses failed first, proving that shape options could replace the wrapper's native law and current finite limits. Their minimal selector correction passes both public cases and the high-limit recovery neighbors. A separate typed Allocation-to-Query status witness also failed first: it produced `NativeXPathRefused` instead of `ResourceRefused`. After consuming the one-home diagnostic mapper repair, the composed native regression passes all eight resource identities for Pattern and Query, retaining generic and poisoned-cache controls. These allocation tests prove deterministic typed conversion/status composition; they do not claim a host allocation failure was forced.

Actual qualification on the exact committed source:

- Full Shapes and Validate Rust suites/doctests: **2,290 passed, 0 failed, 2 existing regeneration ignores, 98 result groups**.
- Focused public native targets: **48 passed, 0 failed/ignored** (13 dated execution, 15 existing Shapes XPath, 13 SPARQL XPath, 7 shared complete-report cases).
- Warning-denied all-target Clippy for Shapes, Validate, SPARQL evaluator and WASM; warning-denied docs for Shapes, Validate and SPARQL evaluator: passed.
- Unchanged helper/hash-domain/layer self-tests and actual gates: passed; 77 enforced jobs, 92 exact distinct groups, 77 registered domains, 207 first-party edges across 42 members.
- Thread-local and parser-drop self-tests/tree checks: passed (53 actual statics and 13 parser files). Formatting and `git diff --check` passed. The deficiency ledger is notice-only, and the added-source mechanical deferral scan is empty.
- Normal commit/hook reader 23489 and push reader 61476 are terminal EXIT0. Every one of the 26 source hashes (24 owned paths plus the two consumed checkpoint paths) matches the captured passing source and committed tree.

Task-owned receipts are `/tmp/purrdf-402-composed-allocation-fixed-source.json`, `/tmp/purrdf-402-query-allocation-failing-first-source.json`, `/tmp/purrdf-402-legacy-wrapper-failing-first-source.json`, `/tmp/purrdf-402-allocation-checkpoint-consumed-receipt.json`, and `/tmp/purrdf-402-task3-pushed-receipt.json`; failed runs and source-preservation copies remain intact. Wider final integration review and final-source current-main/dependency, release-compatibility, default-host, workspace and hosted qualification are not claimed by this checkpoint.

**Full issue acceptance remains NOT MET.** The mandatory reviewed community kit/corpus dependency is absent from this branch and main: the actual 64-case/115-applicable-run SHACL acceptance has not executed. Its declared recovery owner retains the protected source; no unreviewed recovery code, duplicate grader, substitute corpus or engine-agreement expectations were copied. No ready PR or full issue completion is claimed.


### paudley — 2026-10-05T21:41:57Z

Integrated qualification remains incomplete at signed, pushed head `9048c6b3a19be862e4ac092d9e1453f2d0d8ae54` (tree `2354614d15847b2f068091c692b3d45464ec6106`). The source is clean and all 26 owned source hashes remain unchanged.

The existing native gate payloads passed: 21,100 workspace tests/doctests, zero failures, 36 existing ignores. Both released Shapes/Validate additive API comparisons passed 196 checks with 58 skips each. The existing 32-package release-WASM Cargo build and default Python suite passed; Python reported 3,910 passes, one skip and four expected failures.

The remaining source checks now passed on that same source: metadata regeneration (including all generated projections and 35 recipient-license profiles), mdBook rendering, workspace rustdoc with warnings denied, and the full existing conformance matrix. That matrix reports 15,390 passes, 25 ledgered exceptions and zero failures across 29 rows. These existing suites do not substitute for the required community profile corpus.

The optimized WASM package capture **failed** with exit 1. The unchanged shipping build inputs (+simd128, warning denial and path remapping) compiled successfully, using two ordinary Cargo jobs and inherited PATH. Cargo's selected non-test `purrdf_wasm` artifact record had `fresh: false`, but named an output outside the requested private capture directory. The [existing containment guard](https://github.com/Blackcat-Informatics/purrdf/blob/9048c6b3a19be862e4ac092d9e1453f2d0d8ae54/scripts/build-private-wasm.py#L41-L44) correctly refused it with `Cargo ignored the requested private WebAssembly target directory`.

The failed compiler JSON is preserved as a task-owned artifact with SHA256 `eeb6d0581f5b06875bb03f250a498868ea4a93c1717ac79c6695a7207df80bc1`; the failure log has SHA256 `899efb359a954d5306b0512f87a6b7fd201bdd2d11a9caa7aaf65c2dd90440e2`. No guard was weakened, no shared artifact was substituted, and no retry or toolchain/cache/configuration investigation was performed. Compilation alone does not prove the package or ABI gate. wasm-bindgen, optimization, postlink, npm interface checks and identity ABI checks did not run. The ABI launch is held because it uses the same private-output containment mechanism. Focused unique-WASM execution also remains unrun; no general WASM semantic tests were added.

Full acceptance is **NOT MET**. The reviewed community kit/corpus still needs a durable admitted source and all 64 SHACL cases/115 applicable profile executions with independent exact report grading. The qualified final native XPath dependency and its reviewed inner-query capture repair must be integrated and requalified. The unrun downstream host checks and final coordinated native cost evidence remain open. This is an honest qualification checkpoint, not a completed issue or ready PR.


### paudley — 2026-10-05T23:22:17Z

Signed native composition checkpoint is pushed and read back exactly at `08c399c153c553b75c70904909bdea108a159a46` (tree `c8fa82723e6bbf524b2e9db46871cedcec3afec7`, verified signature). The normal dependency merge is `9acb8406a9a017dfef59c9e05da370b0b1e11278`, retaining the exact dated `FunctionAdmission` capsule alongside the incoming `FunctionOperational` classification and unchanged closed protocol status mapping. Both merge and test commits passed normal hooks; no verification was bypassed.

The two test changes cover the actual shared complete-report door under both dated profiles and the existing canonical worker primitive. `someValue` discards an earlier ordinary missing-function failure only when a later value conforms; without that value the failure remains. Actual MatchSteps0 stays a typed ResourceRefused despite the later conforming value, and current higher limits recover. The worker neighbor pins the earlier opaque host's operational type/code and proves a later admission capsule cannot replace it.

Actual qualification:99 focused native tests passed (13 dated execution,17 native XPath,8 shared complete reports,45 function tests,10 diagnostic-policy tests,6 private ownership/reducer tests), zero failures. Affected Shapes/Validate/Eval all-target warning-denied Clippy, all three warning-denied Rustdoc checks and formatting passed. Earlier compiler/lint/stale-assertion failures are retained separately with their exact source/log hashes; they are not relabelled passes. All new semantic testing is native Rust; no WASM semantic case, host runner, capture guard or wrapper changed.

The incoming #406 checkpoint has focused/API qualification; its final full gates/review/integration remain pending with its owner. The prior9048 whole-native/release/default-Python/API/metadata/conformance receipts remain historical, not a whole-source qualification claim for this checkpoint. Landed main synchronization and affected requalification follow this clean checkpoint.

Full #402 acceptance remains **NOT MET**. The live #384 recovery handoff still owns the exact independent grader, community data/catalog/licensing and Rust runner; the named source homes are absent from landed main87382f71889fb3fa3436e1404ed669cffb696a2c, with no kit PR or new durable handoff. The prior reviewed22cebc8d… inventory identity differs from candidate9112e6f6… and requires owner reconciliation before integration. All64 unique SHACL cases and REC57+WD58=115 actual independently graded executions remain mandatory. No protected recovery source was copied or changed, and no engine-agreement oracle or replacement grader was introduced.

The prior private WASM package capture remains a failed containment admission. Downstream package/interface/ABI checks remain unrun and unqualified; its guard and wrapper are unchanged. There is no ready PR, closure or full completion claim.


### paudley — 2026-10-05T23:42:55Z

The normal current-main synchronization is signed, clean, pushed and exactly read back at `da0745692668f21b03497670f297dd325a862d1e` (tree `74eafecf8fc362bcc0b3d95ff647785aed86e45d`, verified signature). Its parents are the qualified native composition checkpoint08c399c153c553b75c70904909bdea108a159a46 and landed main87382f71889fb3fa3436e1404ed669cffb696a2c. The merge had no conflicts and passed normal hooks.

Actual post-integration qualification passed104 focused native Rust tests:5 existing normalization cases plus99 dated/native SHACL, shared complete-report, function, diagnostic-policy and ownership/reducer cases. Zero failures. Shapes/Validate/Eval all-target warning-denied Clippy passed. The affected package sources are byte-identical to the prior warning-denied Rustdoc-qualified checkpoint; that documentation evidence is retained by exact source parity, not claimed as a new post-main invocation. Source hashes stayed unchanged throughout the reader, and normal push/readback completed.

The reviewed community dependency is still undelivered. The live #384 handoff assigns the single exact grader, kit, catalog, licensing and source corpus to its recovery owner. Those exact homes remain absent on landed main, no kit PR has supplied a reviewed identity, and the prior review-versus-candidate inventory mismatch is still unresolved. The required64 unique SHACL cases and REC57+WD58=115 independently graded executions remain **NOT MET**. No protected recovery source was copied, no replacement grader/oracle was invented, and no ready PR or closure claim is made.

The prior local private WASM package capture remains a failed containment admission; downstream package/interface/ABI checks remain unrun and unqualified. Guards and wrappers were not weakened or rewired. Root-owned #406 final qualification/integration also remains a separate prerequisite for final acceptance.

The source is now parked clean with these acceptance blockers explicit. Earlier passing whole-source9048 receipts and original failing artifacts remain preserved under their original identities. No toolchain, host, cache or global-setting investigation/change and no generic WASM semantic testing was added.


### sanmai-NL — 2026-10-06T06:28:38Z

@paudley This likely solves a real production problem for us. There are incompatibilities between 1.1/2017 and 1.2 that cause failure to apply validation, and in practice our SHACL Shapes Graphs are still 1.1.

