# Independent issue analysis

Scope: complete captured issue body and zero comments, accepted portfolio owner
constraints, current production LUBM lane and its shared laws. This is intake
analysis, not implementation or qualification. No Java, JRE, build, test, forge
operation, Git mutation, UBA source acquisition, or memory mutation was performed.

## Requirements and authority

The body requires replacing the Java UBA generator with deterministic Rust,
matching published LUBM statistics/schema, and removing Java/JRE dependencies
from scripts, tests, CI and documentation. The owner's accepted portfolio
clarification defines byte identity as repeatability of this original native
profile for identical seed, university count, starting index and configuration.
It does not authorize copying UBA bodies, its draw order, coefficient arrays,
or claiming native bytes/answers equal historical UBA results. Running Java on
this host is forbidden, including comparison baselines and discovery probes.

Repository AGENTS.md, .baseline and .goals were read at the main root; .baseline
and .goals are absent from this worktree. The deficiency ledger has no emergency
entries. Rust-first, first-party job homes, deterministic bytes, strict failures,
protected main, dependency/layer discipline, SPDX, and normal mandatory hooks
apply. The generated brief cites no ADR. Absence of a cited ADR is not proof
that all repository design documents are irrelevant.

## Observed current implementation

- `scripts/lubm-lane.sh` acquires UBA, inspects Generator.class collections,
  invokes Java, repairs backslash file placement, and converts RDF/XML through
  the production PurRDF CLI with explicit document bases.
- `scripts/benchmark-acquire.py` owns UBA/Linux-fix downloads and their licensing
  text, ontology/query acquisition, cache integrity, and historical corpus and
  Q1/Q14 pins. Those unrelated acquisition/cache protections must survive.
- `scripts/lubm-queries.py` owns the 14 query normalization rules and regime
  index. Their query/ontology artifacts have no redistribution grant recorded;
  they are fetched by digest into ignored output, never vendored.
- `crates/bench` is an unpublished first-party tool home. Existing runtime
  edges are hash/iri/xsd; test-only RDF/parser/testkit support already exists.
- `scripts/lane-common.sh` is the one shared law home. Existing make-driven
  Rust tests deliberately reach early failures without network or Java.
- Current default corpus and Q1=4/Q14=5916 pins are historical UBA receipts.
  They are not native-profile acceptance oracles. Current comments describing
  JVM locale and collection behavior also cease to describe production.

## Executable completeness contract

| Requirement | Required real entry point and acceptance |
|---|---|
| Original Rust generation | An actual Rust generator binary in the existing tooling home creates a complete corpus without invoking or acquiring Java/UBA. Parse emitted data through PurRDF's real RDF reader; exercise the documented command and `make lubm`, rather than only an internal generator helper. |
| Deterministic bytes | Repeat actual CLI generation in distinct output directories/processes with identical profile, seed, university count, index, ontology and document-base configuration. Compare every file name and byte, not only graph equivalence. Test locale independence and stable ordering. Meaningful seed/index/config changes must affect the intended identities without environment/path/time/random-order leakage. |
| University index semantics | Define and test overflow-safe index ranges, entity uniqueness across departments/universities, and reproducibility of a university generated within a range versus independently at the same starting index when the published university-local reproducibility contract is adopted. Do not accidentally treat an index as a fresh unrelated corpus seed. |
| Published schema | Validate actual emitted graph types, property directions, referential targets, class/category distinctions, university/department membership, courses, research groups, staff, students, degrees, publication coauthors and document ontology metadata. Preserve inferred-only Student/Professor/Chair behavior; adding those assertions to obtain query answers would change the workload. |
| Published statistics | Graph-derived tests check every published cardinality/range/ratio and assignment restriction across a fixed finite seed/university matrix, rather than trusting the generator's own counters. Boundary checks cover inclusive ranges, rational rounding, distinct assignments, advisor subsets, degrees, publication counts and feasible course pools. Record exact matrix coverage. |
| Useful queries and independent oracles | Retain all 14 original queries and their documented mechanical transformations. Validate native Q14 against independently counted undergraduate subjects and native Q1 against independently selected actual graph bindings for the normalized query target. Derive checks from the graph or an independently specified structural oracle, not engine-produced counts blessed as expected. Historical UBA row/corpus pins must not be evaluated against native data. |
| Production lane integration | `make lubm` uses the Rust generator, then the real PurRDF conversion/query path. Preserve every supported knob's behavior or make invalid/unhonorable values fail before acquisition or generation. Show a real default run; test multiple universities, nonzero index and seed, custom ontology/document base, absolute and relative output directories, and a supplied prebuilt CLI. |
| Java/JRE retirement | Remove UBA/Linux-fix artifact entries, class inspection, Java process execution, misplaced-file repair, JVM-specific determinism prose, and obsolete prerequisites. Review all affected scripts/tests/Makefile/CI/docs and residual references semantically. Historical licensing or audit context is not a runtime dependency, but obsolete production instructions must be corrected. |
| License preservation | Neither UBA implementation nor coefficient arrays enter source. Preserve external ontology and original query digest/provenance/license posture; no unauthorized fixture copies. Locally authored reduced synthetic query fixtures remain clearly synthetic and cannot substitute for the fetched original workload. |
| Failure integrity | Invalid counts, malformed numbers/IRIs, index overflow, unavailable generator/CLI, output/log creation failure, failed generation/conversion, missing/empty/partial generated files, mismatched manifests, malformed query results and cache corruption fail actionably. No empty-corpus digest, success summary, persisted certificate, or swallowed errors can attest to a failed run. |
| Verification/portability | Focused generator, graph, CLI, lane-law and retirement checks precede normal hook-qualified commits. Use the workflow's single settled full qualification. Host-only tooling must not introduce shipping target/dependency regressions; affected target/layer/generated gates remain real, not asserted from source inspection. |

