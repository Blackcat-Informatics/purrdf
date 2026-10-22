# Counter and archive replay scope

The measured sources are immutable repository revisions 5db00aaae9 and 13dd0b009. Published c4d1c8a8b retains the substantive source as its parent and changes documentation only. The raw build metadata records commands, compiler 21b707e3f, existing profile/cache environment and actual executable paths. Each instruction record carries exact artifact-bound worker proofs; all original perf, allocation, answer/receipt and shared report data remain in raw/counts.

The stage count-comparison package is process tooling only. Its Cargo.toml uses the existing testkit through ../../../crates/testkit, so replay requires a full repository checkout at the measured source with this stage directory restored at its original relative depth. Recreate any replay checkout only under the canonical main .worktrees directory. Select the archived raw/counts path through PURRDF_BENCH_HOME and run the comparer with settled-main-5db and verified-chunks-13dd. The source program reads shared CountReport/check_compatible/compare and checks all complete guard receipts; it supplies no workload implementation.

Original-chunks-13dd is preserved as inconsistent historical data, not current acceptance. Its cause is not established. Do not overwrite it or relabel it as a pass. The direct-link metadata-stub failure and initial main-root wrapper refusal are preserved too. Source/profile/cache settings were not changed to obtain the verified result.

The comparison package's generated target symlink pointed to a build cache and is removed before capture; its target directory is preserved. Build products are not replay evidence.

The first ghprsq capture stopped before any push because completed perf collectors left 2742 named FIFO endpoints (1371 control.fifo and 1371 ack.fifo). Remote readback confirmed the PR remained open, main remained 68d554993 and no audit refs were published. All collectors had ended and no perf process was live. Only these task-owned pipes under raw/counts were unlinked; a before/after check retained all 5979 regular evidence files with identical size/mtime. Pipes carry no persistent recorded measurement bytes and the collector recreates them on a new run. Their removal resolves an unsupported artifact type, without changing capture policy or removing recorded results.

No raw measured file is removed, compressed, encoded or replaced. All remaining selected-stage files are ordinary source/metadata/log files; none exceeds 20 MiB. The 22 MiB archive is intentionally complete, including negative evidence.
