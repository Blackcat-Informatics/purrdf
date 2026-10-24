# Explicit bounded recursive SHACL semantics

## Current contract from the issue analysis

The latest issue comment (6231674813, October 17) supersedes blanket recursive
shape refusal as the final behavior. The published guard unit below remains the
safety baseline and no-selection behavior; it is not the completed issue.

1. Analyze the complete SHACL Core refers graph once at preparation. Include
   memberShape, reifierShape, someValue and targetWhere. Label negative and
   non-monotone edges, including qualified maxima, xone and disjoint siblings;
   resolve the exact targetWhere/count polarity from their evaluation clauses.
   Reuse the existing native Tarjan SCC implementation.
2. Require explicit Lfp or Gfp selection for recursive graphs. Absence of a
   selection produces a structured cycle failure. Acyclic graphs retain their
   answers. Reject non-stratified recursive negation/non-monotonicity in both
   modes, retaining the original cycle and operator in the failure.
3. Parse each cyclic shape once and reference it by native identity. Extend the
   existing model, planner, validation and report path; never add a second
   validator, empty recursive shape or skipped edge. Use one SCC evaluator with
   lower strata frozen. Only the initial pair assignment differs between modes.
   Bound pair admission and iteration work, and propagate exhaustion as failure.
4. Preserve physical-stack and logical-depth safety during parsing, preparation,
   validation and expression re-entry. Close the known lowering gap. Reuse the
   ShEx SCC/polarity design and address relevant ShEx stack safety under this
   issue; do not invent a second SCC implementation.
5. Select GFP explicitly for the recursive bundled shacl-shacl well-formedness
   validation. Record the chosen semantics consistently in actual host options,
   prepared products/request configuration and report provenance.
6. Port the actual sheval catalog with per-mode expectations, the missing-name
   regression in the comment, shared-DAG/positive-or/lower-negative-stratum
   controls, random recursive/negated catalogs and a valid LFP/GFP duality
   property. Verify the source inventory rather than repeating the comment's
   stated count: its enumeration may contain eighteen cases rather than seventeen.

Deliver bounded compiling slices: dependency analysis/structured refusals;
identity-based cyclic parsing; solver and production/report integration;
host/options/product propagation; complete native/portable acceptance and
delivery. Keep all required scope here until complete. No partial PR or closure.

Issue 570 is separately owned. Its W3C re-vendoring, EARL generation, scoreboards
and upstream report submissions are excluded from this branch. Preserve its
owner's work; use the current frozen suites for applicable local acceptance.
An external sheval runner or WG message is not sent without explicit scope.

The sources are the issue comment and these distinct papers. Inspect their
actual definitions and the production constraint clauses when deriving polarity
and duality expectations. The published KR citation is authoritative for the
companion paper; the initial analysis read its arXiv v1 preprint, so theorem
numbers must be checked against the published edition before being cited.

- Shqiponja Ahmetaj, Iovka Boneva, Jan Hidders, Maxime Jakubowski,
  Jose-Emilio Labra-Gayo, Wim Martens, Filip Murlak, Cem Okulmus,
  Ognjen Savković, Mantas Šimkus, and Dominik Tomaszuk. “sheval: An RDF data
  shapes evaluation tool and test-suite for recursive shapes.” 2nd International
  Workshop on Data Management for Knowledge Graphs (DMKG 2026), 2026.
  https://labra.weso.es/pdf/2026_sheval_DMKG26.pdf
- Shqiponja Ahmetaj, Iovka Boneva, Jan Hidders, Maxime Jakubowski,
  Jose-Emilio Labra-Gayo, Wim Martens, Fabio Mogavero, Filip Murlak,
  Cem Okulmus, Ognjen Savkovic, Mantas Šimkus, and Dominik Tomaszuk.
  “Common Foundations for Recursive Shape Languages.” In Proceedings of the
  23rd International Conference on Principles of Knowledge Representation
  and Reasoning (KR 2026), pp. 2–13. IJCAI Organization, 2026.
  https://doi.org/10.24963/kr.2026/1

## Published safety baseline

Commit 7859b606c is pushed through normal hooks. Initial production compilation,
eleven cycle cases, shared references, all four deep parser families, actual
ordinary/complete/conformance refusal and recovery, existing function/SPARQL
re-entry limits, strict/helper checks and six rendered Chinese book gates pass.
This preserves a safe baseline while the explicit semantic modes are completed.