## Statistical acceptance details

`published-profile-notes.md` records primary specification references verified
by the parent. It supplies these concrete graph checks:

- 15–25 departments per university; staff ranges 7–10 full professors,
  10–14 associate, 8–11 assistant and 5–7 lecturers per department.
- Exactly one full professor heads each department. Each faculty member has
  1–2 undergraduate and 1–2 graduate courses; course ownership is disjoint.
  There are 10–20 department research groups.
- Undergraduate/faculty ratio 8–14; graduate/faculty ratio 3–4. Every student
  belongs to its department. Graduate TA count is between one fifth and one
  quarter of graduates, with distinct assigned courses; RA count between one
  quarter and one third. Tests must use the specification's stated rounding
  convention or an explicit native-profile integer convention where unstated.
- Every graduate and one fifth of undergraduates have professor advisors.
  Undergraduate course load is 2–4; graduate course load is 1–3.
- Publications per full/associate/assistant/lecturer are 15–20/10–18/5–10/0–5;
  graduate coauthors have 0–5. Faculty have three degree-origin relations;
  graduates have one undergraduate degree-origin relation.

Matching these facts is stronger than merely generating approximately 103k
triples. It is weaker than proving an unspecified UBA probability distribution.
Do not invent distributions or use the words "statistical match" to imply
exact UBA frequencies/draw parity. The native profile must state its original
sampling and integer-rounding laws and identify any published facts not yet
resolved. Required unresolved schema/statistical facts block completion; they
are not silently transferred to another issue.

## Benchmark laws that must remain true

The shared locale, checked writes, executable admission, scratch cleanup,
certificate revocation, nonempty data, query-set digest checking before/during/
after execution, result parsing, and verified ignored artifact cache remain
mandatory. A corrupt cached external artifact is an actionable hard failure,
not a silent refetch. A verified cache hit must not require a network fetch.
Any new native reuse record must identify the native profile and full generation
configuration, verify its output, and obey failure revocation; an old UBA corpus
or stamp cannot become a native hit by filename alone.

Preserve the current query regime map: Q1/Q2/Q14 no inference; Q3/Q4 RDFS
subclasses; Q5 RDFS subclasses/subproperties; Q6–Q10 OWL-RL derived Student;
Q11 transitive organization; Q12 realized Chair; Q13 inverse/subproperty alumni.
The full, one-file and slice entailment ladder remains explicitly labeled.
Unsupported or exhausted execution remains CANNOT-EXECUTE, malformed success
output BAD-RESULTS, and a subset result is not a full-corpus answer. A report
with no executed query or universally vacuous results fails rather than looking
fast. Native Q1 may legitimately be zero for a particular draw or target index;
the old claim that every seed/index guarantees its historical nonzero answer
must not become an incorrect hard gate. A real native undergraduate population
and graph-derived Q14 oracle provide useful non-vacuity evidence.

Report corpus profile/configuration/content identity, binary, normalized query
identity, regime, dataset/rung, status, rows and measured time. Native full-rung
results are comparable only against another engine on those exact native bytes,
queries and regime; they are not historical UBA answer receipts. Do not promote
report-only latency to a CI performance assertion or close generator acceptance
from unsupported query timings. WatDiv and scale-corpus shared-law behavior must
not regress while removing LUBM's JVM assumptions.

## Evidence bounds and recurrence

The complete captured issue has zero comments and no linked issue references.
The brief's related search uses three broad title terms and found 14 other
items. It counted 12 `none` and one `performance` trailer occurrences in the
last 200 commits, spanning two labels. Those are bounded trailer counts, not
13 proven recurrences of Java generation defects. This evidence does not count
all historical attempts, all merge history, all unpublished branches or global
generator defects; exact recurrence of this failure is UNKNOWN.

No checks were executed in this analysis. Existing lane-law tests and comments
establish intended acceptance surfaces, not current runtime PASS. The owner
excludes LargeRDFBench/upstream submission work; that exclusion does not remove
any requirement of this native LUBM delivery.

## Assessment

The request is implementable as an original first-party Rust workload, provided
the reviewed plan owns generation, independent graph/statistical validation,
real lane wiring, native identity/oracles, licensing and total Java retirement
as one complete delivery. Two high-risk omissions are retaining historical UBA
pins under the native identity and exercising only early stand-in lane failures
without an actual native default production run. Neither is acceptable evidence
of completion.
