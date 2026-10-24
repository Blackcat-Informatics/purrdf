# PR 691 qualification

Published source: ea133b288, pushed with normal hooks; source worktree clean.
All selected local checks and required hosted checks pass. Independent review and
targeted risk reconciliation are complete, with no actionable finding and no
unresolved thread. Merge readiness is PASS. The backlog goal remains user-paused.

Archive mapping: frozen byte-identical copies of qualifying .log files are under
acceptance-evidence/, with .txt suffixes, indexed by acceptance-evidence-sources.txt.
Imported review feedback excludes only a mechanical attribution banner as
documented in review-evidence-projection.md; technical feedback and metadata are
unabridged. No source, test outcome or publication guard was changed.

## Authorized boundary

The user ordered: "PR the work - make an issue for any regressions, work through
reviews on this PR and merge it. You are forbidden from expdanding the scope of
the PR that you are now filing."

[publication-scope.md](publication-scope.md) bounds this delivery to the behavior
published at 2e42e8792 plus compatibility/review repairs. Full explicit LFP/GFP
semantics and the remaining issue 569 contract are not implemented here. Issue
690 owns the regression inventory; neither issue is automatically closed.

The original plan is plan.md. Its earlier no-partial-publication instructions
are superseded by the quoted user direction. This is the queue delivery path;
reuse the independent PR review instead of importing a Stage review panel.
Historical qualification, limits and failed attempts are preserved in
[qualification-history.md](qualification-history.md) and their referenced logs.

## Evidence and applicability

| Criterion | Observed result and scope |
|---|---|
| Main integration | 4397a2336 normally merged origin/main d26bc6e64. Its delta was README prose. Last fetch showed no later base advancement; refresh at final merge. |
| Commit verification | Normal hooks and pushes passed through 2443eec1b. Every Cargo invocation, including Node/Python subprocesses, uses the configured wrapper with its supported CARGO_REAL pin. No verification escape hatch was used. |
| Core identity/varint | All 1398 core library controls passed in shared-catalog-core-shapes-units.log. The two core source inputs are unchanged since that run. Frozen framing, digest and Vec/inline varint encodings pass. |
| Native Shapes library | All 989 pass in integration-repair-native.log at d0ef5e401 plus test repairs. The final LinkML change additionally passes all 36 focused library controls in final-linkml-library.log and the current projection/ontology checks below. |
| Integrations | The b049fa51c full integration run had 90 target results, 1185 passes and 101 failures in 14 targets. Every failing target subsequently passes via focused repairs: integration-repair-native.log, integration-pins-native.log and projection-pins-final.log. Unchanged successful targets are reused with the source-path assessment below. No fresh whole-suite pass is claimed. |
| Projection and ontology | projection-pins-final.log passes all 139 emitter projections and 29 ontology surface controls on the final source. Complete referenced bodies, sharing, exact loss locations, runtime schema verdicts and the explicit recursive refusal/finite neighbor remain asserted. |
| Product refusal and spelling | integration-pins-native.log passes all 65 product-refusal and 41 SPARQL-expression controls. Mutation probes now target the actual term tag and single retained expression occurrence; all refusal dimensions and adjacent valid controls remain. |
| Allocation growth | All 12 change-path allocation and 7 catalog controls pass in compat-shapes-integrations.log. Admission is 230 allocations versus the old 293; cold static-root preparation is 18 allocations/3106 requested/0 retained/1674 peak bytes at 1/8/32 definitions. Parser/plan/core inputs are unchanged by later projection/test fixes. Original ceilings were not raised. |
| Incremental/carried analysis | All 18 footprint and 10 carried-analysis controls pass, including shared reading diamonds and cyclic read routes. These are read-dependency controls, not fixed-point conformance evidence. Later projection/test changes do not alter their production paths. |
| Product/census | All 7 ordinary determinism and 28 census controls pass across the full integration and corrected-probe runs. Stage 81d0d3901d111fcce14adc418560eadfcdf9bc6f65acb3df831ce11908cef6a5 and the supported 5608-byte current fixture are preserved. The frozen format epoch artifact is unchanged. Later changes do not alter the declarative model, codec or class-analysis derivation. |
| Independent GraphQL | integration-graphql-oracle.log passes GraphQL.js 16.14.0: list probes, exact/coercion probes, located losses, value codec, outputs, temporal/value-shape probes and reverse import. Final LinkML/test changes do not alter the GraphQL path. |
| Independent TypeScript | final-typescript-oracle.log passes compiler 7.0.2: 50 exact probes, optional/null/undefined controls, all located profiles, list/temporal/value-shape controls and verified reverse import on 2443eec1b. |
| Strict compiler/lint | integration-pins-strict-qualified.log passes Shapes/CLI all-target Clippy with warnings denied on the final source. |
| Constructor proofs | capacity-base/current.json and matching constructor lists show identical 173-member source membership. Only core identity.rs and pack/bits.rs fingerprints changed; no capacity authority, constructor or sink changed. Forty aggregate bindings were refreshed to source:b4c07f030aea92a9; authority pins and sink pins are unchanged. final-helpers-hygiene.log passes all 87 enforced jobs, 121 distinct rows and 150 unique prefix-free hash domains. |
| Actual CLI | final-shacl-cli.log passes all 29 native CLI controls. |
| MSRV and WASM schema oracles | Exact final-head Rust 1.98 MSRV, WASM build/execution/package/schema oracles all pass in CI run 40041027523. |
| Hosted CI/review | Final-head CI run 40041027523 is completed/success, bound to ea133b288. hosted-checks-ea133b-final.txt reports state: pass for the full PR rollup, including all seven assembly configurations and aggregates, Python, workspace, integration, conformance and cross-arch gates. Optional profiles are skipped and the informational CodeQL context is neutral; actual Rust/language analyses pass. Final independent review covers ea133b288 with no actionable comments. All four threads are resolved with complete pagination. General risks were independently withdrawn after source/baseline reconciliation; see independent-risk-reconciliation.md. |
| Final LinkML reference carriers | ea133b288 passes the unchanged official LinkML 1.11.1 oracle, all 41 LinkML library tests, 139 projection tests, strict Shapes all-target Clippy and full helper/domain hygiene. slot-carrier-validation.md indexes the exact final logs and reuse limits. |

