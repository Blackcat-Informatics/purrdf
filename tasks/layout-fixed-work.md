# Fixed-work layout acceptance

No local timing, latency or throughput supports a performance verdict. The host
is shared and heavily loaded. The modified `ir_layout` benchmark will run only
its `triple_carrier` routines in the existing harness's `--test` mode.

Source inspection: `bench::Mode::Test` calls `time_iterations(routine, 1)`;
`Bencher::iter` therefore invokes each selected closure once. There is no
calibration, warm-up, sampling, saved estimate or baseline comparison in this
mode. The harness internally reads its stopwatch but discards that result and
reports only success/failure. No instruction comparison is claimed.

The actual changed setup builds exactly 1,000 and 10,000 RDF 1.2 records, each
with one triple, one reifier and one annotation. It asserts physical table bytes
12N/8N/12N, initially absent lazy orders, then exercises both triple orders, both
annotation orders and the reifier-target order and asserts their actual ordinal
bytes. Its existing CurrentThreadWindow reports allocation count, requested
bytes and peak live bytes for build and cold-index work. These counters, fixed
input sizes and assertions are the acceptance evidence.

The standard command was `cargo test --locked -j1 -p purrdf-core --bench ir_layout --
--test triple_carrier`, following warning-denied Clippy for that modified target.
Original benchmark setup outside the selected routines is not used for a claim.


Initial80186 failed two new bench semicolon lints before runtime. Both statement
terminators were corrected; retry64584 terminated0 with warning-denied target
Clippy and all four fixed-once routines PASS. No calibration or timing verdict.

| Records (one triple, reifier, annotation each) | Base table bytes | All five lazy index bytes | Build allocations | Build requested bytes | Build peak live bytes | Index allocations |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 1,000 | 32,000 | 20,000 | 1,097 | 456,494 | 206,589 | 5 |
| 10,000 | 320,000 | 200,000 | 10,128 | 5,380,638 | 2,510,669 | 5 |

Each index requested/live total exactly equals its listed physical bytes.
Build totals include lexical storage, validation and temporary construction, so
no claim of strictly linear total build allocation is made. The physical carrier
and ordinal counts are exact. Receipt: triple-layout-fixed-work-retry.log.
