# G2 independent design review — 2026-10-08

Disposition: DESIGN PASS; implementation/runtime qualification NOT MET yet.
Read-only source review, no build/test/benchmark/forge action. The prior full
gate and hosted report precede G2 and do not qualify changed source or merge.

Read the accepted sole plan, remediation plan, G2 investigation, current source
diff and raw PR500 inline comment4215385143. Main-root baseline/goals and the
repository contract apply; current deficiency ledger has no emergency entry.
Reviewed source snapshot:

- seminaive.rs SHA256
  `88bb0bfad578aa403c3ef15d96894175c8803c0e708b421961498fcfea6dcac6`.
- seminaive/factors.rs SHA256
  `6eb4ce4ff8961ec758549a4826552cdd98075cced8f3b675727d6999d78afaad`.
- G2 investigation SHA256
  `e1f4144adfd3372a9288d5660785a1a152bc544d081b9a4eb9418089e0ebb24a`.

## Finding and chosen repair

The raw review's defect is valid: prior full_matches scanned complete growing
recursive factors before classifying delta tuples. Whole-rule delta eligibility
did not prevent full recursive reconstruction. The suggested all-delta-only
dispatch would remove that particular cost but abandon accepted partial-round
factor additivity. The selected mode-aware repair addresses the actual defect
without that scope loss. No blocking design flaw was found in inspected source.

New discovery uses the ordinary shared last-new-positive-atom decomposition:
Full before the atom anchor, Delta at it, OldOnly after it. Positive coordinates
are stable authored positive positions, independent of physical traversal order;
each tuple has one last-new coordinate and therefore one discovery anchor.
Factor projection/emission remains the separate first-new-factor decomposition:
Old before the factor anchor, New at it, Full after it. These two orientations
are compatible and must not be confused or mechanically made identical.

New is discovered first and filtered through the existing factor-owned negatives.
No actual New tuple anywhere returns before unrelated Full/Old context is built.
Actual productive factors determine necessary modes. An earlier productive
factor needs a later factor Full; a later productive factor needs an earlier
factor Old. A sole productive factor can stay New-only. Standalone Old matching
and New's disjoint atom anchors avoid unconditional complete reconstruction.
If a factor needs Full, that result replaces its temporary New relation.

JoinScan is shared matching state, not a second cyclic algorithm. Both cyclic
variable cursors and fully grounded source capture consume the same selection.
Uniform OldOnly therefore filters every cycle atom; it is not the previous
usize::MAX convention, which means Full. Ordinary hybrid callers retain their
DeltaPosition selection. Certified cycle/group membership remains unchanged.

Each relation owns source frames locally for the frozen round. Projection and
dual frontier entries borrow those frames. Existing global saturated height,
sum/lex threshold selection and authored-coordinate restoration remain intact;
mode reduction does not select a local arbitrary existential witness. Products
remain the necessary projected head combinations; no cross-factor intermediate
Cartesian buffer, persistent cache, memoized negative/guard, new lifetime or
dependency/feature was introduced.

Factor certification still excludes opaque body and negative callbacks and
couples negative outer variables. Ground negative checks retain their existing
precondition path. Matching negatives use frozen current-model state and shared
governors. Scheduled full-model/guarded callers and caller-thread barriers must
remain qualified through their actual public paths, not inferred from pure tests.

## Required qualification and risks

The investigation's planned matrix is appropriate and binding. Current two
initial regressions are necessary but cannot alone discharge G2:

1. Actual recursive chain plus independent ground precondition at32/64/128/256:
   complete planned/ForcedBinary facts AND authored canonical proofs, unchanged
   default limits, deterministic quadratic counter envelope and growth. Include
   an actual default-limit recursive CLI case; no raised limit or timing-only pass.
2. Partial one-factor heads with both authored factor orders, large independent
   existential relation and small delta: additive matching/storage and exhaustive
   small witness parity. Both factors productive must cover multiple insertion
   orders/delta ranges and exclude all-old firings.
3. Delta-addressable partition with no complete positive factor match and a
   negatively blocked New factor: prove no unrelated Full/Old context work or
   head firing. Whole-rule delta_can_match is only an over-approximation.
4. A certified cyclic factor must actually reach Uniform OldOnly and decomposed
   New, in both factor-anchor orientations, against exhaustive binary facts and
   complete proofs. Ordinary cyclic tests with all delta cannot prove this.
5. Partial mode frontiers need masking/saturation coverage or explicit evidence
   that retained existing differential fixtures traverse the changed partial
   branch. All-delta height/sum boundary fixtures alone do not exercise it.
6. Zero/exact/one-below productive partial admission and successful/refused
   worker1/4/32 parity: typed total refusal, no partial closure commit, stable
   counters. New discovery plus necessary context can change charge boundaries;
   audit affected reports/pins through their owning generator, never guess values.
7. Focused Datalog/all-target strict clippy, affected SHACL/entailment/public
   host paths, relevant WASM runtime/governor paths and original allocation
   controls. Root chooses proportionate final source-dependent qualification.