## Source changes and reused checks

96b0ba3d3 fixes the target-driver model fallback, private restore component/target
contexts, parser inspection fixtures, duplicate planned-slot lookup, shared test
term helper and audited constructor bindings. b049fa51c restores existing lint,
legacy query-admission, schema-import and CLI/assembly callers.

d0ef5e401 removes a lone allOf wrapper around a value reference. The official
GraphQL oracle reproduced an unlocated wrong-kind acceptance before this repair
and passes after it. The same repair removes the wrapper allocation. Original
property references now retain their loss comments.

2443eec1b repairs test body/property inspection and classifies a shared pure
object choice as a LinkML class expression, matching its inline carrier rather
than discarding object fields in a scalar type. Existing failure/semantic
assertions remain; alias/body and loss pins follow actual reference locations.

Only projection consumers changed after the last complete integration attempt.
The final 139 projection and 29 ontology tests cover that path. Parser, validation,
rule execution, binding, footprint, allocation, identity and product paths are
unchanged; their passing controls are reused. Corpus differential counts and
ledgers are unchanged. Required hosted checks still must pass.

## Remaining steps

- 0b115e5bc fixes review 4364860635 and five stale assembly documentation cells.
  All 38 LinkML library tests and strict Shapes all-target Clippy pass. The bot
  accepted the repair and resolved the fourth thread (review-choice-ack.txt).
- ea133b288 fixes the hosted LinkML oracle regressions. Named TypeDefinition
  dropped scalar/object and mixed-enum alternatives, and the official 1.11
  generator closes a named choice wrapper over its empty local attributes. The
  repair borrows original slot bodies at reference boundaries, preserves sibling
  conjunctions, guards cycles/stack margin and bounds anonymous expression count
  by source bytes. The unchanged official oracle and all 41 library/139 projection
  controls pass. Final lint, helper checks, normal hook and push pass too.
- Current-head CI and independent review are complete. Main was refreshed and is
  still d26bc6e64, already incorporated in ea133b288. The clean qualified candidate
  remains 0ec565f87b2d3be0bdb92d12f776a47dd709faf8, with no additional code beyond the
  tested branch. Source is clean and fully pushed. Review debt and the completion/
  merge-plan assessments are PASS under the frozen queue-delivery boundary.
- Merge only with ghprsq and this exact Stage directory, verify archive/landing,
  then inspect supported cleanup without disturbing donors or sibling worktrees.
