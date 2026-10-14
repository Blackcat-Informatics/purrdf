# First hosted source failure diagnosis

Read-only diagnosis of PR543 head5170165b8, run38365838491, using retained
ci-workspace-first.log and ci-cross-arch-first.log plus actual source. These are
real failed checks, despite the earlier locally successful full gate. No source
or build change was made by this review.

## Workspace strict failures

The job exits101 on two diagnostics:

- query_scope/counters.rs:22: the existing tests module precedes production
  items. Move that module intact to the file's end. The exact existing
  1e2693242 correction already does this; production counters and test bodies
  remain identical.
- geo-kernel/measure.rs:1218: `point.as_ref().map(&mut *copy)` needlessly
  reborrows a Copy closure. The existing1e2693242 one-line `map(*copy)` repair
  applies to the actual shared-Arithmetic-capturing closure. It preserves the
  same coordinate admission/geometry construction and needs no lint allowance.

Comparison of these two current files against1e2693242's parent is empty, so
reuse can be restricted to those exact previously reviewed file deltas. Do not
copy unrelated documentation changes merely because the commit also has them.
Affected qualification is the original core/geo strict surface; earlier native
compaction behavior evidence remains applicable after an unchanged module move
and the semantically identical closure call.

## Cross-architecture fixture failure

The current i686 distance run passes36 and fails1. At distance/tests.rs:1084,
FTZ actually returns FlushToZero with Register evidence (MXCSR,bits40832); the
fixture wrongly expects Probe evidence. Production XSD environment::register is
cfg-enabled for x86_64 and x86 with SSE2 and calls mxcsr_register in both cases.
The fixture has that same cfg but branches on bare x86 to select the obsolete
probe expectation. Production correctly names the first invalid register.

Reuse the exact c2b54fce8 distance/tests.rs correction: the existing register
expectation serves both supported cfg cases, retaining the original departure
inputs, exact register bits/error kind and restored-register healthy neighbour.
No arithmetic/guard/bypass change is warranted. This is separate from the
already transferred no-SSE2 i586 faulty-token fixture. The same corrected
distance source is present on5392d01ba1b5 and its actual cross-architecture job
115161980821 passes; current542 hosted qualification must still be refreshed
after normal publication. A native x86_64 pass alone would not prove i686.

No evidence here identifies a defect in freeze_with_limits. The hosted checks
remain failed until the corrected current head qualifies; other jobs and any
unreached suffix remain separate. These bounded shared corrections do not
justify a new compaction implementation, altered assertions or tooling changes.
