# Logical capacity census — current implementation checkpoint

The draft was applied after parent confirmed full gate 15630 terminal 0. The
patch file was removed; capacity.rs is the sole implementation. No alternate
runner, target directory or compiler setup was introduced.

The initial seeded controls passed 5/5. Initial strict compilation failed on
14 ordinary lints; the corrected strict retry passed. The unchanged helper gate
failed with 46 candidate logical-capacity sinks and two repeated visitor bodies
(logical-capacity-census-gate.log). These are candidates, not 46 proven runtime
defects. Stored-ID reconstruction, fixed wire constants, emitted-byte assertions,
admitted regex offsets, physical allocation and shipping fixture helpers require
individual source dispositions. No function/message allowlist was added.

Root review identified three necessary repairs: unknown bounds cannot prove a
NonZero slot, all compound mutations must invalidate proof, and imported
associated-operation aliases must retain the full parent path. All are corrected
and seeded. Runs 12984 and 13371 passed 5/5 after these changes. The actual
Datalog limit:u64 parameter proves its width explicitly; an unknown or u128
parameter does not. Rebinding removes that local width evidence.

The shared shipping_visits declaration at rules.rs now owns production-only
visitor dispatch. Capacity function/impl bodies share one declaration, and
Layout retains the identical cfg predicate and original syn traversal. Strict
run 84374 passed on that change; the later typed-parameter proof still needs its
final strict retry. No whole-gate or whole-issue pass is claimed.

## Historical unapplied draft

The concrete patch is `logical-capacity-census-draft.patch` in this directory.
It has not been applied, compiled, or run against the repository. The executing
workspace source and ledger are unchanged. Standard rustfmt parsed/formatted the
new Rust text using stdin/stdout only; `git apply --check` accepted the patch
against the current source. Those checks are not Rust type/lint/runtime proof.

Five eventual paths:

- `crates/helper-census/src/capacity.rs`: one detector home, local facts and
  function aliases, dominating slot-check proof, macro transcriber handling,
  original-producer and physical/invariant neighbours, source-walk/gate seeds.
- `main.rs`: declare that module.
- `normalize.rs`: admit its one rule fingerprint.
- `source.rs`: invoke the rule once for each file reached from a shipping root.
- `helpers-ledger.toml`: one enforced logical-capacity job at the original
  CapacityError::check/slot home, no exceptions or forbidden variants.

The source integration deliberately uses `walk_file` rather than `scan_rules`:
the latter also parses out-of-line test modules not reached by production.
Existing module discovery, path exclusion and cfg-test reachability are reused;
the detector handles inline test cfgs without a second file walker. Tests seed a
private cfg(test) module, a production any(test, wasm32) module and an integration
test root, then require exactly the two production violations to reach both the
Rust check and emitted index job. Correction must produce copies=0, and malformed
source must still fail the original walker. No runner, Makefile, compiler, cache,
target-directory, Python checker or dependency change is present.

The draft inspects direct and local/alias capacity sinks; original native mint
vocabulary, u32 narrowing and count/width assertions; explicit error branches;
and parsed or conservatively reported opaque macro transcribers. Names/messages
alone cannot grant an exemption. Slot proofs are operand-specific, follow `?`,
do not escape a conditional, and are invalidated by assignment/mutable borrow.
Tests mutate or delete the guard, change its operand and insert a lossy cast.
Generic resident physical allocation controls and fixed array/layout invariants
are separate accepted neighbours.

This remains a bounded syntactic detector, not whole-program Rust type analysis.
The draft's conservative producer-name set can expose valid readback invariants:
for example turtle_render::find_iri remints an existing local TermId in its
0..resident term_count loop. The actual census inventory must be read after
application and such cases given a narrow structural or production disposition;
do not add a function/message exception merely to get zero. Cross-file unknown
wrapper inference is not claimed. Current manual site review and actual public
boundary tests continue to qualify semantics alongside the structural rule.

Application order, only after root confirms the active full gate terminal:

1. Apply the patch to the existing checkout and inspect the five-file diff.
2. Remove this draft patch once applied; keep this task receipt updated, leaving
   only the actual Rust module as the implementation home. No standalone draft
   Rust file, temporary runner or alternate checkout has been created.
3. Run the seeded Rust controls and affected strict compilation. Resolve actual
   type/lint errors without allowances or bypasses, then inspect real census
   findings and run the unchanged helper gate to prove enforced registration.
4. Record exact failures/passes and independent source/gate-wiring review. No
   census or whole-issue acceptance is claimed by this draft preparation.
