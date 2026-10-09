# Current-main graph-update baseline witnesses

2026-10-08. Three planned existing-production-path defects are reproduced;
implementation and corrected-result acceptance remain UNRUN. No shipping source,
index, branch or sibling worktree changed. Main remains df2cec88b0f8e05e05ef3d01d14714474be6bbb1,
with only existing Stage work untracked.

The standalone Rust probe in `../raw/graph-baseline-20261008/` uses the actual
current-main core and evaluator via path dependencies, public RdfDatasetBuilder,
GraphResolver and NativeSparqlEngine::update. Fixtures freeze successfully before
their physical table counts are inspected. No parser/evaluator implementation is
copied or mocked; only the admitted resolver returns one cached dataset Arc.

| Production witness | Required result | Actual baseline result |
|---|---|---|
| Two LOAD operations, same document IRI and cached Arc, source blank b0 | Two fresh document blank identities, two rows | One row: document identities alias |
| COPY of one orphan annotation to another graph | Source annotation plus destination annotation | Two rows, only one annotation; destination becomes ordinary |
| COPY of overlapping ordinary+annotation physical rows | Four physical rows, two annotations | Three rows, one annotation; destination collapses the roles |

The source assertions deliberately verify these defective baseline observations.
Probe exit0 establishes reproduction, **not correct update semantics**. The next
implementation must preserve these fixtures as failing-before regressions with
the required-result assertions, then execute corrected production paths. These
three cases do not replace the full typed-storage, classification, suppression,
LOAD blank folding, governor, graph lifetime or host-default contract in
`graph-typed-transfer-implementation-design.md`.

Actual locked-cohort execution handle53766 terminated0. Log:
`../raw/graph-baseline-20261008/execution-main-lock.log`. Private target/build
paths are `/opt/purrdf-graph-baseline-20261008-{target,build}`. Systemd scope
`purrdf-graph-baseline-locked-20261008`, invocation68d605093c3543debfc37aea6ea4abe1,
was launched with MemoryMax64GiB, MemorySwapMax0, jobs8, the managed raw SDK
prepended to inherited PATH, and the existing kache wrapper. This is an
unoptimized development behavioral probe, not shipping-profile qualification,
performance measurement or full make check. Scope was inactive after terminal.

Initial launch failed101 before compilation because its replaced PATH omitted
the existing kache executable; that original log is retained. Corrected PATH
preserved the wrapper. The first successful execution37952 used a newly resolved
dependency cohort and reproduced the same three outputs, but is not the accepted
main-lock baseline. Its initial-resolution.lock is retained. The final run seeded
the standalone Cargo.lock from current main and Cargo pruned unused packages.
The retained Rust verify-lock.rs reported all39 registry packages match main's
versions, sources and checksums; no new runtime dependency was added to PurRDF.

This preparation makes the next coherent401/471 delivery actionable without a
new implementation branch while454 final CI and308 controlled profiling run.
No release, external submission, source commit, PR, cleanup or issue closure.
