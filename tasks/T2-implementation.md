# Task2 production typed transfers and fresh LOAD

Status: PASS for complete Task2 implementation and focused qualification on
Task1 parent b0fc9e889. Source is frozen; independent tasks/T2-review.md PASS.
Actual final ten portable controls,663 existing affected controls, strict
all-target Clippy, fmt and owning helper gates passed. Normal commit/push are
root-owned and pending. Whole401 is not complete: Task3 full gate, additive
semver and WASM runtime remain unrun. Sole local build lane is FREE.

## Production changes

ADD/COPY/MOVE take the role-qualified owned source snapshot before clearing or
removing anything. Only its destination graph slot changes: source S/P/O blank
identities and ordinary/reifier/annotation kinds stay exact. Insertion/removal
uses the Task1 typed API. Existing self/missing admission, physical attempts,
clear/remove and graph declaration order remain in the production update owner.

LOAD retains resolver, SILENT, host-fetch and post-return stop handling. Each
successful resolution exports all physical records and charges their original
count before publication. A fresh map keyed by source label and BlankScope is
built from S/P/O only, using the existing blank visitor. One checked batch
capacity admission precedes request-counter increments. Bare blanks, iterative
nested triples and CDT values share this map; the existing CDT scanner and blank
codec perform lexical rewrites. A missing map entry is a typed hard error before
publication. Source graph identities are discarded unless referenced by S/P/O.
Unchanged ordinary literals reuse their owned lexical string rather than cloning
it. Requested blank prefixes survive the no-collision-selector case, matching
existing template context behavior; collision-selected namespaces override them.

The newly used Task1 selected-record API originally owned every record before
filtering. This concrete caller cost was corrected in the existing core homes:
records_for_pattern resolves bound IDs, selects ordinary/reifier/annotation ID
streams and filters before the shared import projection owns term values.
Missing bound values return empty without exporting payloads. The full-document
LOAD exporter uses that same projection over all physical IDs. No second term
converter/classifier/scanner, dependency, semantic feature or host code was added.

## Actual executions and retained failures

All raw logs and exits reside at /opt/purrdf-401-qualification/logs/task2 and
losslessly in tasks/T2-logs (62 records, approximately316KiB). Filename-only
credential-pattern scan found no matches; recursive copy comparison passed.
tasks/T2-evidence.sha256 is the exact selected inventory; T2-evidence-copy.txt
records the empty successful recursive comparison. These are immutable executions, not
claims that failed attempts passed after a later repair.

| Actual session | Label | Result |
|---|---|---|
| 42961 | failing-before | 101; all three required-result witnesses failed, with valid single-load/self-copy neighbors preceding assertions. |
| 25371 | initial-fix | 101; typed RdfDiagnostic was incorrectly routed through lexical IRI adapter. Corrected to existing UpdateAbort conversion. |
| 3700 | fix-retry | 0; original three regressions pass in both graph modes. |
| 73442 | expanded | 101; built-in test attributes removed functions from the new harness=false target. Removed those attributes only from portable target. |
| 47089 | expanded-retry | 101; selected core view needed its DatasetView trait in scope. |
| 21679 | controls | 101; broad trait import changed unrelated Arc method resolution. Scoped import inside records_for_pattern. |
| 43791 | controls-retry | 101; three unnecessary GraphMatch qualifications. Corrected without lint allows. |
| 66436 | selected-retry | 101; six new cases pass, three fixture costs used schedule indexes instead of ChargePoint.cost(), and empty-destination LOAD lost requested prefix. |
| 23842 | prefix-retry | 0; all ten portable cases pass, zero ignored/filtered. Both modes are exercised inside each case. |
| 23160 | focused | 101 only at strict; all663 preceding runtime controls pass. Three assigning-clones production lints and empty assertion lint corrected without allows; driver stopped before fmt/hygiene. |
| 91727 | strict-retry | 101; remaining assigning-clones lint in expected-image fixture corrected. |
| 1826 | final-focused | 0; strict-final0, portable-final ten PASS including distinct-source IRIs and isolated inventory additions. |
| 55268 | final-gates | 0; fmt-final0, helpers-final0, source frozen. |

