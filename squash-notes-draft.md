Prepare schema-invariant cardinality contradictions once per class

Final notes for published41a224bd0. All48 hosted checks succeed with4 expected
skips. Independent final feedback debt is PASS with no pending checks or findings.

Closes #499

## What changed

Prepare schema-invariant cardinality contradictions once per class, then borrow
the compiled bounds when current individuals are checked. This removes repeated
bound construction without treating an impossible class as an inconsistent
ontology until it is inhabited. Object/data restrictions, qualifiers, exact
datatype cardinalities, equivalence and inherited constraints share the existing
reasoning and range homes.

Every new prepared clash records an independently checked schema derivation and
the current type/existence support actually needed by that contradiction.
Asserted, entailed and existential support, equality/nominal/successor and branch
cases retain their original laws. The proof checker consumes finite support
records instead of trusting a cached verdict or rerunning the reasoner. Generic
legacy proof trust accounting remains explicit; this does not claim the broader
proof-architecture work of issue501.

The preparation belongs to its Kb/source/revision and exact restriction/qualifier
identities. ABox changes, retraction/purge and schema revision reject stale proof
bindings. Resource exhaustion and interruption remain typed incomplete outcomes;
successful retry is supported. Products, exact-range temporaries and support
records admit their actual storage before growth and retain the original owners
through publication, clone/extraction and destruction. Public reasoning limits,
statistics, stop controls and proof/check consumers use the same implementation.

New prepared proof/refusal paths use the new trace/service identity; ordinary
legacy identities and the original frozen seven-case prefix remain byte-identical.
Nine checked cases extend the shared16-case native/C/Python/WASM corpus.

## Commits squashed (4)

- `41a224bd0` Clarify the prepared support tableau arm documentation
- `b40c598a6` Integrate the native Sparrow workspace into prepared schema reasoning
- `b7195b537` Move prepared-schema ownership tests after production items
- `d5ee0c862` Prepare schema-invariant cardinality contradictions once per class

## Files

```
bindings/python/tests/test_entail_reasoning.py     |    8 +-
 crates/entail/README.md                            |   47 +
 crates/entail/src/interner.rs                      |   14 +-
 crates/entail/src/owl_dl/bounds.rs                 | 1142 ++++++++
 crates/entail/src/owl_dl/clause.rs                 |  114 +-
 crates/entail/src/owl_dl/concept.rs                |    7 +
 crates/entail/src/owl_dl/data.rs                   |   37 +
 crates/entail/src/owl_dl/graph.rs                  |    4 +
 crates/entail/src/owl_dl/hyper.rs                  |  372 ++-
 crates/entail/src/owl_dl/mod.rs                    |   61 +-
 crates/entail/src/owl_dl/parser.rs                 |   20 +
 crates/entail/src/owl_dl/proof.rs                  |  380 ++-
 crates/entail/src/owl_dl/proof/schema.rs           |  225 ++
 crates/entail/src/owl_dl/proof/schema_tests.rs     |  240 ++
 crates/entail/src/owl_dl/support.rs                |  288 ++
 crates/entail/src/reasoner/certificate.rs          |   21 +-
 crates/entail/src/reasoner/mod.rs                  |   98 +-
 crates/entail/src/reasoner/proof.rs                |  108 +-
 crates/entail/src/reasoner/proof/current.rs        |  337 +++
 crates/entail/src/reasoner/schema_tests.rs         |  829 ++++++
 crates/entail/tests/schema_cardinality_boundary.rs |  127 +
 crates/hash/README.md                              |    2 +
 crates/lex/src/walk.rs                             |   41 +
 crates/rdf-capi/src/entail.rs                      |   26 +-
 crates/validate/src/regime.rs                      |   62 +-
 crates/validate/src/regime/schema_vectors.rs       |  383 +++
 crates/validate/tests/fixtures/dl-proof.vectors    |  578 ++++
 crates/xsd/src/bigint.rs                           |   17 +-
 crates/xsd/src/bigint/storage.rs                   |   19 +
 crates/xsd/src/exact/decimal.rs                    |   45 +-
 crates/xsd/src/exact/integer.rs                    |    7 +
 crates/xsd/src/range.rs                            | 2755 ++++++++++++--------
 crates/xsd/src/range/storage.rs                    |  416 +++
 crates/xsd/tests/range_storage.rs                  |  246 ++
 34 files changed, 7812 insertions(+), 1264 deletions(-)
```

## Conflicts

