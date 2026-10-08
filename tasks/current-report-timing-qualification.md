# Matched complete-report cost evidence

Actual session79073 exited0; timing.exit0 and source-readback.exit0.
Source44dbf5ceaba7908aee0f840e4227bf42b040f697,
treece2c73f20afbf6b292db91a7c495a218111586a8. All8 benchmarks measured,
0failed. No source change during execution. Raw command output/compiler/source
hashes/all8 estimates including100 samples each are retained at
/opt/purrdf-402-report-timing-20261008 and copied into
raw/current-report-timing-20261008 for the selected Stage archive.

Command: cargo bench --locked --jobs8 -p purrdf-shapes --bench shared_views
-- shacl_complete_reports (actual argv uses separate --jobs and8 tokens).
Normal default sampling, no quick mode or threshold. Each fixture asserts
exactly128 legacy/complete results before timing; product open/admission is
untimed cold-bind setup. Core and SELECT warm reports reuse their bound views.

|Fixture|Operation|Legacy median µs|Complete median µs|
|---|---|---:|---:|
|Core|Cold bind|1.128|12.332|
|Core|Warm report|101.382|108.006|
|SELECT|Cold bind|1.036|26.292|
|SELECT|Warm report|487.154|663.086|

Each JSON record retains median, MAD,95% seeded-bootstrap bounds,10,000
resamples, exact iterations/sample and all100 samples. The complete warm Core
95% interval is107.898–108.166µs; SELECT660.366–665.094µs. These are measured
costs for these same-host fixtures, not portable speed claims, an ABBA baseline
comparison, or a promise that complete reports have no metadata overhead.
Warm complete Core allocation remains5 at N16 andN32 in the separate actual
owning allocation test; this is not a zero-allocation claim.

Actual local compiler is in compiler.txt. Declared workspace bench inherits
release O3/fat-LTO/codegen1, with debug0 and symbols retained; default assertions
and overflow checks are off. The actual selected benchmark Cargo fingerprint
records warnings-denied and target-cpu=native. Normal Stage compiler containment
is its48GiB/8GiBswap policy despite the outer64GiB/Swap0 scope. No global tool
or cache policy was changed. Other admitted local builds were held during this
batch; shared-host scheduling remains a measurement limitation.

This discharges the missing actual matched report measurement. Whole issue
acceptance still requires current-head hosted/review and final integration.