The exact focused commands are retained in focused.sh and *.command. Runtime
counts: eval update::tests43; cdt_query_blank_scope12; governed_update26;
update_graph_existence16; update_graph_modes488 actual portable trials; core
ir::mutable::tests49; blank_publication4; cdt_blank_identity12;
graph_existence_modes11; import_view2. Total663, plus new portable10.
No target had zero executed cases. Final strict command was cargo clippy
--locked -p purrdf-core -p purrdf-sparql-eval --all-targets --jobs 8 -- -D warnings;
portable command was cargo test --locked -p purrdf-sparql-eval --test
update_typed_records --jobs 8. Final fmt was cargo fmt --all --check.
make helpers-hygiene passed81 enforced jobs/23 reasoned variants/91 distinct
rows over1957files, and80 prefix-free hash domains plus existing self-tests.
Layer/terminal/unchanged dependency fence were not redundantly repeated;
normal hooks and Task3 whole gate retain their owning checks.

The final production lint correction uses clone_from, preserving exact graph
values while reusing clone storage. Final portable10 re-executed all transfers
after it. Existing663 successes remain applicable: no admission, charge, stop
or publication algorithm changed. Task1 fullcore1230/shared-view59 evidence
remains applicable outside the selected-projection correction, whose new core
mutable49 and four caller targets passed. This is not Task3 full qualification.

The portable target uses the shipped NativeSparqlEngine, real builder datasets
and a resolver returning the same actual cached Arc. It is registered on the
existing Rust testkit harness for native and subsequent Task3 WASM execution.
Its physical typed images cover all three roles, same-value cross-role overlap,
orphan annotations, default/named transfer directions and exact source identity.
Governed cases assert original physical attempts from the public charge schedule,
including destination dedup, exact ceiling, one-below refusal and original Arc
preservation. LOAD cases cover across-request/same-request identity separation,
same label at two source scopes, eight nested quoted triples, List three embedded
occurrences and Map one (nonvacuous), nested escaped composite identity, opaque
literal bytes, INSERT DATA/template/BNODE shared mint sequence and destination
identity inventory. Internal capacity/map controls exercise MAX, MAX-1, graph-only
discarded blanks, repeated-pair reuse and actionable missing-map refusal.
Different source IRIs returning the same cached Arc additionally yield five
physical rows after the original three. Separate default-prefix cases exercise
unused dictionary c1, suppressed c1 and CDT-only c1 with no bare blank term.

## Resource and applicability boundary

Each admitted build uses the managed nightly SDK executables first in inherited
PATH (kache preserved), CARGO_BUILD_JOBS=8 and explicit --jobs 8,
RUST_TEST_THREADS=8, and private /opt target/build/tmp. The initial focused scope was
purrdf-401-task2-focused-20261008.scope, invocation
a487e89959484b86b98afa43748e088e, observed MemoryMax=68719476736 and
MemorySwapMax=0. Final session55268 scope and contained make/launcher cgroups
are in final-gates-scope.txt/final-gates-children.txt; scope is now inactive/dead.
This capture does not pretend a post-exit scope is a live compiler measurement.
source.sha256 covers exactly five changed/new source homes and final readbacks
all match. rustc.txt/cargo.txt record actual SDK versions; native portable
artifact identity is portable-executable.sha256. No sibling, source index,
branch/ref, hook, global cache or
process management changes were made. No full make check, WASM or semver execution
is substituted by this focused batch. Independent final review and root normal
commit/push follow this source freeze, settled qualification and independent
review. Historical failed attempts remain failures, with all original raw
logs/exits retained; no failed driver was relabeled PASS.
