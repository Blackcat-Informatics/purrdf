# Independent Task 1 review

**PASS.** The settled Task 1 source implements the accepted native generator,
versioned sampling/byte law, independent parsed-graph constraints and focused
qualification. This passes Task 1, not the complete issue or production lane.
Reviewed applicable repository laws, the sole plan, plan-review.md,
published-profile-notes.md, T1-implementation.md, all ten changed/new task files,
unchanged Cargo.lock, and the actual settled test/clippy/hygiene/identity logs.
The reviewer did not implement this source. No build, benchmark, Java/model
invocation, forge request, Git mutation or source change occurred during review.

## Source and runtime findings

- `src/lubm/mod.rs:33` validates positive count, checked exclusive range end,
  absolute query/fragment-free ontology/base IRIs and compatible distinct schema
  and document identities before output creation. `src/lubm_main.rs:10` admits
  exactly the six required options, rejects duplicate/unknown/missing/malformed
  operands and uses a failing exit status with an actionable diagnostic. Public
  `Spec` fields cannot bypass admission: `output.rs:49` revalidates them.
- The shared SplitMix home, university-local seed/index state, unbiased inclusive
  draws and distinct sampling (`mod.rs:70–94`) match LUBM_PROFILE.md. Bounds and
  integer rounding are explicit. There is no time, locale, output-path or random
  table-order input. The checked range law intentionally rejects index MAX with
  count one; MAX-1 with count one is in the executed boundary matrix.
- `generate.rs:74–250` implements all captured published constraints: department
  and staff ranges; exactly one full-professor head; disjoint faculty ownership
  of 1–2 courses at each level; research groups; both student/faculty ratios;
  department membership; bounded TA/RA populations and distinct TA courses;
  professor advisors; student course loads; faculty publication ranges and
  graduate coauthorships; faculty three degree relations and graduate one.
  TA capacity may narrow its upper bound but cannot silently violate its lower
  fraction. Publication ownership counts original faculty publications while
  permitting additional graduate authors. Degree universities are explicitly
  typed external references, separate from generated university capacity.
- Property directions, canonical instance names, metadata, caller-selected schema
  and base, and typed literal/IRI emission follow the declared native profile.
  Student, Professor, Chair and Faculty membership are not asserted to manufacture
  inferred query answers. Escaping, RNG, hashing, JSON records and RDF parsing use
  their existing homes; no UBA implementation, Java process, new external or
  shipping dependency, semantic feature or duplicate kernel was introduced.
- `output.rs:49–118` requires a fresh directory and create-new payloads. It checks
  emission, flush and sync, then rereads actual nonempty payload bytes for checked
  lengths and BLAKE3. Only after every payload succeeds does it write/flush/sync
  the pending sorted configuration-bound receipt and publish receipt.json.
  Errors propagate and retain owned failure evidence. Existing output/sibling
  bytes remain intact. This is checked successful generation, not a cross-process
  transaction or power-loss recovery protocol; neither is Task 1 acceptance.

## Independent acceptance and evidence

`tests/lubm_cli.rs:72–478` parses actual CLI-produced N-Triples through the
production reader, independently indexes RDF statements and derives every
cardinality/relationship constraint above. It checks typed and distinct references,
degree origins, faculty ownership/coauthors, graph/document metadata, naming,
valid IRIs/literals and duplicate statements rather than trusting generator
counters or assignment helpers. Receipt checks independently compare actual file
inventory, configuration, sorted names, lengths and content hashes.

The settled log records actual execution of the named seed/index/count matrix
`(0,0,1)`, `(1,7,1)`, `(MAX,MAX-1,1)`, `(42,2,2)` and eleven planted graph
refusals (`lubm_cli.rs:481–559`). Ten controls remove a real required relationship
and one injects forbidden inferred Student membership; each requires the planted
statement to exist and the independent oracle to refuse it. The separate-process
identity tests (`:562–609`) cover repeated paths/locales, university-alone versus
larger-range bytes, changed seed/schema/base and custom-schema graph checks.
Invalid options/configuration, overflow and preserved existing/sibling output
execute at `:613–676`. Actual emitter write and payload-finish flush failures
execute through the production helpers (`generate.rs:269`, `output.rs:142`).

T1-focused-tests-settled.log records **35 library tests, 20 unchanged scale CLI
tests and 4 native CLI tests PASS**, zero failed/ignored. T1-clippy-settled.log
records strict all-target bench clippy PASS. T1-hygiene.log records successful
layer/helper/hash-domain/terminal self-tests and live checks; formatting and
diff checks are clean. All eleven entries of T1-source-receipt.sha256 independently
match current bytes: ten task files plus unchanged Cargo.lock. The actual default
receipt readback is pinned at BLAKE3
`09438d399fc64246fa5f72fa127ca972212c491b97937dd820ec3d55cae22c6e`.
The scale implementation was not modified; lib.rs only exports the new module,
and the new normal lex edge belongs exclusively to the unpublished host bench.

## Evidence limits and remaining plan

The reviewer inspected settled source-bound runtime logs rather than rerunning
builds during the root's reserved timing lane. The finite matrix and endpoint
tests establish their stated behavior, not exhaustive seeds or historical UBA
probability/byte/answer equivalence. Native profile identity explicitly avoids
that claim. No full local gate, hosted qualification or integration is claimed.

Tasks 2–4 still own production make/lane wiring, conversion and receipt tamper
consumption, graph-derived Q1/Q14 answers, custom TBox projection, complete Java
retirement, actual default/nondefault query-lane execution and final gates.
Those are not silently waived or certified by this Task 1 review. No blocking
Task 1 correction was found.