There is bounded repeated work when New discovery precedes a required Full
relation; the source correctly avoids retaining a second New relation afterward.
During construction the temporary New and new Full can coexist, so a claim of
zero duplicate peak storage would require measurement and is not made here.
Discovery and each required mode should be characterized by actual counters;
the header's historical 'each factor is evaluated once' wording should describe
the per-mode frozen-round behavior rather than imply a single physical scan.
No speculative cross-round cache is needed or authorized to erase that bounded
cost. Required full context cannot be dropped merely to improve a counter.

Design pass authorizes the coherent implementation direction; it does not close
the review debt, prove compilation/runtime, or replace fresh completion evidence.
GL owns source fixes/tests; root owns build admission, forge and integration.

## Coverage refresh after the added test-only matrix

Reviewed subsequent test-only source at factors.rs SHA256
`03eb10b48cbd0f7d6f7dff7afdc73a68ee546f882c350139a26c1a5e626250b7`;
seminaive.rs remains the source hash above. Production design is unchanged.
No build or runtime check ran during this refresh.

The masking fixture now compares exact candidate entries against ForcedBinary
for all 21 contiguous delta ranges, four insertion orders and four height
assignments, including u32MAX−1/MAX. Its specific b1 assertion is correctly
restricted to all-delta; partial ranges can change eligibility and the canonical
winner. Four negative coupling/local-scope fixtures now compare exact candidates
across all contiguous ranges and four insertion orders as well as complete models.
This closes the previously identified source-level partial masking/negative
coverage gap without replacing its exhaustive witness oracle.

The new hybrid fixture reaches shared Uniform OldOnly and decomposed New, both
factor-anchor cases and simultaneous anchors through every delta range/order.
Its addressable-but-unmatched new cycle edge checks that a thousand unrelated
existential rows do not get Full/Old reconstruction. The new 32-rule recursive
fixture checks productive partial exact/zero/one-below credits, complete facts/
proofs and success/refusal reports at one/four/32 workers. All remain written
coverage, not executed passes. Required actual native/WASM/public/CLI qualification
and report/pin audits remain open until root admits execution.

## Independent runtime/collector source refresh — 2026-10-08

Source soundness: PASS for these additions; runtime acceptance remains NOT RUN.
Reviewed actual four-path diff and investigation, with new files at SHA256:

- `crates/datalog/tests/rules_runtime.rs`:
  `5b3b486f7aba4589ffa8396e34daace82fac58d0a3fd776ee91f5de157fc173d`.
- `crates/cli/examples/rules_campaign.rs`:
  `6e2d23450ee48845292ccef0259e752530c3fc66f12ca415cdbc6e34b1614a5a`.

The fifth registered target-neutral runtime case genuinely enters productive
recursive partial rounds: a connected reach/edge factor and independent ground
condition. Its independently enumerated n16 pairs and exact derivation count,
input-plus-inferred fact cardinality, predicate/graph/rule and all three authored
source checks exclude missing pairs, extra facts and shortened proofs. Exact
credit replay compares complete sorted facts, derivations and budget to the
default execution. Zero/one-below demand typed JoinSteps refusal at ceiling+1;
they do not accept a partial closure. The expected credit amount is measured
from the actual default run, not fabricated from a revised pin. Native and WASM
must execute this same registered case before coverage is called qualified.

The collector's new focused mode shares the actual child capture and exact
fact/complete proof validator with the existing campaign. CLI argv leaves all
governors at their defaults. Original Rust generates n32/n256 input/rules once
and supplies identical paths to both binaries. Every distance>=2 pair has an
independently constructed proof using its unique final adjacent edge, recursive
reach prefix and ground condition, in authored order. Set equality plus line/
block counts refuses duplicates, omissions and extra proofs; exact diagnostics
and empty stdout remain required for successful output. A pre-fix unsuccessful
exit is retained as observation with partial artifacts unaccepted; fixed failure
is a hard campaign error. Successful pre-fix output receives the same validation.

Input and rules BLAKE3 receipts before/after each child are checked against
their generated identities before output acceptance. Binary identities are
bound at campaign start and checked after the full paired campaign. Fresh root
creation refuses overwrite; argv/status/load/time/RSS/stdout/stderr and failed
artifacts remain available. Recursive validator self-tests include accepted
blocks and missing/duplicated premise refusals. No second generator/parser,
cache, shipping dependency or silent fallback was added. The original three-
argument campaign still requires successful children through its shared wrapper.

No actionable defect was found in these additions. The two previously reported
source prerequisites (actual partial-factor WASM fixture and focused original
collector mode) are now delivered, not deferred. Compilation, strict clippy,
validator self-test, native five-case execution, actual WASM execution and paired
CLI outcomes remain required and NOT RUN. Full prior reports predate G2 and
cannot establish current runtime success. Qualification preparation is updated
to the delivered production entrypoints without prescribing an unrelated repeat
of all twenty measured cases.

Also read root260 `T3-integration-assessment.md`: fresh main's addition is the
exact additive hash source already qualified on the rules integration candidate
(root session77193 exit0). Existing hashing bodies/manifests/lock/toolchain are
unchanged; reuse of that exact source's 59 tests, strict clippy and WASM build is
sound for the additive base interaction. It does not discharge either pending
glossary full/render acceptance or G2's new runtime qualification.