Actual main a77b743 (#508) conflicted in the original exact integer/decimal homes.
Preserve its native-copy/arithmetic/render layouts and single kernels, adapting
the range owner through those same comparison/rounding bodies. Decimal exponent
preparation threads the selected original allocator into both digit groups and
quotient scratch; a513-digit unequal-scale regression asserts an independently
known empty interval and measures allocation-count/peak/complete release.

Remove the obsolete core boxing definition introduced by the automatic merge,
retaining main's shared lexical-home re-export and the preparation owner's
before-allocation admission/typed failure. No wholesale side selection, alternate
numeric engine, raw postconstruction admission or unsupported fallback is used.
Signed integration b40c598a6 has exactly the native/portable tested tree.

## Validation and integration applicability

Complete original issue qualification passed the unchanged mandatory full gate:
workspace/consumer strict lint, build, tests/doctests, hygiene/generated checks,
preserve-order consumer, kernel ring-fence and all32 release-WASM crates. Earlier
failed gate attempts remain failed historical records with their corrections.
Native independent proof/current-support/owner/refusal/revision/branch controls,
all262 OWL/four historical cases and RL negative controls passed without fixture
or ledger weakening. The original O3 two-axis preparation/instance matrix retains
all36 time/allocation/traffic/peak estimates; representativeR32/N64 cached18.55ms
versus uncached45.22ms is a local measured result, not a portability guarantee.
Actual optimized packaged runtime439 and original ABI suffix also passed.

The real main overlap was qualified through the affected family, not a broad
gate restart solely for branch age. Strict affected all-target lint, all1739
native/C ABI cases, all70 freshly rebuilt Python reasoning cases, shared-helper
and hash gates passed. Actual WASM XSD3/SPARQL2 frozen oracle/digest cases passed;
fresh optimized package/post-link/SIMD validation and all43 public entailment,
proof-checker, byte-golden and typed-refusal cases passed with no skip. The parent
read the terminal logs and updated the existing independent integration review
to PASS. Normal signing/hooks/push passed and source is clean.

The b40 integration candidate matched the tested a222610de483 tree. The current
41a documentation-only candidate is clean (051e1be4c59a7deac8ba0ce7a23d4172d1c38a87).
Production bodies remain unchanged; existing runtime qualification applies.
The b40 workspace failure was the phrase "this branch" in tableau documentation,
now corrected to "this tableau arm" with normal hooks and push. CodeQL's
independently adjudicated public-fixture false positive is explicitly dismissed
in GitHub with the posted rationale. Fresh41a hosted checks all pass, including
the workspace gate, native shards, public bindings, optimized WASM package,
conformance, cross-architecture and CodeQL. The prior b40 CodeRabbit review passes;
the latest prose-only delta was rate-limited despite its SUCCESS check, and is
covered by independent delta inspection. Old b719 CodeQL
aggregate FAILURE is a historical failure despite its five successful analysis
jobs, and is not represented as qualification. Current-head review debt and
binding hosted gates are separately satisfied; issue closure awaits integration.
Final feedback debt is cleared by the independent review-debt report. CodeQL
alert240 is legitimately declined: the ignored test-only fixture sealer writes
fixed public example.org proof/check golden content, not credentials or caller
input. The rationale is posted and the sole thread is resolved. The docstring
percentage heuristic is declined with posted reasoning; no concrete missing
public API documentation contract was identified. NEUTRAL check status alone
was not treated as review-debt acceptance.

## Goals, decisions and evidence

One original implementation is retained for every shared job. No external
runtime dependency, semantic Cargo feature, implicit vocabulary, chosen semantic
ceiling, changed upstream conformance input or waived required gate is introduced.
All original nine #499 families are implemented; no enhancement or required
behavior was silently declined. The original phase/completion reviews and the
affected integration review have independent PASS dispositions; final feedback
is cleared. Hosted acceptance is48 SUCCESS and4 expected SKIPPED, with no pending
or failed checks. The clean actual candidate remains051e1be4c59a, covered by the
qualified production tree and independently inspected prose-only delta.

The authoritative plan is
/home/paudley/Active/purrdf/.worktrees/499-entailment-prepare-schema-invariant/.stage/entailment-prepare-schema-invariant/plan.md.
The selected stage directory is
/home/paudley/Active/purrdf/.worktrees/499-entailment-prepare-schema-invariant/.stage/entailment-prepare-schema-invariant.
Its validation index, complete issue/review inputs, semantic/proof/O3/portable/
host receipts, original failed logs, exact integration assessment and normal
publication records are selected for final archive. CI/review writers have stopped;
durable capture is performed and verified by ghprsq before owned cleanup.

---

Defect-Class: none

