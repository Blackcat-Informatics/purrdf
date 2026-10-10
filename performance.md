# Native release/O3 comparison

PASS: `native-o3-scaling-2.log`, exit 0. Exact workspace release profile:
opt-level 3, fat LTO, codegen-units 1, no debug assertions; pinned compiler in
`qualification-toolchain.log`. The command uses `cargo test --locked --release
-p purrdf-entail --lib reasoner::schema_tests::schema_preparation_o3_scaling --
--ignored --exact --nocapture --test-threads=1`, with the original
`PURRDF_BENCH_HOME` set to this selected Stage's `benchmark-store`. Private
four-job/16GiB/no-swap lane; other authorized host lanes continued. These sampled
local timings do not establish a universal production speedup.

The existing testkit harness saved all 36 standard `estimates.json` files under
`benchmark-store/schema-preparation/`: cold preparation, cold reasoner,
prepared execution and same-kernel uncached execution at each of nine
restriction/instance points. Every row has ten flat samples, median/MAD and a
95% seeded bootstrap interval (10,000 resamples). The fixture checks all outcomes
have measured estimates and no failure. Native counting-allocator samples are
separate from the timed iteration loops. Retained query bytes are measured before
result destruction and are zero for these healthy results in both paths.

The first run (`native-o3-scaling-1.log`) exited 0 but standard store writes
reported FAILED because the unit target supplied no default store. That run is
NOT MET as benchmark qualification; original failure text remains. The corrected
invocation and strengthened outcome assertion govern this measurement.

Times below are rounded milliseconds; exact samples and confidence intervals are
in the standard estimate files. Allocation/traffic/peak/retained columns give
prepared / uncached values, with byte units for the last three columns. Neither
mode ships a different numerical or reasoning algorithm: the uncached control
recomputes the same preparation for each current membership inside the owning
crate's test build.

| Restriction pairs | Shared instances | Prepared median ms | Uncached median ms | Allocations | Requested traffic bytes | Query peak bytes | Query retained bytes |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 1 | 0.012440 | 0.020246 | 267 / 368 | 38940 / 68404 | 35226 / 36984 | 0 / 0 |
| 1 | 16 | 0.084991 | 0.229633 | 1170 / 2786 | 75600 / 547024 | 48372 / 50480 | 0 / 0 |
| 1 | 64 | 0.378919 | 0.935772 | 3977 / 10441 | 193488 / 2079184 | 92004 / 94112 | 0 / 0 |
| 8 | 1 | 0.083182 | 0.155121 | 1037 / 1436 | 83444 / 416572 | 57318 / 66658 | 0 / 0 |
| 8 | 16 | 0.766148 | 1.456436 | 6370 / 12754 | 306250 / 5636298 | 121762 / 131166 | 0 / 0 |
| 8 | 64 | 2.709212 | 3.902375 | 23311 / 48847 | 1051378 / 22371570 | 359930 / 369334 | 0 / 0 |
| 32 | 1 | 0.185731 | 0.339530 | 3609 / 5098 | 238940 / 4369444 | 134158 / 170954 | 0 / 0 |
| 32 | 16 | 4.915189 | 8.732456 | 24099 / 47923 | 1130674 / 67218738 | 402570 / 439622 | 0 / 0 |
| 32 | 64 | 18.548567 | 45.218392 | 89277 / 184573 | 3998082 / 268350338 | 1271770 / 1308822 | 0 / 0 |

Cold preparation completes three class entries at every point. Its native
work/allocation/retained/admitted-peak counters depend on restriction width and
are identical across the three shared-instance populations:

| Restriction pairs | Work | Allocations | Retained bytes | Admitted peak bytes | Measured allocator peak bytes |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 32 | 15 | 1536 | 2304 | 2264 |
| 8 | 179 | 29 | 6144 | 9696 | 9616 |
| 32 | 1427 | 39 | 24576 | 37776 | 37456 |

All cold samples assert exact retained-layout equality, covered allocator peak,
and zero after destroying preparation. Healthy repeated prepared runs have zero
repeated class-preparation work/allocation; uncached records disclose actual
repeated work. Separate native semantic controls compare both verdict and
canonical contradiction proofs and cover original admitted guard products.
WASM/full-gate/publication acceptance is indexed separately in `validation.md`.