The requested outcome is that recursive SHACL cannot panic the process, with a
documented engine behavior. The source contains two cycle-to-empty-shape
substitutions and a nested-property cycle skip. Property constraints are parsed
before the existing nested-property guard, leaving a re-entry gap. Long acyclic
shape/path/expression expansion also lacks physical stack admission. Validation
has a logical expression-depth ceiling but no shared native stack check.

Use the existing stack crate and parser in-flight owner. No new runtime package,
feature, toolchain/cache setting, model service, subagent or timing acceptance.
Main stays protected. Preserve other delivery branches, including the published
error migration, without introducing a dependency on that unmerged protocol.

## Behavior and sources

SHACL 1.2 Core, dated October 14, 2026, sections 6.6.1 and 6.6.3 explicitly permit
recursion/resource failures and leave recursive-shape semantics to processors:
https://www.w3.org/TR/2026/WD-shacl12-core-20261014/#shapes-recursion

The initial guard-only plan used the following failure policy; the current
contract above extends it to explicit Lfp/Gfp modes rather than rejecting every
recursive graph. The referenced sheval paper distinguishes fixed-point policies, errors and
engine-dependent answers. PurRDF must not label its current finite expansion
with empty backedges as a faithful least/greatest fixed point. Adopt an explicit
failure policy for cyclic shape expansion. Preserve ordinary acyclic validation.
Depth/physical exhaustion must be a failure rather than conforms/nonconforms.
No fallback shape, skipped recursive edge, swallowed error or panic-catching
substitute for preventing stack overflow is permitted.

## Task 1: Guard actual parser expansion

Add a focused public regression matrix for named/blank self cycles, mutual
cycles, logical and qualified/list shape edges, nested properties and mixed
property/node/expression re-entry. Keep shared acyclic references as positive
controls. Inspect the real shared parser and centralize its expansion admission
and cleanup rather than duplicating guards. Remove both stand-ins and the edge
skip. Admit physical stack before entering deep frames, and preserve first
unrelated parse errors. Include paths and expressions where they re-enter the
shape parse. Compile the affected production slice, then run these tests only.

## Task 2: Guard validation and define failure propagation

Apply the shared physical stack admission at the actual recursive traversal and
expression boundaries, retaining the existing logical-depth refusal. Exercise
deep acyclic/programmatically built shapes and the filter/exists cycle on bounded
native stacks, plus ordinary nested/foreign-node validations. A returned failure
must stay a failure through public preparation, report and conformance entry
points, including host adapters. Do not turn an operational refusal into a
constraint violation or successful report. Resolve any introduced error-type
adaptation in real callers; do not depend on the unmerged error migration.

## Task 3: Document and deliver

Document the cycle-failure policy, depth/stack refusal and acyclic behavior in
native public guidance and relevant translated guidance, citing the specification
and sheval paper. Add no issue numbers to shipping docs/source. Regenerate
affected projections through existing generators. Run focused strict/helper and
applicable scalar/SIMD/host controls, then the required final gate once fixes
settle. Reuse valid unchanged evidence. Obtain actual hosted source review;
owner reports are not independent reviews. Commit/push normally, publish the
complete PR, satisfy hosted checks, integrate with ghprsq, verify closure/mirror
and remove the completed branch/worktree. Update REWORK_LIST.md at transitions.

## Current evidence limits

The parser cycle matrix, all four deep parser families and shared-reference
controls pass. Actual conformance, ordinary prepared validation and complete
reporting return a stack refusal and recover after the reserve is released.
Existing function and SPARQL re-entry depth controls pass.

Source review also found that stage-0 lowering recursively walks already-built
shapes before the new runtime guard. PreparedShapes::new invokes that lowering
in an infallible constructor. A shape loaded on a larger stack can therefore
reach a smaller-stack preparation; this gap remains part of Task 2 and blocks
issue completion. Finish the bounded parser unit through normal publication,
then make lowering admission and its real preparation consumers fallible without
a panic, partial successful preparation or synthetic class catalog. Do not start
a whole-tree API rewrite without compiling coherent private slices.

This is the initial source-grounded plan, not implementation or qualification.
The complete issue has two sentences and no comments; brief reports complete
issue/comment retrieval and no prior branch/commit for this issue. The referenced
paper was downloaded to the repository recovery Stage directory and read. No
baseline/general test has been run and no independent approval is claimed.
